use axum::{
    Router,
    response::IntoResponse,
    routing::{delete, get, post},
};
use std::sync::Arc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

use crate::{
    api::{
        account::{delete_account, export_account, get_profile, logout, update_username},
        gifs::search_gifs,
        images::{create_image, delete_image, list_images, update_image},
        pools::{create_pool, delete_pool, list_pools, rename_pool},
    },
    app_state::AppState,
    auth::discord_oauth::{handle_discord_oauth_callback, start_discord_oauth},
    discord::interactions::handle_interaction,
    static_files::static_fallback,
};

pub fn build_router(state: AppState) -> Router {
    build_router_internal(state, false)
}

pub fn build_router_for_tests(state: AppState) -> Router {
    build_router_internal(state, true)
}

fn build_router_internal(state: AppState, is_test: bool) -> Router {
    let global_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(20)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );
    let strict_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let mut api_routes = Router::new()
        .route("/health", get(health))
        .route("/api/auth/logout", post(logout))
        .route("/api/gifs/search", get(search_gifs))
        .route(
            "/api/pools/{pool_id}/images",
            get(list_images).post(create_image),
        )
        .route(
            "/api/pools/{pool_id}/images/{image_id}",
            delete(delete_image).patch(update_image),
        )
        .route(
            "/api/pools/{pool_id}/images/{image_id}/move",
            post(crate::api::images::move_image),
        )
        .route(
            "/api/pools/{pool_id}",
            delete(delete_pool).patch(rename_pool),
        )
        .route(
            "/api/pools/{pool_id}/share",
            post(crate::api::pools::share_pool),
        )
        .route(
            "/api/pools/{pool_id}/unshare",
            post(crate::api::pools::unshare_pool),
        )
        .route(
            "/api/share/{token}",
            get(crate::api::pools::get_shared_pool),
        )
        .route(
            "/api/share/{token}/subscribe",
            post(crate::api::pools::subscribe_pool),
        )
        .route(
            "/api/pools/{pool_id}/unsubscribe",
            post(crate::api::pools::unsubscribe_pool),
        )
        .route(
            "/api/pools/{pool_id}/whitelist-enabled",
            axum::routing::patch(crate::api::pools::set_whitelist_enabled),
        )
        .route(
            "/api/pools/{pool_id}/whitelist",
            get(crate::api::pools::list_whitelist_users)
                .post(crate::api::pools::add_whitelist_user),
        )
        .route(
            "/api/pools/{pool_id}/whitelist/{username}",
            delete(crate::api::pools::remove_whitelist_user),
        )
        .route("/api/account/export", get(export_account))
        .route("/api/account", get(get_profile).delete(delete_account))
        .route(
            "/api/account/username",
            axum::routing::patch(update_username),
        );

    if !is_test {
        api_routes = api_routes
            .route(
                "/auth/discord/start",
                get(start_discord_oauth).layer(GovernorLayer::new(strict_governor_conf.clone())),
            )
            .route(
                "/auth/discord/callback",
                get(handle_discord_oauth_callback)
                    .layer(GovernorLayer::new(strict_governor_conf.clone())),
            )
            .route(
                "/api/pools",
                get(list_pools)
                    .post(create_pool)
                    .layer(GovernorLayer::new(strict_governor_conf.clone())),
            )
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                csrf_middleware,
            ))
            .layer(GovernorLayer::new(global_governor_conf.clone()));
    } else {
        api_routes = api_routes
            .route("/auth/discord/start", get(start_discord_oauth))
            .route("/auth/discord/callback", get(handle_discord_oauth_callback))
            .route("/api/pools", get(list_pools).post(create_pool));
    }

    Router::new()
        .merge(api_routes)
        .route("/discord/interactions", post(handle_interaction))
        .fallback(static_fallback)
        .with_state(state)
}

async fn csrf_middleware(
    state: axum::extract::State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method();
    if method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::PATCH
        || method == axum::http::Method::DELETE
    {
        use axum::extract::FromRequestParts;
        let (mut parts, body) = req.into_parts();
        match crate::auth::middleware::RequireCsrf::from_request_parts(&mut parts, &state).await {
            Ok(_) => {
                req = axum::extract::Request::from_parts(parts, body);
            }
            Err(status) => return status.into_response(),
        }
    }
    next.run(req).await
}

async fn health() -> &'static str {
    "ok"
}
