use crate::{errors::AppError, server::AppState};
pub use syslog::setup_syslog;
use tracing::debug;

use std::{
    io::{BufRead, Write},
    process::Command,
};

use axum::{
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;

mod syslog;

pub fn routes() -> Router<AppState> {
    Router::new().route("/syslog", get(get_log))
}

async fn get_log() -> Result<impl IntoResponse, AppError> {
    debug!("Preparing log file for download");
    let stdout = Command::new("logread").output()?.stdout;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let to_write = stdout
        .lines()
        .flatten()
        .filter(|l| l.contains("wifiremote") || l.contains("FBInk"));

    for line in to_write {
        encoder.write_all(line.as_bytes())?;
        encoder.write_all(&[b'\n'])?;
    }
    let bytes = encoder.finish()?;
    let timestamp = Local::now().format("%s");
    let filename = format!("wifiremote-{timestamp}.log.gz");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-download"),
    );
    let value = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))?;
    headers.insert(header::CONTENT_DISPOSITION, value);
    Ok((headers, bytes))
}
