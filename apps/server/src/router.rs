use axum::{
    routing::{get, post},
    Router,
};

use crate::{app_state::AppState, discord::interactions::handle_interaction};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/discord/interactions", post(handle_interaction))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
