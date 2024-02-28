use kobo_wifi_remote::{cli::cli, config::Config, logging::setup_syslog};

use anyhow::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();
    } else if !Config::is_dev_mode() {
        setup_syslog();
    }
    cli()?;
    Ok(())
}
