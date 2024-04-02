use crate::{
    actions::{arbitrary::InputManager, ActionManager, ActionMsg},
    config::Config,
    init::init,
};

use std::{sync::Arc, thread};

use anyhow::{Context, Result};
use axum::{
    extract::Request,
    http::{header, HeaderValue},
    Router, ServiceExt,
};
use fbink_rs::{config::Font, FbInk, FbInkConfig};
use tokio::sync::mpsc;
use tower::Layer;
use tower_http::{
    normalize_path::NormalizePathLayer, set_header::response::SetResponseHeaderLayer,
};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub tx: mpsc::Sender<ActionMsg>,
    pub fbink: Arc<FbInk>,
    pub config: Config,
}

#[tokio::main(flavor = "current_thread")]
pub async fn serve(config: &Config) -> Result<()> {
    let (tx, rx) = mpsc::channel(32);
    let fbink = Arc::new(FbInk::new(FbInkConfig {
        is_centered: true,
        is_halfway: true,
        is_padded: true,
        font: Font::Fatty,
        to_syslog: true,
        ..Default::default()
    })?);
    init(config, fbink.clone())?;
    let state = AppState {
        tx,
        fbink: fbink.clone(),
        config: config.clone(),
    };
    let mut manager =
        ActionManager::from_path(config.action_file(), config.recordings_file(), fbink, rx)?;
    if config.arbitrary.enabled {
        if let Ok(template) = manager.recordings.get_any("next-page") {
            let mut input_manager =
                InputManager::new(template.clone(), state.fbink.clone(), config)?;
            thread::spawn(move || input_manager.manage());
        } else {
            info!("No recording to use as template. Not running arbitrary input manager.");
        }
    }
    thread::spawn(move || manager.manage());
    let app = Router::new()
        .merge(crate::config::routes())
        .merge(crate::actions::routes())
        .merge(crate::frontend::routes())
        .merge(crate::kobo_config::routes())
        .merge(crate::screenshot::routes())
        .merge(crate::logging::routes())
        .with_state(state);

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    )
    .layer(app);
    let app = ServiceExt::<Request>::into_make_service(app);
    let host = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .with_context(|| format!("Failed to bind TcpListener to {}", &host))?;
    info!("Server listening on {host}");
    axum::serve(listener, app)
        .await
        .context("Failed to start Axum server")?;
    Ok(())
}
