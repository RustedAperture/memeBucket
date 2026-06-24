-- Rename tables
ALTER TABLE pools RENAME TO buckets;
ALTER TABLE pool_subscriptions RENAME TO bucket_subscriptions;
ALTER TABLE pool_whitelists RENAME TO bucket_whitelists;

-- Rename columns
ALTER TABLE bucket_subscriptions RENAME COLUMN pool_id TO bucket_id;
ALTER TABLE bucket_whitelists RENAME COLUMN pool_id TO bucket_id;
ALTER TABLE images RENAME COLUMN pool_id TO bucket_id;
ALTER TABLE send_history RENAME COLUMN pool_id TO bucket_id;
ALTER TABLE send_history RENAME COLUMN pool_name TO bucket_name;

-- Drop old indexes
DROP INDEX IF EXISTS idx_pools_owner;
DROP INDEX IF EXISTS idx_pools_share_token;
DROP INDEX IF EXISTS idx_pool_subscriptions_pool;
DROP INDEX IF EXISTS idx_images_owner_pool;
DROP INDEX IF EXISTS idx_send_history_owner_pool_sent_at;

-- Create new indexes referencing buckets
CREATE INDEX idx_buckets_owner ON buckets(owner_user_id);
CREATE UNIQUE INDEX idx_buckets_share_token ON buckets(share_token) WHERE share_token IS NOT NULL;
CREATE INDEX idx_bucket_subscriptions_bucket ON bucket_subscriptions(bucket_id);
CREATE INDEX idx_images_owner_bucket ON images(owner_user_id, bucket_id);
CREATE INDEX idx_send_history_owner_bucket_sent_at ON send_history(owner_user_id, bucket_id, sent_at);
