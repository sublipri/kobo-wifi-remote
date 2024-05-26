use crate::server::AppState;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use tracing::{debug, error, warn};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/restart", get(restart_handler))
        // Use /exit for compatibility with KoboPageTurner
        .route("/exit", get(shutdown_handler))
}

async fn restart_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.config().app.allow_remote_restart {
        restart_server();
        (StatusCode::OK, "Restart successful")
    } else {
        warn!("Remote restart attempted but disabled in AppConfig");
        (
            StatusCode::FORBIDDEN,
            "Remote restart disabled in AppConfig",
        )
    }
}

async fn shutdown_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.config().app.allow_remote_shutdown {
        debug!("Received request to shutdown server");
        run_command("stop");
        (StatusCode::OK, "Shutdown successful")
    } else {
        warn!("Remote shutdown attempted but disabled in AppConfig");
        (
            StatusCode::FORBIDDEN,
            "Remote shutdown disabled in AppConfig",
        )
    }
}

pub fn restart_server() {
    debug!("Restarting server");
    run_command("restart");
}

// A proper graceful shutdown/restart would be less hacky but SIGTERM via the CLI should work okay
// for now.
fn run_command(cmd: &'static str) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let Ok(bin_path) = std::env::current_exe() else {
            error!("Failed to get path of wifiremote binary");
            return;
        };
        if std::process::Command::new(bin_path)
            .arg(cmd)
            .spawn()
            .is_err()
        {
            error!("Failed to run {cmd} command");
        }
    });
}
