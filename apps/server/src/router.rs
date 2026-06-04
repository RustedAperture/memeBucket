use axum::{routing::{get, post}, Router};

use crate::{
    api::categories::list_categories,
    app_state::AppState,
    auth::discord_oauth::start_discord_oauth,
    discord::interactions::handle_interaction,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/discord/start", get(start_discord_oauth))
        .route("/api/categories", get(list_categories))
        .route("/discord/interactions", post(handle_interaction))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
