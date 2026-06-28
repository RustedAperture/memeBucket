use std::str::FromStr;
use std::{env, fs, net::SocketAddr, path::PathBuf};

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub session_secret: String,
    pub discord_application_id: String,
    pub discord_bot_token: String,
    pub discord_public_key: String,
    pub klipy_api_key: Option<String>,
    pub imgbb_api_key: Option<String>,
    pub static_dir: PathBuf,
    pub telegram_bot_token: Option<String>,
    pub telegram_bot_username: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()?;
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/app.db".to_string());
        let discord_application_id = env::var("DISCORD_APPLICATION_ID").unwrap_or_default();
        let discord_bot_token = env::var("DISCORD_BOT_TOKEN").unwrap_or_default();
        let discord_public_key = env::var("DISCORD_PUBLIC_KEY").unwrap_or_default();
        let session_secret = required_env("SESSION_SECRET")?;
        let klipy_api_key = env::var("KLIPY_API_KEY").ok().filter(|v| !v.is_empty());
        let imgbb_api_key = env::var("IMGBB_API_KEY").ok().filter(|v| !v.is_empty());
        let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|v| !v.is_empty());
        let telegram_bot_username = env::var("TELEGRAM_BOT_USERNAME")
            .ok()
            .filter(|v| !v.is_empty());
        let static_dir = env::var("STATIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("apps/web/out"));

        Ok(Self {
            bind_addr,
            database_url,
            session_secret,
            discord_application_id,
            discord_bot_token,
            discord_public_key,
            klipy_api_key,
            imgbb_api_key,
            static_dir,
            telegram_bot_token,
            telegram_bot_username,
        })
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

pub async fn connect_sqlite_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    ensure_sqlite_parent_dir(database_url)?;
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
    Ok(SqlitePool::connect_with(options).await?)
}

fn ensure_sqlite_parent_dir(database_url: &str) -> anyhow::Result<()> {
    let Some(path) = sqlite_file_path(database_url) else {
        return Ok(());
    };

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}

fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
    let url_path = database_url.strip_prefix("sqlite:")?;
    if url_path == ":memory:" {
        return None;
    }

    let path = url_path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(url_path)
        .strip_prefix("//")
        .unwrap_or(url_path);

    if path.is_empty() {
        return None;
    }

    Some(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::{Config, connect_sqlite_pool};
    use std::fs;
    use tokio::sync::Mutex;

    static CWD_LOCK: Mutex<()> = Mutex::const_new(());

    #[tokio::test]
    async fn config_defaults_to_repo_local_sqlite_path() {
        let _cwd_lock = CWD_LOCK.lock().await;
        let old_discord_public_key = std::env::var("DISCORD_PUBLIC_KEY").ok();
        let old_session_secret = std::env::var("SESSION_SECRET").ok();
        unsafe {
            std::env::remove_var("DISCORD_PUBLIC_KEY");
            std::env::set_var("SESSION_SECRET", "test-session-secret");
        }

        let config = Config::from_env().unwrap();

        match old_discord_public_key {
            Some(value) => unsafe {
                std::env::set_var("DISCORD_PUBLIC_KEY", value);
            },
            None => unsafe {
                std::env::remove_var("DISCORD_PUBLIC_KEY");
            },
        }
        match old_session_secret {
            Some(value) => unsafe {
                std::env::set_var("SESSION_SECRET", value);
            },
            None => unsafe {
                std::env::remove_var("SESSION_SECRET");
            },
        }

        assert_eq!(config.database_url, "sqlite://data/app.db");
        assert_eq!(config.discord_public_key, "");
        assert_eq!(config.session_secret, "test-session-secret");
    }

    #[tokio::test]
    async fn connect_sqlite_pool_creates_missing_parent_dir() {
        let _cwd_lock = CWD_LOCK.lock().await;
        let old_cwd = std::env::current_dir().unwrap();
        let root = std::env::temp_dir().join(format!("random-media-bot-{}", uuid::Uuid::new_v4()));
        let db_path = root.join("data").join("app.db");

        fs::create_dir_all(&root).unwrap();
        std::env::set_current_dir(&root).unwrap();

        assert!(!db_path.parent().unwrap().exists());

        let pool = connect_sqlite_pool("sqlite://data/app.db").await.unwrap();

        assert!(db_path.parent().unwrap().exists());
        assert!(db_path.exists());
        drop(pool);

        std::env::set_current_dir(old_cwd).unwrap();
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
    }
}
