use crate::{config::Config, server};

use std::fs;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Parser, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    #[serde(skip)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server in the background
    Start,
    /// Stop the server
    Stop,
    /// Start the server in the foreground
    Serve,
    /// Restart the server
    Restart,
    /// Enable the server to run at boot
    Enable {
        #[arg(long, help = "Start the server if it's not running")]
        now: bool,
    },
    /// Disable the server from running at boot
    Disable {
        #[arg(long, help = "Stop the server if it's running")]
        now: bool,
    },
    /// Enable or disable the server, start/stopping it if necessary
    Toggle,
    /// Uninstall wifiremote
    Uninstall,
}

pub fn cli() -> Result<()> {
    let args = Cli::parse();
    let config = Config::default();

    let Some(subcommand) = &args.command else {
        return Ok(());
    };

    match subcommand {
        Commands::Start => {
            start_server(&config)?;
            // Print to stdout so it can be displayed in NickelMenu
            println!("Wi-Fi Remote started");
        }
        Commands::Stop => {
            stop_server(&config)?;
            println!("Wi-Fi Remote stopped");
        }
        Commands::Restart => {
            restart_server(&config)?;
            println!("Wi-Fi Remote restarted")
        }
        Commands::Enable { now } => enable_server(&config, *now)?,
        Commands::Disable { now } => disable_server(&config, *now)?,
        Commands::Toggle => {
            if let Ok(pid) = config.pid() {
                if !is_running(pid) {
                    warn!("PID file contains {pid} but no process running");
                    enable_server(&config, true)?;
                } else {
                    disable_server(&config, true)?;
                }
            } else {
                enable_server(&config, true)?;
            }
        }
        Commands::Uninstall => {
            todo!()
        }
        Commands::Serve => server::serve()?,
    }
    Ok(())
}

fn start_server(config: &Config) -> Result<()> {
    if let Ok(pid) = config.pid() {
        if is_running(pid) {
            return Err(anyhow!("Server already running with PID {pid}"));
        }
        warn!("PID file contains {pid} but no process running");
    }
    spawn_server(config)?;
    Ok(())
}

fn restart_server(config: &Config) -> Result<()> {
    if let Ok(pid) = config.pid() {
        if is_running(pid) {
            kill(pid, Signal::SIGTERM)?;
            fs::remove_file(&config.pid_file).context("Failed to remove PID file")?;
        }
        warn!("PID file contains {pid} but no process running");
    }
    spawn_server(config)?;
    Ok(())
}

fn stop_server(config: &Config) -> Result<()> {
    let pid = config.pid()?;
    info!("Stopping server with PID {pid}");
    kill(pid, Signal::SIGTERM)?;
    fs::remove_file(&config.pid_file).context("Failed to remove PID file")?;
    Ok(())
}

fn enable_server(config: &Config, now: bool) -> Result<()> {
    let symlink = config.udev_link();
    if symlink.exists() {
        info!(
            "Symlink to UDEV rules already exists at {}",
            symlink.display()
        );
    } else {
        std::os::unix::fs::symlink(config.udev_file(), &symlink)?;
        info!("Created symlink to UDEV rules at {}", &symlink.display());
    }
    if !now {
        return Ok(());
    }
    if let Ok(pid) = config.pid() {
        if !is_running(pid) {
            spawn_server(config)?;
        }
    } else {
        spawn_server(config)?;
    }
    println!("Wi-Fi Remote enabled");
    Ok(())
}

fn disable_server(config: &Config, now: bool) -> Result<()> {
    let symlink = config.udev_link();
    if symlink.exists() {
        info!("Removing symlink to UDEV rules at {}", &symlink.display());
        fs::remove_file(symlink)?;
    } else {
        info!(
            "Symlink to UDEV rules doesn't exist at {}",
            &symlink.display()
        );
    }
    if !now {
        return Ok(());
    }
    if let Ok(pid) = config.pid() {
        if is_running(pid) {
            stop_server(config)?;
        }
    }
    println!("Wi-Fi Remote disabled");
    Ok(())
}

fn spawn_server(config: &Config) -> Result<()> {
    let bin = std::env::current_exe().context("Failed to get path of wifiremote binary")?;
    let pid = Command::new(bin)
        .arg("serve")
        .spawn()
        .context("Failed to spawn server")?
        .id();
    info!("Started server with PID {pid}");
    fs::write(&config.pid_file, pid.to_string()).context("Failed to write PID file")?;
    Ok(())
}

fn is_running(pid: Pid) -> bool {
    kill(pid, Signal::SIGCONT).is_ok()
}
