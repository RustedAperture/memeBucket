use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::{
        account::{delete_account, export_account},
        pools::{create_pool, delete_pool, list_pools},
        images::{create_image, delete_image, list_images},
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
            "/api/pools",
            get(list_pools).post(create_pool),
        )
        .route(
            "/api/pools/{pool_id}/images",
            get(list_images).post(create_image),
        )
        .route(
            "/api/pools/{pool_id}/images/{image_id}",
            delete(delete_image),
        )
        .route("/api/pools/{pool_id}", delete(delete_pool))
        .route("/api/account/export", get(export_account))
        .route("/api/account", delete(delete_account))
        .route("/discord/interactions", post(handle_interaction))
        .fallback(static_fallback)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
