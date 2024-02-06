use std::{collections::HashMap, path::PathBuf, thread};

use actions::ActionMsg;
use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Request, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router, ServiceExt,
};
use templates::RemoteControl;
use tokio::sync::{mpsc, oneshot};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    actions::{ActionManager, RecordActionOptions},
    errors::AppError,
};

mod actions;
mod errors;
mod input;
mod templates;

pub struct ConfigOptions {
    pub action_file: PathBuf,
    pub port: u32,
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            action_file: "/tmp/actions.bin".into(),
            port: 8000,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    tx: mpsc::Sender<ActionMsg>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    debug!("Starting wifiremote");

    let config = ConfigOptions::default();
    let (tx, rx) = mpsc::channel(32);
    let mut manager = ActionManager::from_path(config.action_file.clone(), rx)?;
    let state = AppState { tx };
    thread::spawn(move || manager.manage());
    let app = Router::new()
        .route("/", get(|| async { templates::Index {} }))
        .route("/setup", get(|| async { templates::Setup {} }))
        .route("/page-turner", get(|| async { templates::PageTurner {} }))
        .route(
            "/troubleshooting",
            get(|| async { templates::Troubleshooting {} }),
        )
        .route(
            "/custom-actions",
            get(|| async { templates::CustomActions {} }),
        )
        .route("/remote-control", get(remote_control))
        // Now that we're not restricted by the BusyBox HTTP server we'll switch to an
        // /actions/:uri endpoint, but keep /left and /right for compatibility with KoboPageTurner
        // and anything that users integrated with the original version
        .route("/actions", get(get_actions))
        .route("/actions", post(record_action))
        .route("/actions/:uri", get(play_action))
        .route("/left", get(prev_page))
        .route("/right", get(next_page))
        .route("/styles/main.css", get(main_css))
        .route("/styles/remote.css", get(remote_css))
        .route("/js/alert-recording.js", get(alert_recording))
        .route("/js/colored-buttons.js", get(colored_buttons))
        .with_state(state);

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app = ServiceExt::<Request>::into_make_service(app);
    let host = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}

async fn main_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    (headers, include_str!("www/styles/main.css"))
}

async fn remote_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    (headers, include_str!("www/styles/remote.css"))
}

async fn alert_recording() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript"),
    );
    (headers, include_str!("www/js/alert-recording.js"))
}

async fn colored_buttons() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript"),
    );
    (headers, include_str!("www/js/colored-buttons.js"))
}

async fn next_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_uri("next-page".into(), &state).await?)
}

async fn prev_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_uri("prev-page".into(), &state).await?)
}

async fn remote_control(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    dbg!(&actions);
    let mut shortcuts = HashMap::new();
    for a in &actions {
        if let Some(shortcut) = a.keyboard_shortcut {
            shortcuts.insert(shortcut, a.uri.clone());
        }
    }
    let shortcuts_json = serde_json::to_string_pretty(&shortcuts)?;
    Ok(RemoteControl { actions, shortcuts_json })
}

async fn get_actions(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    Ok(Json(actions))
}

async fn play_action(
    State(state): State<AppState>,
    AxumPath(uri): AxumPath<String>,
) -> Result<impl IntoResponse, AppError> {
    Ok(play_uri(uri, &state).await?)
}

async fn play_uri(uri: String, state: &AppState) -> Result<()> {
    debug!("Received request to play action {uri}");
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::Play { uri, resp: tx }).await?;
    rx.await??;
    debug!("Successfully played action");
    Ok(())
}

async fn record_action(
    State(state): State<AppState>,
    Json(opts): Json<RecordActionOptions>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Received request to record action: {:?}", &opts);
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::Record { opts, resp: tx }).await?;
    rx.await??;
    debug!("Successfully recorded action");
    Ok(())
}
