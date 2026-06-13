ALTER TABLE images ADD COLUMN title TEXT;
ALTER TABLE images ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
ALTER TABLE images ADD COLUMN random_weight INTEGER NOT NULL DEFAULT 1;

CREATE TABLE image_tags (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    image_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL,
    name_folded TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (owner_user_id, image_id) REFERENCES images(owner_user_id, id) ON DELETE CASCADE,
    UNIQUE(owner_user_id, image_id, name_folded)
);

CREATE UNIQUE INDEX idx_images_owner_id ON images(owner_user_id, id);
CREATE INDEX idx_images_owner_favorite ON images(owner_user_id, favorite);
CREATE INDEX idx_images_owner_weight ON images(owner_user_id, random_weight);
CREATE INDEX idx_image_tags_owner_name ON image_tags(owner_user_id, name_folded);
CREATE INDEX idx_image_tags_image ON image_tags(image_id);
CREATE INDEX idx_image_tags_image_position ON image_tags(image_id, position);
CREATE INDEX idx_send_history_owner_pool_sent_at ON send_history(owner_user_id, pool_id, sent_at);
