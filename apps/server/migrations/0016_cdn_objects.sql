CREATE TABLE IF NOT EXISTS cdn_objects (
    content_hash TEXT PRIMARY KEY NOT NULL,
    cdn_url      TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
