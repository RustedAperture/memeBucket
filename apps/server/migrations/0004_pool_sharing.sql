-- Add share_token to pools
ALTER TABLE pools ADD COLUMN share_token TEXT;

-- Create unique index for share_token
CREATE UNIQUE INDEX idx_pools_share_token ON pools(share_token) WHERE share_token IS NOT NULL;

-- Create subscriptions table
CREATE TABLE pool_subscriptions (
    subscriber_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subscriber_user_id, pool_id)
);

CREATE INDEX idx_pool_subscriptions_pool ON pool_subscriptions(pool_id);
