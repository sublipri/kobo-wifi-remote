use crate::{
    actions::{ActionManager, ActionMsg},
    config::Config,
    init::first_run,
};

use std::thread;

use anyhow::Result;
use axum::{
    extract::Request,
    http::{header, HeaderValue},
    Router, ServiceExt,
};
use tokio::sync::mpsc;
use tower::Layer;
use tower_http::{
    normalize_path::NormalizePathLayer, set_header::response::SetResponseHeaderLayer,
};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub tx: mpsc::Sender<ActionMsg>,
}

#[tokio::main(flavor = "current_thread")]
pub async fn serve() -> Result<()> {
    let config = Config::default();
    if !config.action_file().exists() {
        first_run(&config)?;
    }
    let (tx, rx) = mpsc::channel(32);
    let mut manager = ActionManager::from_path(config.action_file(), rx)?;
    let state = AppState { tx };
    thread::spawn(move || manager.manage());
    let app = Router::new()
        .merge(crate::actions::routes())
        .merge(crate::frontend::routes())
        .merge(crate::kobo_config::routes())
        .merge(crate::screenshot::routes())
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
    info!("Starting server on port {}", config.port);
    axum::serve(listener, app).await?;
    Ok(())
}
