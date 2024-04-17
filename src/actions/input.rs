use std::fmt::Display;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread::{self, sleep};

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use evdev_rs::enums::EventCode::{EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::enums::EV_KEY::BTN_TOUCH;
use evdev_rs::enums::{EV_ABS::*, EV_SYN::*};
use evdev_rs::{Device, DeviceWrapper, InputEvent, ReadFlag, ReadStatus};
use nix::libc::suseconds_t;
use tracing::{debug, warn};

pub fn get_input_devices(use_by_path: bool) -> Result<Vec<InputDevice>> {
    // /dev/input/eventX paths aren't guaranteed to be stable, but they don't seem to change in
    // practise. /dev/input/by-path doesn't exist on old kernels, and on at least the Aura H20
    // it exists but it doesn't contain the touchscreen. So we'll always use /dev/input/eventX
    // by default, but provide an option to use by-path in case there are ever situations where
    // it works better.
    let by_path = PathBuf::from("/dev/input/by-path");
    let use_by_path = use_by_path && by_path.exists();
    let device_dir = if use_by_path {
        by_path
    } else {
        "/dev/input".into()
    };
    let entries = fs::read_dir(&device_dir)
        .with_context(|| format!("Failed to list {}", &device_dir.display()))?;
    let mut devices = Vec::new();
    for e in entries {
        let path = e
            .with_context(|| format!("Failed to read entry in {}", &device_dir.display()))?
            .path();
        if path.is_dir() || path.file_name().map(|n| n.to_str()).is_none() {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap();
        if !use_by_path && !name.starts_with("event") {
            continue;
        }

        let f = File::open(&path).with_context(|| format!("Failed to open {}", path.display()))?;
        let d = Device::new_from_file(f)
            .with_context(|| format!("Failed to initialize evdev device {}", path.display()))?;
        let device = InputDevice {
            path,
            name: d.name().map(|s| s.to_string()),
            evdev: d,
        };
        devices.push(device);
    }
    Ok(devices)
}

fn read_events(
    device: InputDevice,
    tx: Sender<ReadEventsMsg>,
    poll_wait: Duration,
    no_input_timeout: Duration,
    new_event_timeout: Duration,
) {
    let mut events = Vec::new();
    let mut last_event_time = None;
    let start_time = Utc::now();
    debug!("Reading events from {device}");
    loop {
        if !device.evdev.has_event_pending() {
            sleep(poll_wait.to_std().unwrap());
            if let Some(last_time) = last_event_time {
                let time_since_last = Utc::now() - last_time;
                if time_since_last > new_event_timeout {
                    debug!(
                        "{}ms since last event on {device}. Stopping",
                        new_event_timeout.num_milliseconds(),
                    );
                    break;
                }
            } else if Utc::now() - start_time > no_input_timeout {
                debug!(
                    "No input detected on {device} after {}ms. Stopping",
                    no_input_timeout.num_milliseconds(),
                );
                break;
            }
            continue;
        }
        let (status, event) = device
            .evdev
            .next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)
            .unwrap();
        match status {
            ReadStatus::Success => (),
            ReadStatus::Sync => {
                // Not sure if it's worth trying to handle this if it occurs. Never happened in
                // testing and unsure of the best approach, so for now just return an error.
                // https://docs.rs/evdev-rs/0.6.1/evdev_rs/struct.Device.html#method.next_event
                if tx.send(ReadEventsMsg::Error).is_err() {
                    warn!("Failed to send ReadEventsMsg::Error to receiver");
                    return;
                }
            }
        }
        events.push(event);
        last_event_time = Some(Utc::now());
    }
    if events.is_empty() {
        if tx.send(ReadEventsMsg::NoEvents).is_err() {
            warn!("Failed to send ReadEventsMsg::NoEvents to receiver");
        }
    } else if tx.send(ReadEventsMsg::Events((device, events))).is_err() {
        warn!("Failed to send ReadEventsMsg::Events to receiver");
    }
}

enum ReadEventsMsg {
    Events((InputDevice, Vec<InputEvent>)),
    NoEvents,
    Error,
}

pub fn read_input(
    devices: impl Iterator<Item = InputDevice>,
    poll_wait: Duration,
    no_input_timeout: Duration,
    new_event_timeout: Duration,
) -> Result<Vec<(InputDevice, Vec<InputEvent>)>> {
    let mut devices_with_events = Vec::new();
    let (tx, rx) = channel();
    let mut len = 0;
    for d in devices {
        len += 1;
        let dtx = tx.clone();
        thread::spawn(move || read_events(d, dtx, poll_wait, no_input_timeout, new_event_timeout));
    }
    let err = anyhow!("An error occured while reading input. Please try again.");
    for _ in 0..len {
        match rx.recv() {
            Ok(ReadEventsMsg::Events(events)) => devices_with_events.push(events),
            Ok(ReadEventsMsg::NoEvents) => continue,
            Ok(ReadEventsMsg::Error) => return Err(err),
            Err(_) => return Err(err),
        }
    }
    Ok(devices_with_events)
}

#[derive(Debug)]
pub struct InputDevice {
    pub path: PathBuf,
    pub name: Option<String>,
    pub evdev: Device,
}

impl Display for InputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())?;
        if let Some(name) = &self.name {
            write!(f, " ({})", name)?;
        }
        Ok(())
    }
}

pub fn is_touch_device(d: &InputDevice) -> bool {
    debug!("Checking if {d} is a touch device");
    let has_btn_touch = d.evdev.has(EV_KEY(BTN_TOUCH));
    debug!("has_btn_touch: {has_btn_touch}");
    let has_mt_pos_x = d.evdev.has(EV_ABS(ABS_MT_POSITION_X));
    debug!("has_mt_pos_x: {has_mt_pos_x}");
    has_btn_touch || has_mt_pos_x
}

pub fn optimize_events(events: &mut Vec<InputEvent>, syn_gap: Duration) -> bool {
    if events.is_empty()
        // Skip actions more complex than a single tap or swipe
        || events.iter().filter(|ev| ev.is_code(&EV_KEY(BTN_TOUCH))).count() > 2
        // Skip multi-touch gestures
        || events.iter().filter(|ev| ev.is_code(&EV_ABS(ABS_MT_SLOT))).count() > 1
    {
        debug!("Skipped optimizing events");
        return false;
    }

    debug!("Optimizing events");
    // Taps and swipes seem to register fine with only a start and end X/Y coordinate,
    // so only keep the first, last, and second-to-last batches of events. On some devices the last
    // batch will only signal the release of a touch, with end coordinates in the penultimate batch
    let idx = events
        .iter()
        .position(|ev| ev.is_code(&EV_SYN(SYN_REPORT)))
        .unwrap();
    let first: Vec<InputEvent> = events.drain(..idx + 1).collect();
    let mut last = drain_last_batch(events);

    // On some old Kobos a quick tap won't produce a second batch of events, which prevents
    // action playback from working properly
    let mut penultimate = if events.is_empty() {
        first
            .iter()
            .filter(|ev| !ev.is_code(&EV_KEY(BTN_TOUCH)))
            .cloned()
            .collect()
    } else {
        drain_last_batch(events)
    };

    // They also might only produce one X or Y coordinate, which again causes issues
    if !penultimate.iter().any(is_x_coord) {
        let mut iter = events.iter().rev().chain(first.iter().rev());
        if let Some(last_x) = iter.find(|ev| is_x_coord(ev)) {
            penultimate.insert(0, last_x.clone());
        }
    }
    if !penultimate.iter().any(is_y_coord) {
        let mut iter = events.iter().rev().chain(first.iter().rev());
        if let Some(last_y) = iter.find(|ev| is_y_coord(ev)) {
            penultimate.insert(0, last_y.clone());
        }
    }

    // They also don't like the last X or Y coordinate being identical to the first
    if let Some(first_x) = first.iter().find(|ev| is_x_coord(ev)) {
        let mut iter = last.iter_mut().rev().chain(penultimate.iter_mut().rev());
        if let Some(last_x) = iter.find(|ev| is_x_coord(ev)) {
            if first_x.value == last_x.value {
                last_x.value += 1;
            }
        }
    }
    if let Some(first_y) = first.iter().find(|ev| is_y_coord(ev)) {
        let mut iter = last.iter_mut().rev().chain(penultimate.iter_mut().rev());
        if let Some(last_y) = iter.find(|ev| is_y_coord(ev)) {
            if first_y.value == last_y.value {
                last_y.value += 1;
            }
        }
    }

    events.clear();
    events.extend(first.into_iter().chain(penultimate.into_iter().chain(last)));

    // Reduce the gap between each batch of events to the smallest possible amount
    let mut new_time = events.first().unwrap().time;
    for ev in events {
        ev.time = new_time;
        if ev.is_code(&EV_SYN(SYN_REPORT)) {
            new_time.tv_usec += syn_gap.num_microseconds().unwrap() as suseconds_t;
        }
    }
    true
}

fn drain_last_batch(events: &mut Vec<InputEvent>) -> Vec<InputEvent> {
    if let Some(idx) = &events[..events.len() - 1]
        .iter()
        .rposition(|ev| ev.is_code(&EV_SYN(SYN_REPORT)))
    {
        events.drain(idx + 1..).collect()
    } else {
        std::mem::take(events)
    }
}

pub fn is_x_coord(ev: &InputEvent) -> bool {
    ev.is_code(&EV_ABS(ABS_X)) || ev.is_code(&EV_ABS(ABS_MT_POSITION_X))
}

pub fn is_y_coord(ev: &InputEvent) -> bool {
    ev.is_code(&EV_ABS(ABS_Y)) || ev.is_code(&EV_ABS(ABS_MT_POSITION_Y))
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use evdev_rs::enums::EventCode::{self, EV_ABS, EV_KEY, EV_SYN};
    use evdev_rs::enums::EV_ABS::*;
    use evdev_rs::enums::EV_KEY::BTN_TOUCH;
    use evdev_rs::enums::EV_SYN::{SYN_MT_REPORT, SYN_REPORT};
    use evdev_rs::InputEvent;
    use evdev_rs::TimeVal;

    use super::optimize_events;
    use pretty_assertions::assert_eq;

    #[test]
    fn optimize_events_second_x_missing() {
        let mut events = vec![
            input_event(1705963814, 769621, EV_ABS(ABS_Y), 51),
            input_event(1705963814, 769664, EV_ABS(ABS_X), 522),
            input_event(1705963814, 769678, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769693, EV_KEY(BTN_TOUCH), 1),
            input_event(1705963814, 769705, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 798527, EV_ABS(ABS_Y), 51),
            input_event(1705963814, 798576, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 798598, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 807840, EV_ABS(ABS_PRESSURE), 0),
            input_event(1705963814, 807881, EV_KEY(BTN_TOUCH), 0),
            input_event(1705963814, 807892, EV_SYN(SYN_REPORT), 0),
        ];
        let expected = vec![
            input_event(1705963814, 769621, EV_ABS(ABS_Y), 51),
            input_event(1705963814, 769621, EV_ABS(ABS_X), 522),
            input_event(1705963814, 769621, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769621, EV_KEY(BTN_TOUCH), 1),
            input_event(1705963814, 769621, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 769622, EV_ABS(ABS_X), 523),
            input_event(1705963814, 769622, EV_ABS(ABS_Y), 52),
            input_event(1705963814, 769622, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769622, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 769623, EV_ABS(ABS_PRESSURE), 0),
            input_event(1705963814, 769623, EV_KEY(BTN_TOUCH), 0),
            input_event(1705963814, 769623, EV_SYN(SYN_REPORT), 0),
        ];

        let syn_gap = Duration::microseconds(1);
        optimize_events(&mut events, syn_gap);
        assert_eq!(expected, events);
    }

    #[test]
    fn optimize_events_second_batch_missing() {
        let mut events = vec![
            input_event(1705963814, 769621, EV_ABS(ABS_Y), 51),
            input_event(1705963814, 769664, EV_ABS(ABS_X), 522),
            input_event(1705963814, 769678, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769693, EV_KEY(BTN_TOUCH), 1),
            input_event(1705963814, 769705, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 807840, EV_ABS(ABS_PRESSURE), 0),
            input_event(1705963814, 807881, EV_KEY(BTN_TOUCH), 0),
            input_event(1705963814, 807892, EV_SYN(SYN_REPORT), 0),
        ];

        let expected = vec![
            input_event(1705963814, 769621, EV_ABS(ABS_Y), 51),
            input_event(1705963814, 769621, EV_ABS(ABS_X), 522),
            input_event(1705963814, 769621, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769621, EV_KEY(BTN_TOUCH), 1),
            input_event(1705963814, 769621, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 769622, EV_ABS(ABS_Y), 52),
            input_event(1705963814, 769622, EV_ABS(ABS_X), 523),
            input_event(1705963814, 769622, EV_ABS(ABS_PRESSURE), 100),
            input_event(1705963814, 769622, EV_SYN(SYN_REPORT), 0),
            input_event(1705963814, 769623, EV_ABS(ABS_PRESSURE), 0),
            input_event(1705963814, 769623, EV_KEY(BTN_TOUCH), 0),
            input_event(1705963814, 769623, EV_SYN(SYN_REPORT), 0),
        ];

        let syn_gap = Duration::microseconds(1);
        optimize_events(&mut events, syn_gap);
        assert_eq!(expected, events);
    }

    fn input_event(tv_sec: i64, tv_usec: i64, event_code: EventCode, value: i32) -> InputEvent {
        let time = TimeVal { tv_sec, tv_usec };
        InputEvent {
            time,
            event_code,
            value,
        }
    }

    #[test]
    // test that optimizing works when there are two types of SYN reports as on the Aura H20v1
    fn optimize_events_multiple_syn_reports() {
        let mut events = vec![
            input_event(0, 1, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 731, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 769, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 788, EV_ABS(ABS_MT_POSITION_X), 790),
            input_event(0, 800, EV_ABS(ABS_MT_POSITION_Y), 1012),
            input_event(0, 814, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 824, EV_SYN(SYN_REPORT), 0),
            input_event(0, 42261, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 43031, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 43061, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 43079, EV_ABS(ABS_MT_POSITION_X), 791),
            input_event(0, 43091, EV_ABS(ABS_MT_POSITION_Y), 1012),
            input_event(0, 43105, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 43117, EV_SYN(SYN_REPORT), 0),
            input_event(0, 52835, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 52878, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 52887, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 52898, EV_ABS(ABS_MT_POSITION_X), 793),
            input_event(0, 52910, EV_ABS(ABS_MT_POSITION_Y), 1011),
            input_event(0, 52922, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 52932, EV_SYN(SYN_REPORT), 0),
            input_event(0, 63256, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 63294, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 63302, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 63314, EV_ABS(ABS_MT_POSITION_X), 796),
            input_event(0, 63325, EV_ABS(ABS_MT_POSITION_Y), 1009),
            input_event(0, 63338, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 63347, EV_SYN(SYN_REPORT), 0),
            input_event(0, 73467, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 73941, EV_ABS(ABS_MT_TOUCH_MAJOR), 0),
            input_event(0, 74155, EV_ABS(ABS_MT_WIDTH_MAJOR), 0),
            input_event(0, 74191, EV_ABS(ABS_MT_POSITION_X), 796),
            input_event(0, 74205, EV_ABS(ABS_MT_POSITION_Y), 1009),
            input_event(0, 74219, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 74231, EV_SYN(SYN_REPORT), 0),
        ];
        let expected = vec![
            input_event(0, 1, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 1, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 1, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 1, EV_ABS(ABS_MT_POSITION_X), 790),
            input_event(0, 1, EV_ABS(ABS_MT_POSITION_Y), 1012),
            input_event(0, 1, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 1, EV_SYN(SYN_REPORT), 0),
            input_event(0, 2, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 2, EV_ABS(ABS_MT_TOUCH_MAJOR), 1),
            input_event(0, 2, EV_ABS(ABS_MT_WIDTH_MAJOR), 1),
            input_event(0, 2, EV_ABS(ABS_MT_POSITION_X), 796),
            input_event(0, 2, EV_ABS(ABS_MT_POSITION_Y), 1009),
            input_event(0, 2, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 2, EV_SYN(SYN_REPORT), 0),
            input_event(0, 3, EV_ABS(ABS_MT_TRACKING_ID), 1),
            input_event(0, 3, EV_ABS(ABS_MT_TOUCH_MAJOR), 0),
            input_event(0, 3, EV_ABS(ABS_MT_WIDTH_MAJOR), 0),
            input_event(0, 3, EV_ABS(ABS_MT_POSITION_X), 796),
            input_event(0, 3, EV_ABS(ABS_MT_POSITION_Y), 1009),
            input_event(0, 3, EV_SYN(SYN_MT_REPORT), 0),
            input_event(0, 3, EV_SYN(SYN_REPORT), 0),
        ];
        let syn_gap = Duration::microseconds(1);
        optimize_events(&mut events, syn_gap);
        assert_eq!(expected, events);
    }
}
