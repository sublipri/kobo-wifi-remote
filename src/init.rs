use crate::config::{Config, FbInkOptions};
use crate::fbink::FbInkWrapper;
use crate::kobo_config::KoboConfigFile;

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufRead, BufReader, LineWriter, Write},
    path::PathBuf,
    process::Command,
    sync::Arc,
    thread::{self, sleep},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use fbink_rs::{state::SunxiForceRotation, FbInk};
use tracing::{debug, error, trace, warn};

pub fn init(config: &Config, fbink: FbInkWrapper) -> Result<()> {
    let fbink = fbink.try_inner();
    if let Ok(fbink) = fbink {
        set_sunxi_rota(&config.user.fbink, fbink);
    }
    merge_files(config.file_list(), config.file_list().with_extension("new"))
        .context("Failed to merge file list")?;
    merge_files(config.dir_list(), config.dir_list().with_extension("new"))
        .context("Failed to merge dir list")?;

    if !config.action_file().exists() && !Config::is_dev_mode() {
        first_run(config, fbink.cloned());
    }
    Ok(())
}

fn first_run(config: &Config, fbink: Result<Arc<FbInk>>) {
    if let Err(e) = set_force_wifi_on() {
        error!("Failed to set ForceWifiOn=true. {e}");
    }
    if let Err(e) = install_koreader_plugin() {
        error!("Failed to install KOReader plugin. {e}");
    }
    let port = config.app.port;
    let Ok(fbink) = fbink else { return };
    thread::spawn(move || {
        if let Err(e) = display_ip_address(port, fbink) {
            error!("Failed to display IP Address. {e}")
        }
    });
}

pub fn set_sunxi_rota(opts: &FbInkOptions, fbink: &FbInk) {
    let state = fbink.state();
    if state.is_sunxi && std::env::var("FBINK_FORCE_ROTA").is_err() {
        let mut force_rota = opts.sunxi_force_rota;
        // TODO: provide a user-friendly way to download fbdamage and patch on-animator.sh ala
        // NanoClock
        if force_rota == SunxiForceRotation::Workbuf && !state.sunxi_has_fbdamage {
            warn!("sunxi_force_rota set to Workbuf but fbdamage isn't loaded. Using Gyro");
            force_rota = SunxiForceRotation::Gyro;
        }
        debug!("Setting sunxi_force_rota to {force_rota}");
        if let Err(e) = fbink.sunxi_ntx_enforce_rota(force_rota) {
            error!("Failed to set sunxi_force_rota. {e}");
        }
    }
}

fn merge_files(old_files: PathBuf, new_files: PathBuf) -> Result<()> {
    if old_files.exists() && new_files.exists() {
        let (old, new) = (old_files.display(), new_files.display());
        debug!("Merging {new} with {old}");
        let mut existing = HashSet::new();
        for line in BufReader::new(File::open(&old_files)?)
            .lines()
            .map_while(Result::ok)
        {
            existing.insert(line);
        }
        let file = File::options().append(true).open(&old_files)?;
        let mut file = LineWriter::new(file);
        for line in BufReader::new(File::open(&new_files)?)
            .lines()
            .map_while(Result::ok)
        {
            if !existing.contains(&line) {
                file.write_all(&line.into_bytes())?;
            }
        }
        fs::remove_file(&new_files)?;
    } else if new_files.exists() {
        let (old, new) = (old_files.display(), new_files.display());
        debug!("Renaming {new} to {old}");
        fs::rename(&new_files, &old_files)?;
    }
    Ok(())
}

fn set_force_wifi_on() -> Result<()> {
    let mut kobo_config = KoboConfigFile::open(Default::default())?;
    if kobo_config.set_value("DeveloperSettings", "ForceWifiOn", Some("true"))? {
        kobo_config.write()?;
    }
    Ok(())
}

fn install_koreader_plugin() -> Result<()> {
    let koreader_path = PathBuf::from("/mnt/onboard/.adds/koreader/plugins");
    if koreader_path.exists() {
        let wfr_path = koreader_path.join("wifiremote.koplugin");
        if !wfr_path.exists() {
            let p = wfr_path.display();
            debug!("KOReader installation detected. Writing plugin files to {p}");
            fs::create_dir(&wfr_path).with_context(|| format!("Failed to create {p}"))?;
            let main = wfr_path.join("main.lua");
            let meta = wfr_path.join("_meta.lua");
            fs::write(&main, include_str!("../wifiremote.koplugin/main.lua"))
                .with_context(|| format!("Failed to write {}", &main.display()))?;
            fs::write(&meta, include_str!("../wifiremote.koplugin/_meta.lua"))
                .with_context(|| format!("Failed to write {}", &meta.display()))?;
        }
    }
    Ok(())
}

fn display_ip_address(port: u32, fbink: Arc<FbInk>) -> Result<()> {
    let mut msg = format!("\nWi-Fi Remote {} initialized.\n\n ", Config::version());
    msg.push_str("Visit this address to setup your device:\n\n");
    let mut addr = wait_for_ip_address()?;
    if port != 80 {
        addr.push_str(&format!(":{}", port))
    }
    msg.push_str(&format!("http://{addr}\n "));
    sleep(Duration::from_millis(2000));
    fbink.print(&msg).context("Failed to display IP address")?;
    Ok(())
}

fn wait_for_ip_address() -> Result<String> {
    debug!("Waiting for IP Address");
    let max_attempts = 600;
    let retry_after = Duration::from_millis(1000);
    let mut attempts = 0;
    loop {
        let output = Command::new("/sbin/ifconfig")
            .output()
            .context("Failed to run ifconfig")?;
        let output =
            String::from_utf8(output.stdout).context("Failed to decode output of ifconfig")?;
        let output = output.trim();
        trace!("{}", output);

        let start = match output.find("inet addr:") {
            Some(i) => i + 10,
            None => {
                sleep(retry_after);
                attempts += 1;
                if attempts > max_attempts {
                    return Err(anyhow!(
                        "Failed to obtain IP address after {attempts} attempts"
                    ));
                }
                continue;
            }
        };

        let end = output[start..].find(char::is_whitespace).unwrap() + start;
        let ip_addr = output[start..end].to_string();
        return Ok(ip_addr);
    }
}
