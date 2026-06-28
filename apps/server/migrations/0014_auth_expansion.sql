-- 1. Add role column to users
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

-- 2. Recreate users table without discord_user_key, preserving all data.
--    We do this BEFORE creating user_identities so no FK cascade can fire.
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

-- Keep the old table alive until after we've captured discord_user_key values.
-- We rename users -> users_old so we can still SELECT from it below.
ALTER TABLE users RENAME TO users_old;
ALTER TABLE users_new RENAME TO users;

-- 3. Create user_identities table now that users is the final table.
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

-- 4. Migrate existing Discord users from the preserved old table.
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
FROM users_old;

-- 5. Drop the old table now that migration is complete.
DROP TABLE users_old;
