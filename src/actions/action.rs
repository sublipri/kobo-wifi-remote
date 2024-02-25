use super::input::{get_input_devices, optimize_events, read_input};

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
use fbink_rs::{CanonicalRotation, FbInk};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMicroSeconds, DurationMilliSeconds};
use slug::slugify;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

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

    fn record(&mut self, opts: RecordActionOptions) -> Result<RecordActionResponse> {
        let path_segment = opts.path_segment.clone().unwrap_or(slugify(&opts.name));
        let response = if let Some(ref mut action) = self.actions.get_mut(&path_segment) {
            action.record(&opts)?
        } else {
            let mut action = Action::new(&opts)?;
            let response = action.record(&opts)?;
            self.actions.insert(path_segment, action);
            response
        };
        self.write()?;
        Ok(response)
    }

    fn write(&self) -> Result<()> {
        let bytes = bincode::serialize(&self.actions).context("Failed to serialize actions")?;
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("bin.bkp"))
                .context("Failed to backup actions file")?;
        }
        let tmp = self.path.with_extension("tmp");
        debug!("Writing actions to {}", tmp.display());
        fs::write(&tmp, bytes)
            .with_context(|| format!("Failed to write actions to {}", tmp.display()))?;
        fs::rename(&tmp, &self.path).context("Failed to rename temporary actions file")?;
        Ok(())
    }

    fn play(&mut self, path_segment: &str) -> Result<()> {
        // Don't play consecutive actions immediately so the device has time to act on the input.
        // Allows a user to spam page turns and have them all register
        if Utc::now() < self.play_wait_until {
            sleep(self.play_wait_until - Utc::now());
        }
        let Some(action) = self.actions.get(path_segment) else {
            return Err(anyhow!("No action exists for {path_segment}"));
        };
        action.play()?;
        self.play_wait_until = Utc::now() + action.post_playback_delay;
        Ok(())
    }

    fn delete(&mut self, path_segment: &str) -> Result<()> {
        if self.actions.remove(path_segment).is_some() {
            self.write()?;
            Ok(())
        } else {
            Err(anyhow!("No action exists for {path_segment}"))
        }
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
                Some(ActionMsg::Play { path_segment, resp }) => {
                    let result = self.play(&path_segment);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Play result. Receiver dropped")
                    }
                }
                Some(ActionMsg::List { resp }) => {
                    let mut actions: Vec<Action> = self.actions.values().cloned().collect();
                    actions.sort_by(|a, b| a.sort_value.partial_cmp(&b.sort_value).unwrap());
                    if resp.send(actions).is_err() {
                        warn!("Unable to send actions list. Receiver dropped")
                    }
                }
                Some(ActionMsg::Delete { path_segment, resp }) => {
                    let result = self.delete(&path_segment);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Delete result. Receiver dropped")
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
        resp: oneshot::Sender<Result<RecordActionResponse>>,
    },
    Play {
        path_segment: String,
        resp: oneshot::Sender<Result<()>>,
    },
    List {
        resp: oneshot::Sender<Vec<Action>>,
    },
    Delete {
        path_segment: String,
        resp: oneshot::Sender<Result<()>>,
    },
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub path_segment: String,
    pub sort_value: String,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    pub recordings: [Option<ActionRecording>; 4],
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionRecording {
    pub rotation: CanonicalRotation,
    pub events: Vec<ActionEvent>,
    pub dev_path: PathBuf,
    pub is_optimized: bool,
}

fn current_rotation() -> Result<CanonicalRotation> {
    let fbink = FbInk::with_defaults()?;
    Ok(fbink.state().canonical_rotation())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordActionResponse {
    pub name: String,
    pub rotation: String,
    pub was_optimized: bool,
    pub device: String,
}

impl Action {
    pub fn new(opts: &RecordActionOptions) -> Result<Self> {
        Ok(Self {
            sort_value: opts.sort_value.clone().unwrap_or(opts.name.clone()),
            path_segment: opts.path_segment.clone().unwrap_or(slugify(&opts.name)),
            name: opts.name.clone(),
            keyboard_shortcut: opts.keyboard_shortcut,
            post_playback_delay: opts.post_playback_delay,
            recordings: [None, None, None, None],
        })
    }

    pub fn shortcut_name(&self) -> String {
        self.keyboard_shortcut
            .map_or("None".to_string(), |s| s.to_string())
    }

    pub fn record(&mut self, opts: &RecordActionOptions) -> Result<RecordActionResponse> {
        let devices = get_input_devices()?;
        let rotation = current_rotation()?;

        let devices_with_events = if opts.only_check_touch {
            read_input(
                devices
                    .into_iter()
                    .filter(|d| d.evdev.has(EV_KEY(BTN_TOUCH))),
                opts.poll_wait,
                opts.no_input_timeout,
                opts.new_event_timeout,
            )?
        } else {
            read_input(
                devices.into_iter(),
                opts.poll_wait,
                opts.no_input_timeout,
                opts.new_event_timeout,
            )?
        };

        if devices_with_events.is_empty() {
            return Err(anyhow!("No input detected"));
        }
        for (d, e) in &devices_with_events {
            debug!("Input detected on {d} ({} events)", e.len());
        }

        let (device, mut events) = if devices_with_events.len() > 1 {
            // TODO: It's unlikely that there will ever be multiple devices with events detected,
            // but the ideal way to handle this would be to enable the user to select which device
            // they intended to record
            devices_with_events
                .into_iter()
                .max_by_key(|(_d, e)| e.len())
                .unwrap()
        } else {
            devices_with_events.into_iter().next().unwrap()
        };

        let is_optimized = if opts.optimize {
            optimize_events(&mut events, opts.syn_gap)
        } else {
            false
        };

        let response = RecordActionResponse {
            name: self.name.clone(),
            rotation: rotation.to_string(),
            was_optimized: is_optimized,
            device: format!("{}", &device),
        };

        let recording = ActionRecording {
            dev_path: device.path,
            events: create_action_events(&events),
            rotation,
            is_optimized,
        };
        self.recordings[rotation as usize] = Some(recording);

        Ok(response)
    }

    pub fn play(&self) -> Result<()> {
        let rotation = current_rotation()?;
        let Some(ref recording) = self.recordings[rotation as usize] else {
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
            &self.path_segment,
            &recording.dev_path.display()
        );
        for ev in &recording.events {
            f.write_all(&ev.buf).unwrap();
            if let Some(dur) = ev.sleep_duration {
                sleep(dur);
            }
        }
        debug!("Finished writing events for {}", &self.path_segment);

        Ok(())
    }
}

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordActionOptions {
    pub name: String,
    pub sort_value: Option<String>,
    pub path_segment: Option<String>,
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
            path_segment: None,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
/// InputEvent that's been processed ready to use as part of an action
pub struct ActionEvent {
    /// native endian bytes ready to write to the input device
    pub buf: Vec<u8>,
    /// How long to sleep for after writing the input event
    #[serde_as(as = "Option<DurationMicroSeconds<i64>>")]
    pub sleep_duration: Option<Duration>,
}

impl Default for ActionEvent {
    fn default() -> Self {
        Self {
            buf: Vec::with_capacity(16),
            sleep_duration: None,
        }
    }
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
        ae.buf.extend(seconds.to_ne_bytes());
        ae.buf.extend(microseconds.to_ne_bytes());
        ae.buf.extend((ev_type as u16).to_ne_bytes());
        ae.buf.extend((ev_code as u16).to_ne_bytes());
        ae.buf.extend(ev.value.to_ne_bytes());
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
