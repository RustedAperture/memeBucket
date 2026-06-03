PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    discord_user_key TEXT NOT NULL UNIQUE,
    display_name TEXT,
    avatar_url TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf_token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE categories (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    name_folded TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(owner_user_id, name_folded)
);

CREATE TABLE media_links (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    preview_status TEXT NOT NULL DEFAULT 'unchecked',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE send_history (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
    media_link_id TEXT REFERENCES media_links(id) ON DELETE SET NULL,
    category_name TEXT NOT NULL,
    url TEXT NOT NULL,
    response_visibility TEXT NOT NULL,
    sent_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_categories_owner ON categories(owner_user_id);
CREATE INDEX idx_media_links_owner_category ON media_links(owner_user_id, category_id);
CREATE INDEX idx_send_history_owner_sent_at ON send_history(owner_user_id, sent_at);
CREATE INDEX idx_sessions_user_expires ON sessions(user_id, expires_at);
