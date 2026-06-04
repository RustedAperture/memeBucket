use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::{
        account::{delete_account, export_account},
        categories::list_categories,
    },
    app_state::AppState,
    auth::discord_oauth::{handle_discord_oauth_callback, start_discord_oauth},
    discord::interactions::handle_interaction,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/discord/start", get(start_discord_oauth))
        .route("/auth/discord/callback", get(handle_discord_oauth_callback))
        .route("/api/categories", get(list_categories))
        .route("/api/account/export", get(export_account))
        .route("/api/account", delete(delete_account))
        .route("/discord/interactions", post(handle_interaction))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
