use super::input::{get_input_devices, is_touch_device, optimize_events, read_input};
use crate::util::sleep;

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use evdev_rs::util::event_code_to_int;
use evdev_rs::{InputEvent, TimeVal};
use fbink_rs::{CanonicalRotation, FbInk};
use nix::libc;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMicroSeconds, DurationMilliSeconds};
use slug::slugify;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

pub struct ActionManager {
    pub actions: ActionsFile,
    fbink: Arc<FbInk>,
    pub recordings: RecordingsFile,
    rx: mpsc::Receiver<ActionMsg>,
    play_wait_until: DateTime<Utc>,
}

impl ActionManager {
    pub fn from_path(
        actions_path: PathBuf,
        recordings_path: PathBuf,
        fbink: Arc<FbInk>,
        rx: mpsc::Receiver<ActionMsg>,
    ) -> Result<Self> {
        Ok(Self {
            actions: ActionsFile::load(actions_path)?,
            recordings: RecordingsFile::load(recordings_path)?,
            fbink,
            rx,
            play_wait_until: Utc::now(),
        })
    }

    fn record(&mut self, opts: RecordActionOptions) -> Result<RecordActionResponse> {
        let path_segment = opts.path_segment.clone().unwrap_or(slugify(&opts.name));

        if !self.actions.data.contains_key(&path_segment) {
            self.actions.data.insert(
                path_segment.clone(),
                ActionOptions {
                    sort_value: opts.sort_value.clone().unwrap_or(opts.name.clone()),
                    name: opts.name.clone(),
                    keyboard_shortcut: opts.keyboard_shortcut,
                    voice_trigger: opts.voice_trigger.clone(),
                    post_playback_delay: opts.post_playback_delay,
                },
            );
            self.actions.write()?;
        }

        let action = self.actions.data.get(&path_segment).unwrap();
        let rotation = self.fbink.current_rotation()?;
        let recording = ActionRecording::record(&opts, rotation)?;
        if rotation != self.fbink.current_rotation()? {
            return Err(anyhow!("The rotation changed during recording."));
        }
        let response = RecordActionResponse {
            name: action.name.clone(),
            path_segment: path_segment.clone(),
            sort_value: action.sort_value.clone(),
            keyboard_shortcut: action.keyboard_shortcut,
            rotation: rotation.to_string(),
            was_optimized: recording.is_optimized,
            device: recording.dev_name.clone(),
        };
        self.recordings.add(path_segment, recording, rotation)?;

        Ok(response)
    }

    fn play(&mut self, path_segment: &str) -> Result<()> {
        // Don't play consecutive actions immediately so the device has time to act on the input.
        // Allows a user to spam page turns and have them all register
        if Utc::now() < self.play_wait_until {
            sleep(self.play_wait_until - Utc::now());
        }
        let rotation = self.fbink.current_rotation()?;
        let recording = self.recordings.get(path_segment, rotation)?;
        let opts = self.actions.data.get(path_segment).unwrap();
        recording.play(path_segment)?;
        self.play_wait_until = Utc::now() + opts.post_playback_delay;
        Ok(())
    }

    fn delete(&mut self, path_segment: &str) -> Result<()> {
        if self.actions.data.remove(path_segment).is_some() {
            self.actions.write()?;
            self.recordings.data.remove(path_segment);
            self.recordings.write()?;
            Ok(())
        } else {
            Err(anyhow!("No action exists for {path_segment}"))
        }
    }

    fn update(&mut self, path_segment: String, opts: ActionOptions) -> Result<()> {
        if !self.actions.data.contains_key(&path_segment) {
            return Err(anyhow!("{path_segment} doesn't exist"));
        }

        self.actions.data.insert(path_segment, opts);
        self.actions.write()?;
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
                Some(ActionMsg::Play { path_segment, resp }) => {
                    let result = self.play(&path_segment);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Play result. Receiver dropped")
                    }
                }
                Some(ActionMsg::List { resp }) => {
                    let mut actions = Vec::new();
                    for (path_segment, opts) in self.actions.data.iter() {
                        actions.push(ListActionResponse {
                            name: opts.name.clone(),
                            path_segment: path_segment.clone(),
                            sort_value: opts.sort_value.clone(),
                            keyboard_shortcut: opts.keyboard_shortcut,
                            post_playback_delay: opts.post_playback_delay,
                            voice_trigger: opts.voice_trigger.clone(),
                        })
                    }
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
                Some(ActionMsg::Update {
                    path_segment,
                    opts,
                    resp,
                }) => {
                    let result = self.update(path_segment, opts);
                    if resp.send(result).is_err() {
                        warn!("Unable to send Update result. Receiver dropped")
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
        resp: oneshot::Sender<Vec<ListActionResponse>>,
    },
    Delete {
        path_segment: String,
        resp: oneshot::Sender<Result<()>>,
    },
    Update {
        path_segment: String,
        opts: ActionOptions,
        resp: oneshot::Sender<Result<()>>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordActionResponse {
    pub name: String,
    pub path_segment: String,
    pub sort_value: String,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    pub rotation: String,
    pub was_optimized: bool,
    pub device: String,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionOptions {
    pub name: String,
    pub sort_value: String,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
    pub voice_trigger: Option<String>,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListActionResponse {
    pub name: String,
    pub path_segment: String,
    pub sort_value: String,
    pub keyboard_shortcut: Option<keyboard_types::Code>,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
    pub voice_trigger: Option<String>,
}

impl ListActionResponse {
    pub fn shortcut_name(&self) -> String {
        self.keyboard_shortcut
            .map_or("None".to_string(), |s| s.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionRecording {
    pub rotation: CanonicalRotation,
    pub events: Vec<ActionEvent>,
    pub dev_path: PathBuf,
    pub dev_name: String,
    pub is_optimized: bool,
}

impl ActionRecording {
    pub fn record(
        opts: &RecordActionOptions,
        rotation: CanonicalRotation,
    ) -> Result<ActionRecording> {
        let devices = get_input_devices(opts.use_by_path)?;

        let devices_with_events = if opts.only_check_touch {
            read_input(
                devices.into_iter().filter(is_touch_device),
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

        log_events(&events);
        let is_optimized = if opts.optimize {
            optimize_events(&mut events, opts.syn_gap, opts.optimize_max_duration)
        } else {
            false
        };
        if is_optimized {
            log_events(&events);
        }

        let dev_name = device.to_string();
        Ok(ActionRecording {
            dev_path: device.path,
            events: create_action_events(&events),
            rotation,
            dev_name,
            is_optimized,
        })
    }

    pub fn open_device(&self) -> Result<File> {
        File::options()
            .read(true)
            .write(true)
            .open(&self.dev_path)
            .with_context(|| format!("Failed to open {} to write events", self.dev_path.display()))
    }

    pub fn play(&self, path_segment: &str) -> Result<()> {
        let mut f = self.open_device()?;
        debug!(
            "Writing events for {} to {}",
            path_segment,
            &self.dev_path.display()
        );
        for ev in &self.events {
            f.write_all(&ev.buf)
                .with_context(|| format!("Failed to write event to {}", self.dev_path.display()))?;
            if let Some(dur) = ev.sleep_duration {
                sleep(dur);
            }
        }
        debug!("Finished writing events for {}", path_segment);

        Ok(())
    }
}

fn log_events(events: &[InputEvent]) {
    for e in events {
        debug!(
            "{}.{} {} {}",
            e.time.tv_sec, e.time.tv_usec, e.event_code, e.value
        );
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
    pub voice_trigger: Option<String>,
    pub only_check_touch: bool,
    pub optimize: bool,
    pub use_by_path: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub optimize_max_duration: Duration,
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
            voice_trigger: None,
            only_check_touch: true,
            optimize: false,
            use_by_path: false,
            optimize_max_duration: Duration::milliseconds(1000),
            post_playback_delay: Duration::milliseconds(300),
            syn_gap: Duration::microseconds(1),
            no_input_timeout: Duration::milliseconds(5000),
            new_event_timeout: Duration::milliseconds(4000),
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

// Might have been better to just store InputEvents and convert to bytes when writing to the
// device, but can't change that easily now without breaking existing action recordings.
// libc::time_t is deprecated but not sure how to deal with it so ignoring it for now
// https://github.com/rust-lang/libc/issues/1848
#[allow(deprecated)]
impl ActionEvent {
    /// Convert the ActionEvent back into an InputEvent
    pub fn input_event(&self) -> Result<InputEvent> {
        let i = std::mem::size_of::<libc::time_t>();
        let expected = (i * 2) + 8;
        if self.buf.len() != expected {
            return Err(anyhow!(
                "Can't convert ActionEvent. Expected len of {expected} but found {}",
                self.buf.len()
            ));
        }
        let time = libc::timeval {
            tv_sec: libc::time_t::from_ne_bytes(self.buf[..i].try_into()?),
            tv_usec: libc::suseconds_t::from_ne_bytes(self.buf[i..i + i].try_into()?),
        };
        let rem = &self.buf[i + i..];

        let raw = libc::input_event {
            time,
            type_: libc::__u16::from_ne_bytes([rem[0], rem[1]]),
            code: libc::__u16::from_ne_bytes([rem[2], rem[3]]),
            value: i32::from_ne_bytes([rem[4], rem[5], rem[6], rem[7]]),
        };

        Ok(InputEvent::from_raw(&raw))
    }

    pub fn set_value(&mut self, value: i32) {
        let mut buf = self.buf.clone();
        buf.truncate(buf.len() - 4);
        buf.extend(value.to_ne_bytes());
        self.buf = buf;
    }

    pub fn set_time(&mut self, duration: Duration) {
        let mut buf = Vec::with_capacity(self.buf.len());
        let seconds = duration.num_seconds() as libc::time_t;
        let microseconds = duration.num_microseconds().unwrap() as libc::suseconds_t % 1_000_000;
        buf.extend(seconds.to_ne_bytes());
        buf.extend(microseconds.to_ne_bytes());
        let i = std::mem::size_of::<libc::time_t>();
        buf.extend_from_slice(&self.buf[i + i..]);
        self.buf = buf;
    }
}

#[allow(deprecated)]
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
        let seconds = time_since_start.num_seconds() as libc::time_t;
        let microseconds =
            time_since_start.num_microseconds().unwrap() as libc::suseconds_t % 1_000_000;
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
pub fn parse_timeval(tv: TimeVal) -> DateTime<Utc> {
    let nsec = tv.tv_usec as u32 * 1000;
    // tv_sec will be i32 on 32bit platforms
    DateTime::from_timestamp(tv.tv_sec.into(), nsec).unwrap()
}

pub struct ActionsFile {
    pub path: PathBuf,
    pub data: BTreeMap<String, ActionOptions>,
}

impl ActionsFile {
    pub fn load(path: PathBuf) -> Result<Self> {
        let data = if path.exists() {
            debug!("Loading actions from {}", path.display());
            let file = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read actions from {}", &path.display()))?;
            let mut data: BTreeMap<String, ActionOptions> =
                toml::from_str(&file).with_context(|| {
                    format!("Failed to deserialize actions from {}", &path.display())
                })?;
            // voice_trigger was introduced in 0.3.0, so set it for page turns recorded with older
            // versions.
            data.entry("next-page".to_string()).and_modify(|a| {
                if a.voice_trigger.is_none() {
                    a.voice_trigger = Some("Forward".to_string());
                }
            });
            data.entry("prev-page".to_string()).and_modify(|a| {
                if a.voice_trigger.is_none() {
                    a.voice_trigger = Some("Backward".to_string());
                }
            });
            data
        } else {
            debug!("No action file at {}", path.display());
            BTreeMap::new()
        };
        Ok(Self { path, data })
    }

    pub fn write(&self) -> Result<()> {
        let serialized = toml::to_string(&self.data).context("Failed to serialize actions")?;
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("toml.bkp"))
                .context("Failed to backup actions file")?;
        }
        let tmp = self.path.with_extension("tmp");
        debug!("Writing actions to {}", tmp.display());
        fs::write(&tmp, serialized)
            .with_context(|| format!("Failed to write actions to {}", tmp.display()))?;
        fs::rename(&tmp, &self.path).context("Failed to rename temporary actions file")?;
        Ok(())
    }
}

pub struct RecordingsFile {
    pub path: PathBuf,
    pub data: BTreeMap<String, [Option<ActionRecording>; 4]>,
}

impl RecordingsFile {
    pub fn load(path: PathBuf) -> Result<Self> {
        let data = if path.exists() {
            debug!("Loading recordings from {}", path.display());
            let bytes = fs::read(&path)
                .with_context(|| format!("Failed to read recordings from {}", &path.display()))?;
            bincode::deserialize(&bytes).with_context(|| {
                format!("Failed to deserialize recordings from {}", &path.display())
            })?
        } else {
            debug!("No recordings file at {}", path.display());
            BTreeMap::new()
        };
        Ok(Self { path, data })
    }
    pub fn write(&self) -> Result<()> {
        let bytes = bincode::serialize(&self.data).context("Failed to serialize recordings")?;
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("bin.bkp"))
                .context("Failed to backup recordings file")?;
        }
        let tmp = self.path.with_extension("tmp");
        debug!("Writing recordings to {}", tmp.display());
        fs::write(&tmp, bytes)
            .with_context(|| format!("Failed to write recordings to {}", tmp.display()))?;
        fs::rename(&tmp, &self.path).context("Failed to rename temporary recordings file")?;
        Ok(())
    }

    pub fn get_any(&self, path_segment: &str) -> Result<&ActionRecording> {
        let Some(recordings) = self.data.get(path_segment) else {
            return Err(anyhow!("No recordings for {path_segment}"));
        };
        for r in recordings.iter() {
            if r.is_some() {
                return Ok(r.as_ref().unwrap());
            }
        }
        Err(anyhow!("No recordings for {path_segment}"))
    }

    pub fn get(&self, path_segment: &str, rotation: CanonicalRotation) -> Result<&ActionRecording> {
        let Some(recordings) = self.data.get(path_segment) else {
            return Err(anyhow!(
                "No recording for {path_segment} in {rotation} rotation"
            ));
        };
        match recordings[rotation as usize] {
            Some(ref recording) => Ok(recording),
            None => Err(anyhow!(
                "No recording for {path_segment} in {rotation} rotation"
            )),
        }
    }

    pub fn add(
        &mut self,
        path_segment: String,
        recording: ActionRecording,
        rotation: CanonicalRotation,
    ) -> Result<()> {
        let recordings = self.data.entry(path_segment).or_default();
        recordings[rotation as usize] = Some(recording);
        self.write()?;
        Ok(())
    }
}
