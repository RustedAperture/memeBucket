-- 1. Add role column to users
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

-- 2. Create user_identities table
CREATE TABLE user_identities (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK(provider IN ('discord', 'telegram')),
    provider_user_id TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    linked_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_user_id)
);

CREATE INDEX idx_user_identities_user ON user_identities(user_id);

-- 3. Migrate existing Discord users.
--    discord_user_key is the HMAC-derived hex of the real Discord user ID —
--    carry it forward as provider_user_id so auth lookup continues to work.
INSERT INTO user_identities (id, user_id, provider, provider_user_id, display_name, avatar_url)
SELECT
    lower(hex(randomblob(16))),
    id,
    'discord',
    discord_user_key,
    display_name,
    avatar_url
FROM users;

-- 4. Recreate users table without discord_user_key.
--    SQLite requires table recreation to remove a column.
CREATE TABLE users_new (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    username TEXT UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_new (id, display_name, avatar_url, username, role, created_at, updated_at)
SELECT id, display_name, avatar_url, username, role, created_at, updated_at
FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
