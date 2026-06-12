use ezgif_server::{
    app_state::AppState,
    config::{Config, connect_sqlite_pool},
    discord::commands::command_definitions,
    router::build_router,
};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let pool = connect_sqlite_pool(&config.database_url).await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migration failed");
    tracing::info!("Database migrations complete.");

    // Register Discord slash commands if credentials are configured
    if !config.discord_application_id.is_empty() && !config.discord_bot_token.is_empty() {
        register_discord_commands(&config.discord_application_id, &config.discord_bot_token).await;
    }

    let app = build_router(
        AppState::new(pool)
            .with_session_secret(config.session_secret)
            .with_discord_public_key(config.discord_public_key)
            .with_static_dir(config.static_dir.clone())
            .with_klipy_api_key(config.klipy_api_key)
            .with_imgbb_api_key(config.imgbb_api_key),
    );
    let listener = TcpListener::bind(config.bind_addr).await?;

    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn register_discord_commands(application_id: &str, bot_token: &str) {
    let commands = command_definitions();
    let client = reqwest::Client::new();
    let url = format!("https://discord.com/api/v10/applications/{application_id}/commands");

    for command in &commands {
        let name = command["name"].as_str().unwrap_or("unknown");
        match client
            .post(&url)
            .header("Authorization", format!("Bot {bot_token}"))
            .json(command)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("registered Discord command: {name}");
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!("failed to register command {name}: {status} {body}");
            }
            Err(err) => {
                tracing::warn!("error registering command {name}: {err}");
            }
        }
    }
}
