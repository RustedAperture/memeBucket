ALTER TABLE pools ADD COLUMN whitelist_enabled BOOLEAN NOT NULL DEFAULT 0;

CREATE TABLE pool_whitelists (
    pool_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    PRIMARY KEY (pool_id, user_id),
    FOREIGN KEY (pool_id) REFERENCES pools(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
