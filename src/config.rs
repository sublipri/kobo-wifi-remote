use std::path::{Path, PathBuf};

use anyhow::Result;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub udev_dir: PathBuf,
    pub user_dir: PathBuf,
    pub port: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "/opt/wifiremote/data".into(),
            udev_dir: "/etc/udev/rules.d".into(),
            user_dir: "/mnt/onboard/.adds/wifiremote".into(),
            port: 80,
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
