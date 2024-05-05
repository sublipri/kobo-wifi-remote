use crate::{
    actions::{arbitrary::InputManager, ActionManager, ActionMsg},
    config::Config,
    init::init,
};

use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
};

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
use tracing::{error, info};

#[derive(Clone)]
pub struct AppState {
    pub tx: mpsc::Sender<ActionMsg>,
    pub fbink: Arc<FbInk>,
    pub config: Arc<Mutex<Config>>,
}

impl AppState {
    pub fn config(&self) -> MutexGuard<'_, Config> {
        self.config.lock().expect("Failed to lock Config")
    }
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
        config: Arc::new(Mutex::new(config.clone())),
    };
    let mut manager =
        ActionManager::from_path(config.action_file(), config.recordings_file(), fbink, rx)?;
    let c = &config.user;
    if c.page_turner.enable_arbitrary_input || c.remote_control.enable_arbitrary_input {
        if let Ok(template) = manager.recordings.get_any("next-page") {
            match InputManager::new(template.clone(), state.fbink.clone(), config) {
                Ok(mut input_manager) => {
                    thread::spawn(move || input_manager.manage());
                }
                Err(e) => error!("Failed to start arbitrary input manager. {e}"),
            };
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
    let host = format!("0.0.0.0:{}", config.app.port);
    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .with_context(|| format!("Failed to bind TcpListener to {}", &host))?;
    info!("Server listening on {host}");
    axum::serve(listener, app)
        .await
        .context("Failed to start Axum server")?;
    Ok(())
}
