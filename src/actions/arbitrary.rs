//! Handles performing input and printing a cursor at arbitrary locations on the screen
use super::input::{is_x_coord, is_y_coord};
use super::{ActionEvent, ActionRecording};
use crate::config::Config;
use crate::server::AppState;
use crate::util::sleep;

use std::fmt::Display;
use std::io::Write;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::spawn;
use strum::Display;

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    extract::State,
    response::Response,
    routing::get,
    Router,
};

use chrono::{DateTime, Duration, Utc};
use evdev_rs::enums::EventType::EV_SYN;
use fbink_rs::dump::Dump;
use fbink_rs::image::{self, DynamicImage};
use fbink_rs::{CanonicalRotation, FbInk, FbInkRect};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use tracing::{debug, error, trace, warn};

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputOptions {
    pub cursor_width: u16,
    pub cursor_height: u16,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub cursor_min_refresh: Duration,
    pub custom_cursor_path: PathBuf,
    pub cursor_invert_color: bool,
    pub reload_background_after_input: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub reload_background_delay: Duration,
    pub client: ClientInputOptions,
    pub use_overrides: bool,
    pub swap_axes_override: bool,
    pub mirror_x_override: bool,
    pub mirror_y_override: bool,
}

impl Default for InputOptions {
    fn default() -> Self {
        Self {
            cursor_width: 32,
            cursor_height: 50,
            custom_cursor_path: "cursor.png".into(),
            cursor_min_refresh: Duration::milliseconds(200),
            cursor_invert_color: false,
            reload_background_after_input: true,
            reload_background_delay: Duration::milliseconds(1500),
            client: Default::default(),
            use_overrides: false,
            swap_axes_override: true,
            mirror_x_override: true,
            mirror_y_override: false,
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientInputOptions {
    pub start_on_longpress: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub start_press_duration: Duration,
    pub start_on_swipe: bool,
    pub swipe_prevent_default: bool,
    pub start_swipe_min_distance: u16,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub move_send_wait: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub touch_wait_duration: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub short_press_duration: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub long_press_duration: Duration,
    pub sensitivity: f32,
    pub tap_distance_cutoff: u8,
    pub arrow_move_distance: u8,
    pub control_move_multiplier: f32,
    pub shift_move_multiplier: f32,
    pub final_move_send_delay: u16,
    pub start_shortcut: keyboard_types::Code,
}

impl Default for ClientInputOptions {
    fn default() -> Self {
        Self {
            start_on_longpress: true,
            start_on_swipe: false,
            start_swipe_min_distance: 200,
            swipe_prevent_default: false,
            start_press_duration: Duration::milliseconds(1000),
            move_send_wait: Duration::milliseconds(500),
            touch_wait_duration: Duration::milliseconds(200),
            short_press_duration: Duration::milliseconds(1),
            long_press_duration: Duration::milliseconds(600),
            sensitivity: 2.0,
            tap_distance_cutoff: 5,
            arrow_move_distance: 20,
            control_move_multiplier: 3.0,
            shift_move_multiplier: 6.0,
            final_move_send_delay: 100,
            start_shortcut: keyboard_types::Code::KeyE,
        }
    }
}

/// Receives InputMsgs from clients. Writes ActionEvents to
/// an input device and sends CursorMsgs to the CursorManager
pub struct InputManager {
    opts: InputOptions,
    template: ActionRecording,
    fbink: Arc<FbInk>,
    rota: CanonicalRotation,
    start_events: Vec<ActionEvent>,
    move_events: Vec<ActionEvent>,
    stop_events: Vec<ActionEvent>,
    start_time: Option<DateTime<Utc>>,
    swap_axes: bool,
    mirror_x: bool,
    mirror_y: bool,
    screen_width: u32,
    screen_height: u32,
    cursor_x_max: f64,
    cursor_y_max: f64,
    current_coord: Option<Coord>,
    cursor: DynamicImage,
    tx: Option<CursorSender>,
    rx: InputReceiver,
}

#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum InputMsg {
    StartInput(Option<Coord>),
    StopInput(Option<Coord>),
    MoveAbsolute(Coord),
    MoveRelative(Coord),
    Reinit,
    ClientConnect,
    ClientDisconnect,
    Shutdown,
}

#[derive(Debug)]
pub struct InputMsgWrapper {
    pub msg: InputMsg,
    pub resp: tokio::sync::oneshot::Sender<Result<()>>,
}

type InputReceiver = tokio::sync::mpsc::Receiver<InputMsgWrapper>;
pub type InputSender = tokio::sync::mpsc::Sender<InputMsgWrapper>;
type CursorSender = std::sync::mpsc::Sender<CursorMsg>;
type CursorReceiver = std::sync::mpsc::Receiver<CursorMsg>;

/// Manages drawing a cursor on the screen with FBInk
pub struct CursorManager {
    cursor_min_refresh: Duration,
    current_coord: Option<Coord>,
    min_change: i32,
    last_draw: Option<LastDraw>,
    fbink: Arc<FbInk>,
    cursor: DynamicImage,
    rx: CursorReceiver,
}

#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum CursorMsg {
    Draw(Coord),
    Hide,
    ReloadBackground,
    Reinit,
    Stop,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
/// Message to send back to the client
pub enum ClientMsg {
    Error(String),
}

impl InputManager {
    pub fn new(
        template: ActionRecording,
        fbink: Arc<FbInk>,
        config: Config,
        rx: InputReceiver,
    ) -> Result<Self> {
        if !template.is_optimized {
            return Err(anyhow!(
                "An optimized recording is required to use as a template"
            ));
        }
        let state = fbink.state();

        let rota = state.canonical_rotation();
        let mut iter = template.events.iter();
        let start_events = get_event_batch(&mut iter)?;
        let move_events = get_event_batch(&mut iter)?;
        let stop_events = get_event_batch(&mut iter)?;
        let current_coord = None;
        let cursor = Self::load_cursor(&config)?;
        let opts = config.user.arbitrary_input;
        Ok(Self {
            opts,
            template,
            rota,
            fbink,
            start_events,
            move_events,
            stop_events,
            start_time: None,
            swap_axes: state.touch_swap_axes,
            mirror_x: state.touch_mirror_x,
            mirror_y: state.touch_mirror_y,
            screen_width: state.screen_width,
            screen_height: state.screen_height,
            cursor_x_max: (state.screen_width - cursor.width()) as f64,
            cursor_y_max: (state.screen_height - cursor.height()) as f64,
            current_coord,
            cursor,
            tx: None,
            rx,
        })
    }

    pub fn manage(&mut self) -> Result<()> {
        while let Some(InputMsgWrapper { msg, resp }) = self.rx.blocking_recv() {
            let control_flow = match self.handle_msg(msg) {
                Ok(c_f) => {
                    if resp.send(Ok(())).is_err() {
                        error!("InputManager failed to send Result to WebSocket handler")
                    };
                    c_f
                }
                Err(e) => {
                    error!("{e}");
                    if resp.send(Err(e)).is_err() {
                        error!("InputManager failed to send Result to WebSocket handler")
                    };
                    ControlFlow::Break(())
                }
            };
            if control_flow.is_break() {
                break;
            }
        }
        Ok(())
    }

    fn start_cursor_manager(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut cursor =
            CursorManager::new(self.fbink.clone(), self.cursor.clone(), rx, &self.opts);
        self.tx = Some(tx);
        spawn(move || cursor.manage());
        let start = self.get_coord(None);
        self.send(CursorMsg::Draw(start))?;
        Ok(())
    }

    fn stop_cursor_manager(&mut self) -> Result<()> {
        self.send(CursorMsg::Hide)?;
        self.send(CursorMsg::Stop)?;
        self.current_coord = None;
        self.tx = None;
        Ok(())
    }

    fn handle_msg(&mut self, msg: InputMsg) -> Result<ControlFlow<()>> {
        use InputMsg::*;
        debug!("Received InputMsg::{msg}");
        match msg {
            ClientConnect => {
                self.reinit_screen()?;
                if self.tx.is_none() {
                    self.start_cursor_manager()?;
                }
            }
            StartInput(coord) => self.input_start(coord)?,
            StopInput(coord) => self.input_stop(coord)?,
            MoveAbsolute(coord) => self.input_move_abs(coord)?,
            MoveRelative(coord) => self.input_move_rel(coord)?,
            Reinit => self.reinit()?,
            ClientDisconnect => self.stop_cursor_manager()?,
            Shutdown => {
                self.stop_cursor_manager()?;
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn write_events(&self, events: &[ActionEvent], change_time: bool, coord: &Coord) -> Result<()> {
        let p = self.template.dev_path.display();
        let mut f = self.template.open_device()?;

        for action_event in events {
            let mut ae = action_event.clone();
            let ie = action_event.input_event()?;
            if change_time {
                let duration = if let Some(start_time) = self.start_time {
                    Utc::now() - start_time
                } else {
                    warn!("No start time when writing events. Using 1ms Duration");
                    Duration::milliseconds(1)
                };
                ae.set_time(duration);
            }
            if is_x_coord(&ie) {
                ae.set_value(coord.x as i32);
            } else if is_y_coord(&ie) {
                ae.set_value(coord.y as i32);
            };
            f.write_all(&ae.buf)
                .with_context(|| format!("Failed to write event to {p}"))?;
        }
        Ok(())
    }

    fn get_coord(&self, coord: Option<Coord>) -> Coord {
        if let Some(coord) = coord {
            coord
        } else if let Some(coord) = self.current_coord {
            coord
        } else {
            debug!("No existing coordinate. Using middle of screen.");
            let max_width = (self.screen_width - self.cursor.width()) as f64;
            let max_height = (self.screen_height - self.cursor.height()) as f64;
            Coord {
                x: max_width / 2.0,
                y: max_height / 2.0,
            }
        }
    }

    fn input_start(&mut self, coord: Option<Coord>) -> Result<()> {
        self.send(CursorMsg::Hide)?;
        let mut coord = self.get_coord(coord);
        self.translate_coord(&mut coord);

        self.start_time = Some(Utc::now());
        self.write_events(&self.start_events, false, &coord)?;
        Ok(())
    }

    fn input_move_abs(&mut self, mut coord: Coord) -> Result<()> {
        self.current_coord = Some(coord);
        if self.start_time.is_some() {
            self.translate_coord(&mut coord);
            self.write_events(&self.move_events, true, &coord)?;
        } else {
            self.send(CursorMsg::Draw(coord))?;
        }
        Ok(())
    }

    fn input_move_rel(&mut self, relative: Coord) -> Result<()> {
        let current = self.get_coord(None);
        let absolute = Coord {
            x: (current.x + relative.x).max(0.0).min(self.cursor_x_max),
            y: (current.y + relative.y).max(0.0).min(self.cursor_y_max),
        };
        self.input_move_abs(absolute)
    }

    fn input_stop(&mut self, coord: Option<Coord>) -> Result<()> {
        let mut coord = self.get_coord(coord);
        self.translate_coord(&mut coord);
        coord.x += 1.0;
        coord.y += 1.0;
        self.write_events(&self.move_events, true, &coord)?;
        self.write_events(&self.stop_events, true, &coord)?;
        self.start_time = None;
        if self.opts.reload_background_after_input {
            sleep(self.opts.reload_background_delay);
            self.send(CursorMsg::ReloadBackground)?;
        }
        Ok(())
    }

    fn send(&self, msg: CursorMsg) -> Result<(), std::sync::mpsc::SendError<CursorMsg>> {
        if let Some(tx) = &self.tx {
            tx.send(msg)
        } else {
            warn!("Tried to send CursorMsg::{msg} with no Sender");
            Ok(())
        }
    }

    fn reinit(&mut self) -> Result<()> {
        self.send(CursorMsg::Hide)?;
        sleep(Duration::milliseconds(100));
        self.reinit_screen()?;
        self.current_coord = None;
        let start = self.get_coord(None);
        self.send(CursorMsg::Reinit)?;
        self.send(CursorMsg::ReloadBackground)?;
        self.send(CursorMsg::Draw(start))?;
        self.current_coord = Some(start);
        Ok(())
    }

    fn reinit_screen(&mut self) -> Result<()> {
        self.fbink.reinit()?;
        let state = self.fbink.state();
        self.rota = state.canonical_rotation();
        self.screen_width = state.screen_width;
        self.screen_height = state.screen_height;
        self.cursor_x_max = (state.screen_width - self.cursor.width()) as f64;
        self.cursor_y_max = (state.screen_height - self.cursor.height()) as f64;
        Ok(())
    }

    fn load_cursor(config: &Config) -> Result<DynamicImage> {
        let opts = &config.user.arbitrary_input;
        let cursor_path = if opts.custom_cursor_path.is_relative() {
            config.app.user_dir.join(&opts.custom_cursor_path)
        } else {
            opts.custom_cursor_path.clone()
        };
        let cursor = if cursor_path.exists() {
            image::io::Reader::open(cursor_path)?.decode()?
        } else {
            image::load_from_memory(include_bytes!("../../cursor.png"))?
        };
        let mut cursor = cursor.resize(
            opts.cursor_width.into(),
            opts.cursor_height.into(),
            image::imageops::FilterType::Nearest,
        );

        if opts.cursor_invert_color {
            cursor.invert();
        }
        Ok(cursor)
    }

    /// Translate coordinate from canonical rotation to native rotation
    fn translate_coord(&self, coord: &mut Coord) {
        // Adapted from FBInk https://github.com/NiLuJe/FBInk/blob/master/utils/finger_trace.c
        // Note that we swap the axes at the end rather than at the start.  I assume this is
        // required due to translating canonical -> native not native -> canonical, but I'm bad at
        // maths and don't really understand why this works (tested on a Glo and Sage).
        debug!("Input coordinates: {coord}");
        let mut swap_axes = self.swap_axes;
        let mut mirror_x = self.mirror_x;
        let mut mirror_y = self.mirror_y;
        match self.rota {
            CanonicalRotation::Upright => (),
            CanonicalRotation::Clockwise => {
                swap_axes = !swap_axes;
                mirror_y = !mirror_y;
            }
            CanonicalRotation::UpsideDown => {
                mirror_x = !mirror_x;
                mirror_y = !mirror_y;
            }
            CanonicalRotation::CounterClockwise => {
                swap_axes = !swap_axes;
                mirror_x = !mirror_x;
            }
        }
        // Allow users to override the computed values in case they're wrong
        if self.opts.use_overrides {
            swap_axes = self.opts.swap_axes_override;
            mirror_x = self.opts.mirror_x_override;
            mirror_y = self.opts.mirror_x_override;
        }

        if mirror_x {
            coord.x = self.screen_width as f64 - 1.0 - coord.x;
            debug!("Mirrored x coordinates: {coord}");
        }
        if mirror_y {
            coord.y = self.screen_height as f64 - 1.0 - coord.y;
            debug!("Mirrored y coordinates: {coord}");
        }
        if swap_axes {
            std::mem::swap(&mut coord.x, &mut coord.y);
            debug!("Swapped coordinates: {coord}");
        }
    }
}

pub struct LastDraw {
    pub time: DateTime<Utc>,
    pub coord: Coord,
    pub rect: FbInkRect,
}

impl CursorManager {
    pub fn new(
        fbink: Arc<FbInk>,
        cursor: DynamicImage,
        rx: CursorReceiver,
        opts: &InputOptions,
    ) -> Self {
        Self {
            cursor_min_refresh: opts.cursor_min_refresh,
            last_draw: None,
            current_coord: None,
            min_change: 5,
            fbink,
            cursor,
            rx,
        }
    }
    pub fn manage(&mut self) {
        let timeout = self
            .cursor_min_refresh
            .to_std()
            .unwrap_or(std::time::Duration::from_millis(100));
        let mut dump = match self.fbink.dump_workaround_sunxi() {
            Ok(dump) => dump,
            Err(e) => {
                error!("Failed to get FBInk dump. {e}");
                return;
            }
        };
        let min_range = -self.min_change..self.min_change;
        loop {
            match self.rx.recv_timeout(timeout) {
                Ok(msg) => match msg {
                    CursorMsg::Draw(coord) => {
                        self.current_coord = Some(coord);
                        if let Some(last) = &self.last_draw {
                            if last.coord == coord {
                                continue;
                            }
                            if Utc::now() - last.time < self.cursor_min_refresh {
                                continue;
                            }
                            let x_diff = (last.coord.x - coord.x) as i32;
                            let y_diff = (last.coord.y - coord.y) as i32;
                            if min_range.contains(&x_diff) && min_range.contains(&y_diff) {
                                debug!("Cursor hasn't moved enough to trigger redraw");
                                continue;
                            }
                            debug!("Cursor moved. X: {x_diff}, Y: {y_diff}. Redrawing");
                        } else {
                            debug!("Drawing initial cursor");
                        }
                        self.draw_cursor(&mut dump, coord)
                    }
                    CursorMsg::Hide => {
                        self.hide_cursor(&mut dump);
                    }
                    CursorMsg::ReloadBackground => {
                        debug!("Reloading background");
                        dump = match self.fbink.dump_workaround_sunxi() {
                            Ok(dump) => dump,
                            Err(e) => {
                                error!("Failed to get FBInk dump. {e}");
                                return;
                            }
                        };
                    }
                    CursorMsg::Reinit => {
                        self.current_coord = None;
                        self.last_draw = None;
                    }
                    CursorMsg::Stop => {
                        debug!("Stopping CursorManager");
                        return;
                    }
                },
                // Always draw the cursor at the last known location once the mouse/finger has
                // stopped moving
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    let Some(current) = self.current_coord else {
                        continue;
                    };
                    let Some(last) = &self.last_draw else {
                        continue;
                    };
                    if current != last.coord {
                        let x_diff = (last.coord.x - current.x) as i32;
                        let y_diff = (last.coord.y - current.y) as i32;
                        if min_range.contains(&x_diff) && min_range.contains(&y_diff) {
                            debug!("Cursor hasn't moved enough to trigger redraw");
                            continue;
                        }
                        self.draw_cursor(&mut dump, current)
                    }
                    sleep(self.cursor_min_refresh)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    warn!("CursorManager receiver was disconnected");
                    return;
                }
            }
        }
    }
    // Remove the last drawn cursor from the screen
    fn hide_cursor(&mut self, dump: &mut Box<dyn Dump>) {
        if let Some(last) = &self.last_draw {
            debug!("Restoring dump where last cursor was drawn");
            dump.crop_rect(last.rect);
            if let Err(e) = dump.restore(&self.fbink) {
                error!("Failed to restore dump. {e}");
            }
            self.last_draw = None;
        }
    }

    fn draw_cursor(&mut self, dump: &mut Box<dyn Dump>, coord: Coord) {
        let x = coord.x as i64;
        let y = coord.y as i64;

        self.hide_cursor(dump);
        if let Err(e) = dump.print_overlay(&self.fbink, &self.cursor, x as u32, y as u32) {
            error!("Failed to print cursor. {e}");
        }

        self.last_draw = Some(LastDraw {
            time: Utc::now(),
            coord,
            rect: self.fbink.get_last_rect(false),
        });
    }
}

fn get_event_batch<'a>(
    iter: &mut impl Iterator<Item = &'a ActionEvent>,
) -> Result<Vec<ActionEvent>> {
    let mut events = Vec::new();
    for action_event in iter.by_ref() {
        events.push(action_event.clone());
        let ie = action_event.input_event()?;
        if ie.is_type(&EV_SYN) {
            break;
        }
    }
    Ok(events)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

impl Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(handler))
}

async fn handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    debug!("Received request to /ws endpoint");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    use axum::extract::ws::Message::*;
    send_input_msg(&state, &mut socket, InputMsg::ClientConnect).await;
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Text(text) => {
                trace!("Received {text}");
                let Ok(msg) = serde_json::from_str::<InputMsg>(&text) else {
                    error!("Received invalid Text from WebSocket. {text}");
                    continue;
                };
                send_input_msg(&state, &mut socket, msg).await;
            }
            Binary(_) => {
                debug!("Received unexpected Binary message from WebSocket");
            }
            // Ping and Pong are handled by axum
            Ping(_) => (),
            Pong(_) => (),
            Close(_) => {
                send_input_msg(&state, &mut socket, InputMsg::ClientDisconnect).await;
                return;
            }
        }
    }
    // client disconnected without sending Close
    send_input_msg(&state, &mut socket, InputMsg::ClientDisconnect).await;
}

async fn send_input_msg(state: &AppState, socket: &mut WebSocket, msg: InputMsg) {
    if let Err(e) = state.send_input_msg(msg).await {
        error!("{e}");
        let string = serde_json::to_string(&ClientMsg::Error(e.to_string())).unwrap();
        let message = axum::extract::ws::Message::Text(string);
        if let Err(e) = socket.send(message).await {
            error!("Failed to send error to WebSocket client. {e}");
        }
    }
}
