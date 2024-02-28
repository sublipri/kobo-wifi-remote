use std::path::PathBuf;

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
    pub fn action_file(&self) -> PathBuf {
        self.data_dir.join("actions.bin")
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
