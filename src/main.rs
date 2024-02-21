use crate::{
    actions::{ActionManager, ActionMsg, RecordActionOptions},
    errors::AppError,
};

use std::{path::PathBuf, thread};

use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Request, State},
    http::{header, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router, ServiceExt,
};
use tokio::sync::{mpsc, oneshot};
use tower::Layer;
use tower_http::{
    normalize_path::NormalizePathLayer, set_header::response::SetResponseHeaderLayer,
};
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod actions;
mod errors;
mod frontend;
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
        // Now that we're not restricted by the BusyBox HTTP server we'll switch to an
        // /actions/:path_segment endpoint, but keep /left and /right for compatibility with KoboPageTurner
        // and anything that users integrated with the original version
        .route("/actions", get(get_actions))
        .route("/actions", post(record_action))
        .route("/actions/:path_segment", get(play_action_handler))
        .route("/left", get(prev_page))
        .route("/right", get(next_page))
        .merge(frontend::routes())
        .with_state(state);

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    )
    .layer(app);
    let app = ServiceExt::<Request>::into_make_service(app);
    let host = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}

async fn next_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_action("next-page".into(), &state).await?)
}

async fn prev_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(play_action("prev-page".into(), &state).await?)
}

async fn get_actions(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    Ok(Json(actions))
}

async fn play_action_handler(
    State(state): State<AppState>,
    AxumPath(path_segment): AxumPath<String>,
) -> Result<impl IntoResponse, AppError> {
    Ok(play_action(path_segment, &state).await?)
}

async fn play_action(path_segment: String, state: &AppState) -> Result<()> {
    debug!("Received request to play action {path_segment}");
    let (tx, rx) = oneshot::channel();
    let msg = ActionMsg::Play {
        path_segment,
        resp: tx,
    };
    state.tx.send(msg).await?;
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
    let response = rx.await??;
    debug!("Successfully recorded action");
    Ok(Json(response))
}
