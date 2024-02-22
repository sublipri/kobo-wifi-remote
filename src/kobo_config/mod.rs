pub use self::config::{KoboConfigFile, KoboConfigOptions, KoboConfigSetting};
use crate::{errors::AppError, AppState};

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

mod config;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/kobo-config", get(get_settings))
        .route("/kobo-config", post(update_settings))
}

async fn get_settings() -> Result<impl IntoResponse, AppError> {
    let config = KoboConfigFile::open(KoboConfigOptions::default())?;
    Ok(Json(config.get_values()))
}

async fn update_settings(
    Json(settings): Json<Vec<KoboConfigSetting>>,
) -> Result<impl IntoResponse, AppError> {
    let mut config = KoboConfigFile::open(KoboConfigOptions::default())?;
    for s in settings {
        config.set_value(&s.section, &s.key, s.value.as_deref())?
    }
    config.write()?;
    Ok(())
}
