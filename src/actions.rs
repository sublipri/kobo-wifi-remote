use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use evdev_rs::enums::EventCode::EV_KEY;
use evdev_rs::enums::EV_KEY::BTN_TOUCH;
use evdev_rs::util::event_code_to_int;
use evdev_rs::{DeviceWrapper, InputEvent, TimeVal};
use framebuffer::Framebuffer;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMicroSeconds, DurationMilliSeconds};
use slug::slugify;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

use crate::input::{get_input_devices, optimize_events, read_input};

pub struct ActionManager {
    pub path: PathBuf,
    pub actions: BTreeMap<String, Action>,
    pub rx: mpsc::Receiver<ActionMsg>,
    pub play_wait_until: DateTime<Utc>,
}

impl ActionManager {
    pub fn from_path(path: PathBuf, rx: mpsc::Receiver<ActionMsg>) -> Result<Self> {
        let actions = if path.exists() {
            debug!("Loading actions from {}", path.display());
            let bytes = fs::read(&path)
                .with_context(|| format!("Failed to read actions from {}", &path.display()))?;
            bincode::deserialize(&bytes).with_context(|| {
                format!("Failed to deserialize actions from {}", &path.display())
            })?
        } else {
            debug!("No action file at {}", path.display());
            BTreeMap::new()
        };
        Ok(Self {
            path,
            actions,
            rx,
            play_wait_until: Utc::now(),
        })
    }

    fn record(&mut self, opts: RecordActionOptions) -> Result<()> {
        let uri = opts.uri.clone().unwrap_or(slugify(&opts.name));
        if let Some(ref mut action) = self.actions.get_mut(&uri) {
            action.record(&opts)?;
            return Ok(());
        }

        let mut action = Action::new(&opts)?;
        action.record(&opts)?;
        self.actions.insert(uri, action);
        let bytes = bincode::serialize(&self.actions).context("Failed to serialize actions")?;
        debug!("Writing actions to {}", &self.path.display());
        fs::write(&self.path, bytes)
            .with_context(|| format!("Failed to write actions to {}", self.path.display()))?;
        Ok(())
    }

    fn play(&mut self, uri: &str) -> Result<()> {
        // Don't play consecutive actions immediately so the device has time to act on the input.
        // Allows a user to spam page turns and have them all register
        if Utc::now() < self.play_wait_until {
            sleep(self.play_wait_until - Utc::now());
        }
        let Some(action) = self.actions.get(uri) else {
            return Err(anyhow!("No action exists for {uri}"));
        };
        action.play()?;
        self.play_wait_until = Utc::now() + action.post_playback_delay;
        Ok(())
    }

    pub fn manage(&mut self) {
        loop {
            match self.rx.blocking_recv() {
                Some(ActionMsg::Record { opts, resp }) => {
                    let result = self.record(opts);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Record result. Receiver dropped")
                    }
                }
                Some(ActionMsg::Play { uri, resp }) => {
                    let result = self.play(&uri);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Play result. Receiver dropped")
                    }
                }
                Some(ActionMsg::List { resp }) => {
                    let actions = self.actions.values().cloned().collect();
                    if resp.send(actions).is_err() {
                        warn!("Unable to send actions list. Receiver dropped")
                    }
                }
                None => break,
            }
        }
    }
}

pub enum ActionMsg {
    Record {
        opts: RecordActionOptions,
        resp: oneshot::Sender<Result<()>>,
    },
    Play {
        uri: String,
        resp: oneshot::Sender<Result<()>>,
    },
    List {
        resp: oneshot::Sender<Vec<Action>>,
    },
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub uri: String,
    pub sort_value: String,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    pub recordings: [Option<ActionRecording>; 4],
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionRecording {
    pub rotation: usize,
    pub events: Vec<ActionEvent>,
    pub dev_path: PathBuf,
}

fn current_rotation() -> usize {
    let Ok(framebuffer) = Framebuffer::new("/dev/fb0") else {
        return 0;
    };
    // Always returns 0 on some Kobo models. TODO: use FBInk instead
    framebuffer.var_screen_info.rotate as usize
}

impl Action {
    pub fn new(opts: &RecordActionOptions) -> Result<Self> {
        Ok(Self {
            sort_value: opts.sort_value.clone().unwrap_or(opts.name.clone()),
            uri: opts.uri.clone().unwrap_or(slugify(&opts.name)),
            name: opts.name.clone(),
            keyboard_shortcut: opts.keyboard_shortcut,
            post_playback_delay: opts.post_playback_delay,
            recordings: [None, None, None, None],
        })
    }
    pub fn record(&mut self, opts: &RecordActionOptions) -> Result<()> {
        let devices = get_input_devices()?;
        let rotation = current_rotation();

        let devices_with_events = if opts.only_check_touch {
            read_input(
                devices
                    .into_iter()
                    .filter(|d| d.evdev.has(EV_KEY(BTN_TOUCH))),
                opts.poll_wait,
                opts.no_input_timeout,
                opts.new_event_timeout,
            )
        } else {
            read_input(
                devices.into_iter(),
                opts.poll_wait,
                opts.no_input_timeout,
                opts.new_event_timeout,
            )
        };

        if devices_with_events.is_empty() {
            return Err(anyhow!("No input detected"));
        }
        for (d, _) in &devices_with_events {
            debug!("Input detected on {d}");
        }

        let (device, mut events) = if devices_with_events.len() > 1 {
            todo!()
        } else {
            devices_with_events.into_iter().next().unwrap()
        };

        if opts.optimize {
            optimize_events(&mut events, opts.syn_gap);
        }
        let recording = ActionRecording {
            dev_path: device.path,
            events: create_action_events(&events),
            rotation,
        };
        self.recordings[rotation] = Some(recording);
        Ok(())
    }

    pub fn play(&self) -> Result<()> {
        let rotation = current_rotation();
        let Some(ref recording) = self.recordings[rotation] else {
            return Err(anyhow!(
                "No recording for {} in rotation {}",
                self.name,
                rotation
            ));
        };

        let mut f = File::options()
            .read(true)
            .write(true)
            .open(&recording.dev_path)
            .unwrap();

        debug!(
            "Writing events for {} to {}",
            &self.uri,
            &recording.dev_path.display()
        );
        for ev in &recording.events {
            f.write_all(&ev.buf).unwrap();
            if let Some(dur) = ev.sleep_duration {
                sleep(dur);
            }
        }
        debug!("Finished writing events for {}", &self.uri);

        Ok(())
    }
}

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordActionOptions {
    pub name: String,
    pub sort_value: Option<String>,
    pub uri: Option<String>,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    pub only_check_touch: bool,
    pub optimize: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
    #[serde_as(as = "DurationMicroSeconds<i64>")]
    pub syn_gap: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub no_input_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub new_event_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub poll_wait: Duration,
}

impl Default for RecordActionOptions {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            sort_value: None,
            uri: None,
            keyboard_shortcut: None,
            only_check_touch: true,
            optimize: true,
            post_playback_delay: Duration::milliseconds(300),
            syn_gap: Duration::microseconds(1),
            no_input_timeout: Duration::milliseconds(5000),
            new_event_timeout: Duration::milliseconds(500),
            poll_wait: Duration::milliseconds(10),
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// InputEvent that's been processed ready to use as part of an action
pub struct ActionEvent {
    /// native endian bytes ready to write to the input device
    pub buf: Vec<u8>,
    /// How long to sleep for after writing the input event
    #[serde_as(as = "Option<DurationMicroSeconds<i64>>")]
    pub sleep_duration: Option<Duration>,
}

fn create_action_events(events: &[InputEvent]) -> Vec<ActionEvent> {
    let mut action_events = Vec::new();
    if events.is_empty() {
        return action_events;
    }
    let mut iter = events.iter().peekable();
    let start_time = parse_timeval(events.first().unwrap().time) - Duration::microseconds(1);
    let error_margin = Duration::microseconds(150); // copied from evemu
    while let Some(ev) = iter.next() {
        let (ev_type, ev_code) = event_code_to_int(&ev.event_code);
        let ev_time = parse_timeval(ev.time);
        let time_since_start = ev_time - start_time;
        let seconds = time_since_start.num_seconds() as usize;
        let microseconds = time_since_start.num_microseconds().unwrap() as usize % 1_000_000;
        let mut ae = ActionEvent::default();
        ae.buf.extend_from_slice(&seconds.to_ne_bytes());
        ae.buf.extend_from_slice(&microseconds.to_ne_bytes());
        ae.buf.extend_from_slice(&(ev_type as u16).to_ne_bytes());
        ae.buf.extend_from_slice(&(ev_code as u16).to_ne_bytes());
        ae.buf.extend_from_slice(&ev.value.to_ne_bytes());
        if let Some(next) = iter.peek() {
            let next_time = parse_timeval(next.time);
            let gap = next_time - ev_time;
            if gap > error_margin * 2 {
                ae.sleep_duration = Some(gap - error_margin);
            }
        }
        action_events.push(ae);
    }
    action_events
}

#[allow(clippy::useless_conversion)]
fn parse_timeval(tv: TimeVal) -> DateTime<Utc> {
    let nsec = tv.tv_usec as u32 * 1000;
    // tv_sec will be i32 on 32bit platforms
    DateTime::from_timestamp(tv.tv_sec.into(), nsec).unwrap()
}

fn sleep(duration: Duration) {
    if duration > Duration::zero() {
        debug!("Sleeping for {}ms", duration.num_milliseconds());
        std::thread::sleep(duration.to_std().unwrap());
    }
}
