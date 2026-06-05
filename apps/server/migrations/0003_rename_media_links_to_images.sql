-- Rename the table
ALTER TABLE media_links RENAME TO images;

-- Rename column in send_history
ALTER TABLE send_history RENAME COLUMN media_link_id TO image_id;

-- Rename indexes for images
DROP INDEX IF EXISTS idx_media_links_owner_pool;
CREATE INDEX idx_images_owner_pool ON images(owner_user_id, pool_id);
