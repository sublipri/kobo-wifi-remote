use crate::actions::arbitrary::InputOptions;
use crate::frontend::index::IndexOptions;
use crate::{errors::AppError, server::AppState};

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use axum::routing::post;
use axum::Json;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use chrono::Duration;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMicroSeconds, DurationMilliSeconds, DurationSeconds};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub app_config_path: PathBuf,
    pub app: AppConfig,
    pub user_config_path: PathBuf,
    pub user: UserConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub udev_dir: PathBuf,
    pub user_dir: PathBuf,
    pub port: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: "/opt/wifiremote/data".into(),
            udev_dir: "/etc/udev/rules.d".into(),
            user_dir: "/mnt/onboard/.adds/wifiremote".into(),
            port: 80,
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let config = Figment::from(Serialized::defaults(Self::default()))
            .merge(Toml::file(path))
            .merge(Env::prefixed("WIFIREMOTE_"))
            .extract()?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UserConfig {
    pub page_turner: PageTurnerOptions,
    pub remote_control: RemoteOptions,
    pub auto_turner: AutoTurnerOptions,
    pub voice_activation: VoiceActivationOptions,
    pub setup: SetupOptions,
    pub custom_action_defaults: CustomActionOptions,
    pub arbitrary_input: InputOptions,
    pub index: IndexOptions,
}

impl UserConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let config = Figment::from(Serialized::defaults(Self::default()))
            .merge(Toml::file(path))
            .merge(Env::prefixed("WIFIREMOTE_"))
            .extract()?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        let app = AppConfig::default();
        let app_config_path = app.user_dir.join("app-config.toml");
        let user_config_path = app.user_dir.join("user-config.toml");
        let user = UserConfig::default();
        Self {
            app,
            user,
            user_config_path,
            app_config_path,
        }
    }
}

impl Config {
    pub fn action_file(&self) -> PathBuf {
        self.app.user_dir.join("actions.toml")
    }
    pub fn recordings_file(&self) -> PathBuf {
        self.app.data_dir.join("recordings.bin")
    }
    pub fn udev_file(&self) -> PathBuf {
        self.app.data_dir.join("udev.rules")
    }
    pub fn udev_link(&self) -> PathBuf {
        self.app.udev_dir.join("98-wifiremote.rules")
    }
    pub fn dir_list(&self) -> PathBuf {
        self.app.data_dir.join("tracked_dirs")
    }
    pub fn file_list(&self) -> PathBuf {
        self.app.data_dir.join("tracked_files")
    }
    pub fn version() -> &'static str {
        clap::crate_version!()
    }
    pub fn is_dev_mode() -> bool {
        std::env::var("WIFIREMOTE_DEV_MODE").is_ok_and(|v| v == "1")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PageTurnerOptions {
    pub next_color: String,
    pub prev_color: String,
    pub enable_arbitrary_input: bool,
    pub prompt_fullscreen: bool,
}

impl Default for PageTurnerOptions {
    fn default() -> Self {
        Self {
            next_color: "#33b249".into(),
            prev_color: "#5783db".into(),
            enable_arbitrary_input: true,
            prompt_fullscreen: false,
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetupOptions {
    pub only_check_touch: bool,
    pub optimize: bool,
    pub use_by_path: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
    #[serde_as(as = "DurationMicroSeconds<i64>")]
    pub syn_gap: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub no_input_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub new_event_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub poll_wait: Duration,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            only_check_touch: true,
            optimize: true,
            use_by_path: false,
            post_playback_delay: Duration::milliseconds(300),
            syn_gap: Duration::microseconds(1),
            no_input_timeout: Duration::milliseconds(5000),
            new_event_timeout: Duration::milliseconds(250),
            poll_wait: Duration::milliseconds(10),
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomActionOptions {
    pub only_check_touch: bool,
    pub optimize: bool,
    pub use_by_path: bool,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub optimize_max_duration: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub post_playback_delay: Duration,
    #[serde_as(as = "DurationMicroSeconds<i64>")]
    pub syn_gap: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub no_input_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub new_event_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    pub poll_wait: Duration,
}

impl Default for CustomActionOptions {
    fn default() -> Self {
        Self {
            only_check_touch: true,
            optimize: false,
            use_by_path: false,
            optimize_max_duration: Duration::milliseconds(1000),
            post_playback_delay: Duration::milliseconds(300),
            syn_gap: Duration::microseconds(1),
            no_input_timeout: Duration::milliseconds(5000),
            new_event_timeout: Duration::milliseconds(4000),
            poll_wait: Duration::milliseconds(10),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteOptions {
    pub color1: String,
    pub color2: String,
    pub enable_arbitrary_input: bool,
    pub prompt_fullscreen: bool,
}

impl Default for RemoteOptions {
    fn default() -> Self {
        Self {
            color1: "#5783db".into(),
            color2: "#33b249".into(),
            enable_arbitrary_input: true,
            prompt_fullscreen: false,
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoTurnerOptions {
    #[serde_as(as = "DurationSeconds<i64>")]
    pub default_delay: Duration,
}

impl Default for AutoTurnerOptions {
    fn default() -> Self {
        Self {
            default_delay: Duration::seconds(120),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VoiceActivationOptions {
    pub language_code: String,
}

impl Default for VoiceActivationOptions {
    fn default() -> Self {
        Self {
            language_code: "en-US".into(),
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config))
        .route("/config/user/toml", post(update_user_toml))
        .route("/config/user/toml", get(get_user_toml))
}

async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(serde_json::to_string(&state.config)?)
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateTomlRequest {
    toml: String,
}

async fn update_user_toml(
    State(state): State<AppState>,
    Json(edited): Json<UpdateTomlRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user: UserConfig = toml::from_str(&edited.toml)?;
    let mut config = state.config();
    fs::write(&config.user_config_path, &edited.toml)
        .context("Failed to write user config file")?;
    config.user = user;
    Ok(())
}

async fn get_user_toml(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let toml = fs::read_to_string(&state.config().user_config_path)
        .context("Failed to read user config file")?;
    Ok(toml)
}
