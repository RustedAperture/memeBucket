-- 1. Add role column to users.
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

-- 2. Capture discord_user_key values before we recreate users.
CREATE TABLE discord_keys_temp AS
    SELECT id AS user_id, discord_user_key, display_name, avatar_url
    FROM users
    WHERE discord_user_key IS NOT NULL;

-- 3. Recreate users without discord_user_key (removes NOT NULL constraint too).
--    With FK enforcement off, DROP TABLE does not cascade to child tables.
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

-- 4. Create user_identities now that users is the final clean table.
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

-- 5. Migrate existing Discord users from the captured keys.
--    On fresh databases discord_keys_temp is empty so this is a no-op.
INSERT INTO user_identities (id, user_id, provider, provider_user_id, display_name, avatar_url)
SELECT
    lower(hex(randomblob(16))),
    user_id,
    'discord',
    discord_user_key,
    display_name,
    avatar_url
FROM discord_keys_temp;

DROP TABLE discord_keys_temp;
