use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::{
        account::{delete_account, export_account},
        categories::{create_category, delete_category, list_categories},
        media_links::{create_link, delete_link, list_links},
    },
    app_state::AppState,
    auth::discord_oauth::{handle_discord_oauth_callback, start_discord_oauth},
    discord::interactions::handle_interaction,
    static_files::static_fallback,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/discord/start", get(start_discord_oauth))
        .route("/auth/discord/callback", get(handle_discord_oauth_callback))
        .route(
            "/api/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/api/categories/{category_id}/links",
            get(list_links).post(create_link),
        )
        .route(
            "/api/categories/{category_id}/links/{link_id}",
            delete(delete_link),
        )
        .route("/api/categories/{category_id}", delete(delete_category))
        .route("/api/account/export", get(export_account))
        .route("/api/account", delete(delete_account))
        .route("/discord/interactions", post(handle_interaction))
        .fallback(static_fallback)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
