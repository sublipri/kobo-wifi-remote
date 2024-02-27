use crate::config::Config;
use crate::kobo_config::KoboConfigFile;

use std::{process::Command, thread::sleep, time::Duration};

use anyhow::{anyhow, Context, Result};
use fbink_rs::{config::Font, FbInk, FbInkConfig};
use tracing::trace;

pub fn first_run(config: &Config) -> Result<()> {
    let mut kobo_config = KoboConfigFile::open(Default::default())?;
    if kobo_config.set_value("DeveloperSettings", "ForceWifiOn", Some("true"))? {
        kobo_config.write()?;
    }

    let fbink = FbInk::new(FbInkConfig {
        is_centered: true,
        is_halfway: true,
        is_padded: true,
        font: Font::Fatty,
        ..Default::default()
    })?;

    let mut msg = format!("\nWi-Fi Remote {} initialized.\n\n ", Config::version());
    msg.push_str("Visit this address to setup your device:\n\n");
    let mut addr = wait_for_ip_address()?;
    if config.port != 80 {
        addr.push_str(&format!(":{}", config.port))
    }
    msg.push_str(&format!("http://{addr}\n "));
    fbink.print(&msg)?;
    Ok(())
}

pub fn wait_for_ip_address() -> Result<String> {
    let max_attempts = 120;
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
