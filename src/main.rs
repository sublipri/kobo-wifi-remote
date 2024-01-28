use std::{path::PathBuf, thread};

use actions::ActionMsg;
use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tokio::sync::{mpsc, oneshot};
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    actions::{ActionManager, RecordActionOptions},
    errors::AppError,
};

mod actions;
mod errors;
mod input;

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
        .route("/", get(|| async { "Hello, World!" }))
        // Now that we're not restricted by the BusyBox HTTP server we'll switch to an
        // /actions/:uri endpoint, but keep /left and /right for compatibility with KoboPageTurner
        // and anything that users integrated with the original version
        .route("/actions", post(record_action))
        .route("/actions/:uri", get(play_action))
        .route("/left", get(prev_page))
        .route("/left/", get(prev_page))
        .route("/right", get(next_page))
        .route("/right/", get(next_page))
        .with_state(state);

    let host = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}

async fn next_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_uri("next-page".into(), &state).await?)
}

async fn prev_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_uri("prev-page".into(), &state).await?)
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
