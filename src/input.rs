use std::fmt::Display;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread::{self, sleep};

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use evdev_rs::enums::EventCode::{EV_ABS, EV_KEY};
use evdev_rs::enums::EV_ABS::*;
use evdev_rs::enums::EV_KEY::BTN_TOUCH;
use evdev_rs::{enums::EventType, Device, DeviceWrapper, InputEvent, ReadFlag, ReadStatus};
use nix::libc::suseconds_t;
use tracing::debug;

pub fn get_input_devices() -> Result<Vec<InputDevice>> {
    // /dev/input/eventX paths aren't guaranteed to be stable, so use by-path if possible.
    // It doesn't exist on old kernels but the device paths don't seem to change in practise
    let by_path = PathBuf::from("/dev/input/by-path");
    let mut use_by_path = false;
    let device_dir = if by_path.exists() {
        use_by_path = true;
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
            ReadStatus::Sync => todo!(),
        }
        events.push(event);
        last_event_time = Some(Utc::now());
    }
    if events.is_empty() {
        tx.send(ReadEventsMsg::NoEvents).unwrap();
    } else {
        tx.send(ReadEventsMsg::Events((device, events))).unwrap();
    }
}

enum ReadEventsMsg {
    Events((InputDevice, Vec<InputEvent>)),
    NoEvents,
    // Error,
}

pub fn read_input(
    devices: impl Iterator<Item = InputDevice>,
    poll_wait: Duration,
    no_input_timeout: Duration,
    new_event_timeout: Duration,
) -> Vec<(InputDevice, Vec<InputEvent>)> {
    let mut devices_with_events = Vec::new();
    let (tx, rx) = channel();
    let mut len = 0;
    for d in devices {
        len += 1;
        let dtx = tx.clone();
        thread::spawn(move || read_events(d, dtx, poll_wait, no_input_timeout, new_event_timeout));
    }
    for _ in 0..len {
        if let Ok(ReadEventsMsg::Events(events)) = rx.recv() {
            devices_with_events.push(events);
        }
    }
    devices_with_events
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

pub fn optimize_events(events: &mut Vec<InputEvent>, syn_gap: Duration) -> bool {
    if events.is_empty()
        // Skip actions more complex than a single tap or swipe
        || events.iter().filter(|ev| ev.is_code(&EV_KEY(BTN_TOUCH))).count() > 2
        // Skip multi-touch gestures
        || events.iter().any(|ev| ev.is_code(&EV_ABS(ABS_MT_SLOT)))
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
        .position(|ev| ev.is_type(&EventType::EV_SYN))
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
        if ev.is_type(&EventType::EV_SYN) {
            new_time.tv_usec += syn_gap.num_microseconds().unwrap() as suseconds_t;
        }
    }
    true
}

fn drain_last_batch(events: &mut Vec<InputEvent>) -> Vec<InputEvent> {
    if let Some(idx) = &events[..events.len() - 1]
        .iter()
        .rposition(|ev| ev.is_type(&EventType::EV_SYN))
    {
        events.drain(idx + 1..).collect()
    } else {
        std::mem::take(events)
    }
}

fn is_x_coord(ev: &InputEvent) -> bool {
    ev.is_code(&EV_ABS(ABS_X)) || ev.is_code(&EV_ABS(ABS_MT_POSITION_X))
}

fn is_y_coord(ev: &InputEvent) -> bool {
    ev.is_code(&EV_ABS(ABS_Y)) || ev.is_code(&EV_ABS(ABS_MT_POSITION_Y))
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use evdev_rs::enums::EventCode::{EV_ABS, EV_KEY, EV_SYN};
    use evdev_rs::enums::EV_ABS::*;
    use evdev_rs::enums::EV_KEY::BTN_TOUCH;
    use evdev_rs::enums::EV_SYN::SYN_REPORT;
    use evdev_rs::InputEvent;
    use evdev_rs::TimeVal;

    use super::optimize_events;

    #[test]
    fn optimize_events_second_x_missing() {
        let mut events = vec![
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_ABS(ABS_Y),
                value: 51,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769664,
                },
                event_code: EV_ABS(ABS_X),
                value: 522,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769678,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 100,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769693,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 1,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769705,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 798527,
                },
                event_code: EV_ABS(ABS_Y),
                value: 51,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 798576,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 100,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 798598,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807840,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807881,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807892,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
        ];

        let expected = InputEvent {
            time: TimeVal {
                tv_sec: 1705963814,
                tv_usec: 769622,
            },
            event_code: EV_ABS(ABS_X),
            value: 523,
        };
        let syn_gap = Duration::microseconds(1);
        optimize_events(&mut events, syn_gap);
        assert_eq!(expected, events[5]);
    }

    #[test]
    fn optimize_events_second_batch_missing() {
        let mut events = vec![
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_ABS(ABS_Y),
                value: 51,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769664,
                },
                event_code: EV_ABS(ABS_X),
                value: 522,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769678,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 100,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769693,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 1,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769705,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807840,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807881,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 807892,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
        ];

        let expected = vec![
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_ABS(ABS_Y),
                value: 51,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_ABS(ABS_X),
                value: 522,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 100,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 1,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769621,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769622,
                },
                event_code: EV_ABS(ABS_Y),
                value: 52,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769622,
                },
                event_code: EV_ABS(ABS_X),
                value: 523,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769622,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 100,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769622,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769623,
                },
                event_code: EV_ABS(ABS_PRESSURE),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769623,
                },
                event_code: EV_KEY(BTN_TOUCH),
                value: 0,
            },
            InputEvent {
                time: TimeVal {
                    tv_sec: 1705963814,
                    tv_usec: 769623,
                },
                event_code: EV_SYN(SYN_REPORT),
                value: 0,
            },
        ];

        let syn_gap = Duration::microseconds(1);
        optimize_events(&mut events, syn_gap);
        assert_eq!(expected, events);
    }
}
