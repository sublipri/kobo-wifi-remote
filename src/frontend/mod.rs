use crate::{actions::ActionMsg, errors::AppError, AppState};

use std::collections::HashMap;

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use templates::RemoteControl;
use tokio::sync::oneshot;

mod templates;

pub fn routes() -> Router<AppState> {
    Router::new()
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
        .route("/styles/main.css", get(main_css))
        .route("/styles/remote.css", get(remote_css))
        .route("/js/record-action.js", get(record_action_js))
        .route("/js/colored-buttons.js", get(colored_buttons))
}

async fn main_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    (headers, include_str!("styles/main.css"))
}

async fn remote_css() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    (headers, include_str!("styles/remote.css"))
}

async fn record_action_js() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript"),
    );
    (headers, include_str!("js/record-action.js"))
}

async fn colored_buttons() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript"),
    );
    (headers, include_str!("js/colored-buttons.js"))
}

async fn remote_control(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    let mut shortcuts = HashMap::new();
    for a in &actions {
        if let Some(shortcut) = a.keyboard_shortcut {
            shortcuts.insert(shortcut, a.path_segment.clone());
        }
    }
    let shortcuts_json = serde_json::to_string_pretty(&shortcuts)?;
    Ok(RemoteControl {
        actions,
        shortcuts_json,
    })
}
