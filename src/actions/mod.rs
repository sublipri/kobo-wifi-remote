pub use self::action::*;
use crate::{errors::AppError, server::AppState};
use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use tokio::sync::oneshot;
use tracing::debug;

mod action;
pub mod arbitrary;
mod input;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Now that we're not restricted by the BusyBox HTTP server we'll switch to an
        // /actions/:path_segment endpoint, but keep /left and /right for compatibility with KoboPageTurner
        // and anything that users integrated with the original version
        .route("/actions", get(get_actions))
        .route("/actions", post(record_action))
        .route("/actions/:path_segment", get(play_action_handler))
        .route("/actions/:path_segment", delete(delete_action))
        .route("/actions/:path_segment", put(update_action))
        .route("/left", get(prev_page))
        .route("/right", get(next_page))
}

async fn next_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    play_action("next-page".into(), &state).await
}

async fn prev_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    play_action("prev-page".into(), &state).await
}

async fn get_actions(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::List { resp: tx }).await?;
    let actions = rx.await?;
    Ok(Json(actions))
}

async fn play_action_handler(
    State(state): State<AppState>,
    AxumPath(path_segment): AxumPath<String>,
) -> Result<impl IntoResponse, AppError> {
    play_action(path_segment, &state).await
}

async fn play_action(
    path_segment: String,
    state: &AppState,
) -> Result<impl IntoResponse, AppError> {
    let start = Utc::now();
    debug!("Received request to play action {path_segment}");
    let (tx, rx) = oneshot::channel();
    let msg = ActionMsg::Play {
        path_segment: path_segment.clone(),
        resp: tx,
    };
    state.tx.send(msg).await?;
    rx.await??;
    debug!("Successfully played action");
    Ok(Json(PlayActionResponse {
        path_segment,
        time_taken: Utc::now() - start,
    }))
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlayActionResponse {
    path_segment: String,
    #[serde_as(as = "DurationMilliSeconds<i64>")]
    time_taken: Duration,
}

async fn delete_action(
    State(state): State<AppState>,
    AxumPath(path_segment): AxumPath<String>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Received request to delete action {path_segment}");
    let (tx, rx) = oneshot::channel();
    let msg = ActionMsg::Delete {
        path_segment,
        resp: tx,
    };
    state.tx.send(msg).await?;
    rx.await??;
    debug!("Successfully delete action");
    Ok(())
}

async fn update_action(
    State(state): State<AppState>,
    AxumPath(path_segment): AxumPath<String>,
    Json(opts): Json<ActionOptions>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Received request to update action {path_segment}");
    let (tx, rx) = oneshot::channel();
    let msg = ActionMsg::Update {
        path_segment,
        opts,
        resp: tx,
    };
    state.tx.send(msg).await?;
    rx.await??;
    debug!("Successfully updated action");
    Ok(())
}

async fn record_action(
    State(state): State<AppState>,
    Json(opts): Json<RecordActionOptions>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Received request to record action: {:#?}", &opts);
    let (tx, rx) = oneshot::channel();
    state.tx.send(ActionMsg::Record { opts, resp: tx }).await?;
    let response = rx.await??;
    debug!("Successfully recorded action");
    Ok(Json(response))
}
