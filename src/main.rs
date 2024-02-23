use crate::actions::{ActionManager, ActionMsg};

use std::{path::PathBuf, thread};

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
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod actions;
mod errors;
mod frontend;
mod kobo_config;
mod screenshot;

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
        .merge(actions::routes())
        .merge(frontend::routes())
        .merge(kobo_config::routes())
        .merge(screenshot::routes())
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
