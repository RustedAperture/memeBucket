ALTER TABLE images ADD COLUMN cdn_url TEXT;
ALTER TABLE images ADD COLUMN cdn_status TEXT NOT NULL DEFAULT 'pending';

CREATE INDEX idx_images_cdn_status ON images(cdn_status);
