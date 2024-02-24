use std::path::PathBuf;

pub struct ConfigOptions {
    pub data_dir: PathBuf,
    pub udev_dir: PathBuf,
    pub user_dir: PathBuf,
    pub port: u32,
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            data_dir: "/opt/wifiremote/data".into(),
            udev_dir: "/etc/udev/rules.d".into(),
            user_dir: "/mnt/onboard/.adds/wifiremote".into(),
            port: 8000,
        }
    }
}

impl ConfigOptions {
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
}
