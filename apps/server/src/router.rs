use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::{
        account::{delete_account, export_account, get_profile, update_username, logout},
        images::{create_image, delete_image, list_images, update_image},
        pools::{create_pool, delete_pool, list_pools},
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
        .route("/api/auth/logout", post(logout))
        .route("/api/pools", get(list_pools).post(create_pool))
        .route(
            "/api/pools/{pool_id}/images",
            get(list_images).post(create_image),
        )
        .route(
            "/api/pools/{pool_id}/images/{image_id}",
            delete(delete_image).patch(update_image),
        )
        .route("/api/pools/{pool_id}", delete(delete_pool))
        .route("/api/pools/{pool_id}/share", post(crate::api::pools::share_pool))
        .route("/api/pools/{pool_id}/unshare", post(crate::api::pools::unshare_pool))
        .route("/api/share/{token}", get(crate::api::pools::get_shared_pool))
        .route("/api/share/{token}/subscribe", post(crate::api::pools::subscribe_pool))
        .route("/api/pools/{pool_id}/unsubscribe", post(crate::api::pools::unsubscribe_pool))
        .route("/api/pools/{pool_id}/whitelist-enabled", axum::routing::patch(crate::api::pools::set_whitelist_enabled))
        .route("/api/pools/{pool_id}/whitelist", get(crate::api::pools::list_whitelist_users).post(crate::api::pools::add_whitelist_user))
        .route("/api/pools/{pool_id}/whitelist/{username}", delete(crate::api::pools::remove_whitelist_user))
        .route("/api/account/export", get(export_account))
        .route("/api/account", get(get_profile).delete(delete_account))
        .route("/api/account/username", axum::routing::patch(update_username))
        .route("/discord/interactions", post(handle_interaction))
        .fallback(static_fallback)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
