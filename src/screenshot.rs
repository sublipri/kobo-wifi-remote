use crate::{errors::AppError, server::AppState};

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Local;
use fbink_rs::ImageOutputFormat;

pub fn routes() -> Router<AppState> {
    Router::new().route("/screenshot", get(screenshot))
}

async fn screenshot(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    state.fbink.reinit()?;
    let bytes = state.fbink.screenshot(ImageOutputFormat::Png)?;
    let timestamp = Local::now().format("%Y%m%d-%H%M-%S");
    let filename = format!("{} {timestamp}.png", state.fbink.state().device_id);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    let value = HeaderValue::from_str(&format!("inline; filename=\"{}\"", filename))?;
    headers.insert(header::CONTENT_DISPOSITION, value);
    Ok((headers, bytes))
}
