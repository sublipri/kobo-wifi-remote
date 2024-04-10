use crate::actions::arbitrary::InputOptions;
use crate::frontend::index::IndexOptions;
use crate::{errors::AppError, server::AppState};
use std::path::{Path, PathBuf};

use anyhow::Result;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use chrono::Duration;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use serde_with::DurationSeconds;

#[serde_with::serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub udev_dir: PathBuf,
    pub user_dir: PathBuf,
    pub port: u32,
    pub prompt_fullscreen: bool,
    #[serde_as(as = "DurationSeconds<i64>")]
    pub auto_turner_delay: Duration,
    pub voice_language_code: String,
    pub arbitrary_input: InputOptions,
    pub index: IndexOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "/opt/wifiremote/data".into(),
            udev_dir: "/etc/udev/rules.d".into(),
            user_dir: "/mnt/onboard/.adds/wifiremote".into(),
            port: 80,
            prompt_fullscreen: false,
            voice_language_code: "en-US".into(),
            arbitrary_input: Default::default(),
            index: Default::default(),
            auto_turner_delay: Duration::seconds(120),
        }
    }
}

impl Config {
    pub fn from_path(path: &Path) -> Result<Self> {
        let config = Figment::from(Serialized::defaults(Self::default()))
            .merge(Toml::file(path))
            .merge(Env::prefixed("WIFIREMOTE_"))
            .extract()?;
        Ok(config)
    }
    pub fn action_file(&self) -> PathBuf {
        self.user_dir.join("actions.toml")
    }
    pub fn recordings_file(&self) -> PathBuf {
        self.data_dir.join("recordings.bin")
    }
    pub fn udev_file(&self) -> PathBuf {
        self.data_dir.join("udev.rules")
    }
    pub fn udev_link(&self) -> PathBuf {
        self.udev_dir.join("98-wifiremote.rules")
    }
    pub fn dir_list(&self) -> PathBuf {
        self.data_dir.join("tracked_dirs")
    }
    pub fn file_list(&self) -> PathBuf {
        self.data_dir.join("tracked_files")
    }
    pub fn version() -> &'static str {
        clap::crate_version!()
    }
    pub fn is_dev_mode() -> bool {
        std::env::var("WIFIREMOTE_DEV_MODE").is_ok_and(|v| v == "1")
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/config", get(get_config))
}

async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(serde_json::to_string(&state.config)?)
}
