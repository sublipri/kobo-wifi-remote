use kobo_wifi_remote::server;

use anyhow::Result;
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    debug!("Starting wifiremote");

    server::serve()?;
    Ok(())
}
