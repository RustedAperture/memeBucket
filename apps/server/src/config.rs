use std::{env, fs, net::SocketAddr, path::PathBuf};
use std::str::FromStr;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub discord_public_key: String,
    pub static_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()?;
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/app.db".to_string());
        let discord_public_key = env::var("DISCORD_PUBLIC_KEY").unwrap_or_default();
        let static_dir = env::var("STATIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("apps/web/out"));

        Ok(Self {
            bind_addr,
            database_url,
            discord_public_key,
            static_dir,
        })
    }
}

pub async fn connect_sqlite_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    ensure_sqlite_parent_dir(database_url)?;
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    Ok(SqlitePool::connect_with(options).await?)
}

fn ensure_sqlite_parent_dir(database_url: &str) -> anyhow::Result<()> {
    let Some(path) = sqlite_file_path(database_url) else {
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
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
    use std::sync::Mutex;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn config_defaults_to_repo_local_sqlite_path() {
        let _cwd_lock = CWD_LOCK.lock().unwrap();
        let old_discord_public_key = std::env::var("DISCORD_PUBLIC_KEY").ok();
        unsafe {
            std::env::remove_var("DISCORD_PUBLIC_KEY");
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

        assert_eq!(config.database_url, "sqlite://data/app.db");
        assert_eq!(config.discord_public_key, "");
    }

    #[tokio::test]
    async fn connect_sqlite_pool_creates_missing_parent_dir() {
        let _cwd_lock = CWD_LOCK.lock().unwrap();
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
