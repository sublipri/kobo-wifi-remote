use crate::{
    actions::{
        arbitrary::{InputManager, InputMsg, InputMsgWrapper, InputSender},
        ActionManager, ActionMsg,
    },
    config::Config,
    fbink::FbInkWrapper,
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
use tokio::sync::{mpsc, oneshot};
use tower::Layer;
use tower_http::{
    normalize_path::NormalizePathLayer, set_header::response::SetResponseHeaderLayer,
};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pub tx: mpsc::Sender<ActionMsg>,
    pub fbink: FbInkWrapper,
    pub config: Arc<Mutex<Config>>,
    arbitrary_tx: Arc<tokio::sync::Mutex<Option<InputSender>>>,
}

impl AppState {
    pub fn config(&self) -> MutexGuard<'_, Config> {
        self.config.lock().expect("Failed to lock Config")
    }

    pub async fn send_input_msg(&self, msg: InputMsg) -> Result<()> {
        let arbitrary_tx = self.arbitrary_tx.lock().await;
        let Some(ref tx) = *arbitrary_tx else {
            warn!("Tried to send {msg} when InputManager isn't running");
            return Ok(());
        };
        let (resp, rx) = oneshot::channel();
        let msg = InputMsgWrapper { msg, resp };
        tx.send(msg).await.unwrap();
        rx.await?
    }

    pub async fn start_arbitrary_input(&self) -> Result<()> {
        // Restart the InputManager if it's already running so that config changes take effect
        // and to help minimize the impact of any bugs.
        let fbink = self.fbink.try_inner()?.clone();
        let _ = self.send_input_msg(InputMsg::Shutdown).await;
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ActionMsg::GetRecording {
                path_segment: "next-page".to_string(),
                rotation: None,
                resp,
            })
            .await?;
        if let Ok(template) = rx.await? {
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            let config = self.config().clone();
            match InputManager::new(template.clone(), fbink, config, rx) {
                Ok(mut input_manager) => {
                    thread::spawn(move || input_manager.manage());
                    let mut arbitrary_tx = self.arbitrary_tx.lock().await;
                    *arbitrary_tx = Some(tx);
                }
                Err(e) => error!("Failed to start arbitrary input manager. {e}"),
            };
        } else {
            info!("No recording to use as template. Not running arbitrary input manager.");
        }
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn serve(config: &Config) -> Result<()> {
    let (tx, rx) = mpsc::channel(32);
    let fbink = FbInkWrapper::new(config);
    init(config, fbink.clone())?;
    let mut manager = ActionManager::from_path(
        config.action_file(),
        config.recordings_file(),
        fbink.clone(),
        rx,
    )
    .context("Failed to start ActionManager")?;
    let state = AppState {
        tx,
        fbink: fbink.clone(),
        config: Arc::new(Mutex::new(config.clone())),
        arbitrary_tx: Arc::new(tokio::sync::Mutex::new(None)),
    };
    thread::spawn(move || manager.manage());
    let app = Router::new()
        .merge(crate::config::routes())
        .merge(crate::actions::routes())
        .merge(crate::frontend::routes())
        .merge(crate::kobo_config::routes())
        .merge(crate::screenshot::routes())
        .merge(crate::logging::routes())
        .merge(crate::management::routes())
        .merge(crate::actions::arbitrary::routes())
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
