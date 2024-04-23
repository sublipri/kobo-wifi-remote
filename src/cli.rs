use crate::{config::Config, server};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{env, fs};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use fbink_rs::config::Font;
use fbink_rs::{FbInk, FbInkConfig, ImageOutputFormat};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use slug::slugify;
use tracing::{error, info, warn};

#[derive(Parser, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// Path to the config file
    #[arg(
        long,
        short,
        default_value = "/mnt/onboard/.adds/wifiremote/wifiremote.toml"
    )]
    pub config_path: PathBuf,
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
        /// Start the server if it's not running
        #[arg(long)]
        now: bool,
    },
    /// Disable the server from running at boot
    Disable {
        /// Stop the server if it's running
        #[arg(long)]
        now: bool,
    },
    /// Display the status of the server
    Status,
    /// Enable or disable the server, start/stopping it if necessary
    Toggle,
    /// Uninstall wifiremote
    Uninstall {
        /// Print what will be deleted without removing anything
        #[arg(long)]
        dry_run: bool,
    },
    Screenshot {
        /// How long in seconds to wait before taking the screenshot
        #[arg(long, default_value_t = 0)]
        delay: u64,
        /// Display the output path on the e-reader using FBInk
        #[arg(long = "fbink")]
        use_fbink: bool,
    },
    /// Create a config file with the default values
    CreateConfig {
        #[arg(long, short, default_value = "wifiremote.toml")]
        path: PathBuf,
    },
}

pub fn cli() -> Result<()> {
    let args = Cli::parse();

    let config_path = if let Some(path) = env::var_os("WIFIREMOTE_CONFIG_PATH") {
        path.into()
    } else {
        args.config_path
    };
    let config = match Config::from_path(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Error reading config file: {e}. Using defaults");
            Config::default()
        }
    };

    let Some(subcommand) = &args.command else {
        return Ok(());
    };

    match subcommand {
        Commands::Start => {
            start_server()?;
            // Print to stdout so it can be displayed in NickelMenu
            println!("Wi-Fi Remote started");
        }
        Commands::Stop => {
            stop_server()?;
            println!("Wi-Fi Remote stopped");
        }
        Commands::Restart => {
            restart_server()?;
            println!("Wi-Fi Remote restarted")
        }
        Commands::Enable { now } => enable_server(&config, *now)?,
        Commands::Disable { now } => disable_server(&config, *now)?,
        Commands::Status => {
            if config.udev_link().exists() {
                print!("Server is enabled")
            } else {
                print!("Server is disabled")
            };
            if let Some(pid) = get_pid()? {
                println!(" and running with PID {pid}")
            } else {
                println!(" and not running")
            }
        }
        Commands::Toggle => {
            if get_pid()?.is_none() {
                enable_server(&config, true)?;
            } else {
                disable_server(&config, true)?;
            }
        }
        Commands::Uninstall { dry_run } => uninstall(&config, *dry_run, &config_path)?,
        Commands::Serve => server::serve(&config)?,
        Commands::Screenshot { delay, use_fbink } => screenshot(&config, *delay, *use_fbink)?,
        Commands::CreateConfig { path } => {
            let config = Config::default();
            fs::write(path, toml::to_string_pretty(&config)?)?
        }
    }
    Ok(())
}

fn get_pid() -> Result<Option<Pid>> {
    let server_cmd = format!("{} serve", bin_path()?.display());
    let output = Command::new("pgrep")
        .arg("-o")
        .arg("-f")
        .arg(server_cmd)
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    if let Ok(pid) = stdout.trim().parse::<i32>() {
        Ok(Some(Pid::from_raw(pid)))
    } else {
        Ok(None)
    }
}

fn bin_path() -> Result<PathBuf> {
    std::env::current_exe().context("Failed to get path of wifiremote binary")
}

fn spawn_server() -> Result<()> {
    let pid = Command::new(bin_path()?)
        .arg("serve")
        .spawn()
        .context("Failed to spawn server")?
        .id();
    info!("Started server with PID {pid}");
    Ok(())
}

fn start_server() -> Result<()> {
    if let Some(pid) = get_pid()? {
        Err(anyhow!("Wi-Fi Remote already running with PID {pid}"))
    } else {
        spawn_server()?;
        Ok(())
    }
}

fn restart_server() -> Result<()> {
    if let Some(pid) = get_pid()? {
        kill(pid, Signal::SIGTERM)?;
    }
    spawn_server()?;
    Ok(())
}

fn stop_server() -> Result<()> {
    if let Some(pid) = get_pid()? {
        info!("Stopping server with PID {pid}");
        kill(pid, Signal::SIGTERM)?;
        Ok(())
    } else {
        Err(anyhow!("Wi-Fi Remote isn't running"))
    }
}

fn enable_server(config: &Config, now: bool) -> Result<()> {
    let symlink = config.udev_link();
    let s = symlink.display();
    if symlink.exists() {
        info!("Symlink to UDEV rules already exists at {s}",);
    } else {
        std::os::unix::fs::symlink(config.udev_file(), &symlink)?;
        info!("Created symlink to UDEV rules at {s}");
    }
    if now && get_pid()?.is_none() {
        spawn_server()?;
    }
    println!("Wi-Fi Remote enabled");
    Ok(())
}

fn disable_server(config: &Config, now: bool) -> Result<()> {
    let symlink = config.udev_link();
    let s = symlink.display();
    if symlink.exists() {
        info!("Removing symlink to UDEV rules at {s}");
        fs::remove_file(symlink)?;
    } else {
        info!("Symlink to UDEV rules doesn't exist at {s}",);
    }
    if now && get_pid()?.is_some() {
        stop_server()?;
    }
    println!("Wi-Fi Remote disabled");
    Ok(())
}

fn uninstall(config: &Config, dry_run: bool, config_path: &Path) -> Result<()> {
    info!("Beginning uninstallation");
    if !dry_run {
        disable_server(config, true)?;
    }
    let dir_list = fs::read_to_string(config.dir_list())?;
    let file_list = fs::read_to_string(config.file_list())?;
    // Delete tracked files
    for f in file_list.lines() {
        delete_if_exists(&PathBuf::from(f), dry_run)?;
    }
    delete_if_exists(&config.action_file(), dry_run)?;
    delete_if_exists(&config.action_file().with_extension("toml.bkp"), dry_run)?;
    delete_if_exists(&config.recordings_file(), dry_run)?;
    delete_if_exists(config_path, dry_run)?;
    delete_if_exists(&config.recordings_file().with_extension("bin.bkp"), dry_run)?;
    cleanup_old_version(config, dry_run)?;
    // Delete empty tracked directories
    for d in dir_list.lines() {
        delete_if_exists(&PathBuf::from(d), dry_run)?;
    }
    Ok(())
}

fn cleanup_old_version(config: &Config, dry_run: bool) -> Result<()> {
    // Remove any files/directories that were dynamically generated by wifiremote 0.1.x
    let events_dir = config.data_dir.join("events");
    if events_dir.exists() {
        for entry in fs::read_dir(events_dir)? {
            delete_if_exists(&entry?.path(), dry_run)?;
        }
    }
    let http_dir = PathBuf::from("/opt/wifiremote/http");
    if http_dir.exists() {
        for entry in fs::read_dir(http_dir)? {
            delete_if_exists(&entry?.path(), dry_run)?;
        }
    }
    Ok(())
}

fn delete_if_exists(path: &Path, dry_run: bool) -> Result<()> {
    let p = path.display();
    if !path.exists() {
        if dry_run {
            println!("{p} doesn't exist in the filesystem. Skipping");
        } else {
            info!("{p} doesn't exist in the filesystem. Skipping");
        };
        return Ok(());
    }

    if path.is_file() {
        if dry_run {
            println!("Deleting file {p}");
        } else {
            info!("Deleting file {p}");
            fs::remove_file(path).with_context(|| format!("failed to delete {p}"))?;
        }
    } else if path.is_dir() && dry_run {
        println!("Directory {p} will be deleted if empty");
    } else if path.is_dir() && path.read_dir()?.next().is_none() {
        info!("Deleting empty directory {p}");
        fs::remove_dir(path).with_context(|| format!("failed to delete {p}"))?;
    } else {
        warn!("{p} is not a file or empty directory. Skipping");
    }
    Ok(())
}

fn screenshot(config: &Config, delay: u64, use_fbink: bool) -> Result<()> {
    let fbink = FbInk::new(FbInkConfig {
        is_centered: true,
        is_halfway: true,
        is_padded: true,
        font: Font::Fatty,
        to_syslog: true,
        ..Default::default()
    })?;
    if delay > 0 {
        sleep(Duration::from_secs(delay));
    }
    let bytes = fbink.screenshot(ImageOutputFormat::Png)?;
    let timestamp = Local::now().format("%Y%m%d-%H%M-%S");
    let filename = format!("{}-{timestamp}.png", slugify(fbink.state().device_id));
    let out_dir = config.user_dir.join("screenshots");
    fs::create_dir_all(&out_dir)?;
    let out_file = out_dir.join(&filename);
    fs::write(out_file, bytes)?;
    let display = out_dir.display().to_string();
    let relpath = display.trim_start_matches("/mnt/onboard/");
    if use_fbink {
        fbink.print(&format!("Saved {filename} to\n{relpath}"))?;
    }
    println!("Saved {filename} to {relpath}");
    Ok(())
}
