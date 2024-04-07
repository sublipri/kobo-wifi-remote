use crate::{actions::ActionMsg, errors::AppError, kobo_config::KoboConfigFile, server::AppState};

use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::sync::oneshot;

mod templates;
pub mod index;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/setup", get(|| async { templates::Setup {} }))
        .route("/page-turner", get(page_turner))
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
        .route("/auto-turner", get(auto_turner))
        .route("/developer-settings", get(developer_settings))
        .route("/styles/main.css", get(main_css))
        .route("/styles/remote.css", get(remote_css))
        .route("/js/record-action.js", get(record_action_js))
        .route("/js/colored-buttons.js", get(colored_buttons))
        .route("/js/lib.js", get(lib_js))
        .route("/js/arbitrary-input.js", get(arbitrary_input_js))
        .route("/js/auto-turner.js", get(auto_turner_js))
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

async fn arbitrary_input_js() -> impl IntoResponse {
    (js_header(), include_str!("js/arbitrary-input.js"))
}

async fn index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let items = state.config.index.items.iter().filter(|i| i.enabled);
    let index = templates::Index {
        items: items.collect(),
        button_height: state.config.index.button_height,
    };
    Ok((html_header(), index.render()?))
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
        enable_arbitrary: state.config.arbitrary_input.enabled,
        prompt_fullscreen: state.config.prompt_fullscreen,
    })
}

async fn page_turner(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    let mut next = None;
    let mut prev = None;
    for action in actions.into_iter() {
        match action.path_segment.as_str() {
            "next-page" => next = Some(action),
            "prev-page" => prev = Some(action),
            _ => continue,
        }
    }
    Ok(templates::PageTurner {
        next,
        prev,
        enable_arbitrary: state.config.arbitrary_input.enabled,
        prompt_fullscreen: state.config.prompt_fullscreen,
    })
}

async fn auto_turner(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    let mut next = None;
    let mut prev = None;
    for action in actions.into_iter() {
        match action.path_segment.as_str() {
            "next-page" => next = Some(action),
            "prev-page" => prev = Some(action),
            _ => continue,
        }
    }
    Ok(templates::AutoTurner { next, prev, delay: state.config.auto_turner_delay })
}

async fn auto_turner_js() -> impl IntoResponse {
    (js_header(), include_str!("js/auto-turner.js"))
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
    make_header("text/javascript")
}

fn css_header() -> HeaderMap {
    make_header("text/css")
}

fn html_header() -> HeaderMap {
    make_header("text/html")
}

fn make_header(value: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(value));
    headers
}
