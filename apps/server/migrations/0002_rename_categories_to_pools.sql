-- Rename the table
ALTER TABLE categories RENAME TO pools;

-- Rename indexes for the table
DROP INDEX IF EXISTS idx_categories_owner;
CREATE INDEX idx_pools_owner ON pools(owner_user_id);

-- Rename columns in media_links
ALTER TABLE media_links RENAME COLUMN category_id TO pool_id;

-- Rename indexes for media_links
DROP INDEX IF EXISTS idx_media_links_owner_category;
CREATE INDEX idx_media_links_owner_pool ON media_links(owner_user_id, pool_id);

-- Rename columns in send_history
ALTER TABLE send_history RENAME COLUMN category_id TO pool_id;
ALTER TABLE send_history RENAME COLUMN category_name TO pool_name;
