use random_media_bot_server::{
    app_state::AppState,
    config::{Config, connect_sqlite_pool},
    router::build_router,
};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let pool = connect_sqlite_pool(&config.database_url).await?;
    let app = build_router(
        AppState::new(pool)
            .with_discord_public_key(config.discord_public_key)
            .with_static_dir(config.static_dir.clone()),
    );
    let listener = TcpListener::bind(config.bind_addr).await?;

    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
