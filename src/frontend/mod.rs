use crate::{actions::ActionMsg, errors::AppError, kobo_config::KoboConfigFile, server::AppState};

use std::collections::HashMap;

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
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
        .route("/manage-actions", get(manage_actions))
        .route("/remote-control", get(remote_control))
        .route("/developer-settings", get(developer_settings))
        .route("/styles/main.css", get(main_css))
        .route("/styles/remote.css", get(remote_css))
        .route("/js/record-action.js", get(record_action_js))
        .route("/js/colored-buttons.js", get(colored_buttons))
        .route("/js/lib.js", get(lib_js))
}

async fn main_css() -> impl IntoResponse {
    (css_header(), include_str!("styles/main.css"))
}

async fn remote_css() -> impl IntoResponse {
    (css_header(), include_str!("styles/remote.css"))
}

async fn record_action_js() -> impl IntoResponse {
    (js_header(), include_str!("js/record-action.js"))
}

async fn lib_js() -> impl IntoResponse {
    (js_header(), include_str!("js/lib.js"))
}

async fn colored_buttons() -> impl IntoResponse {
    (js_header(), include_str!("js/colored-buttons.js"))
}

async fn remote_control(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    let mut shortcuts = HashMap::new();
    for a in &actions {
        shortcuts.insert(a.path_segment.clone(), a.keyboard_shortcut);
    }
    let shortcuts_json = serde_json::to_string_pretty(&shortcuts)?;
    Ok(templates::RemoteControl {
        actions,
        shortcuts_json,
    })
}

async fn developer_settings() -> Result<impl IntoResponse, AppError> {
    let config = KoboConfigFile::open(Default::default())?;
    let settings = config.get_values();
    Ok(templates::DeveloperSettings { settings })
}

async fn manage_actions(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    Ok(templates::ManageActions { actions })
}

fn js_header() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    let value = HeaderValue::from_static("text/javascript");
    headers.insert(header::CONTENT_TYPE, value);
    headers
}

fn css_header() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    headers
}
