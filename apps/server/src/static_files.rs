use std::path::PathBuf;

pub fn static_dir_from_env_value(value: Option<String>) -> PathBuf {
    value
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("apps/web/out"))
}
