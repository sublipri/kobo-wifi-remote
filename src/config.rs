use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use nix::unistd::Pid;

pub struct Config {
    pub data_dir: PathBuf,
    pub udev_dir: PathBuf,
    pub user_dir: PathBuf,
    pub pid_file: PathBuf,
    pub port: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "/opt/wifiremote/data".into(),
            udev_dir: "/etc/udev/rules.d".into(),
            user_dir: "/mnt/onboard/.adds/wifiremote".into(),
            pid_file: "/var/run/wifiremote.pid".into(),
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
    pub fn pid(&self) -> Result<Pid> {
        let pid = fs::read_to_string(&self.pid_file).context("Failed to read PID file")?;
        let pid: i32 = pid.parse().context("Failed to parse PID")?;
        Ok(Pid::from_raw(pid))
    }
}
