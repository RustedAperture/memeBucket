# B2 Content Deduplication Design

**Date:** 2026-06-28
**Scope:** Server — `apps/server`

## Problem

The storage service currently keys B2 objects by `sha256(source_url)`. Two different URLs pointing to the same image produce two separate B2 objects. This wastes storage and CDN bandwidth, especially for popular memes uploaded by multiple users from different Discord CDN links.

## Goals

- Deduplicate B2 objects **globally across all users** for new uploads going forward
- Exact duplicates only (byte-for-byte identical after conversion)
- No backfill of existing objects
- No changes to caller signatures

## Non-Goals

- Perceptual / near-duplicate detection
- Retroactive deduplication of existing B2 objects
- Per-user deduplication limits

---

## Design

### 1. Data Layer

New table added via a SQL migration:

```sql
CREATE TABLE cdn_objects (
    content_hash TEXT PRIMARY KEY NOT NULL,
    cdn_url      TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
)
```

`content_hash` is the BLAKE3 hex digest of the **final converted bytes** (post-WebP/format conversion). It doubles as the B2 object key (`{content_hash}.{ext}`), replacing the current `sha256(source_url)` key. The primary key serves as the dedup index with no extra index needed.

### 2. Upload Flow

Both `upload_from_url` and `upload_bytes` in `StorageService` follow this new flow:

1. Download bytes from source URL (for `upload_from_url`)
2. Convert to WebP / target format (same as today)
3. Compute `content_hash = blake3::hash(&final_bytes)` as hex string
4. Query `SELECT cdn_url FROM cdn_objects WHERE content_hash = ?`
   - **Hit:** return existing CDN URL immediately — no B2 upload, no wasted work
   - **Miss:** continue to step 5
5. Upload to B2 using `{content_hash}.{ext}` as the object key
6. `INSERT OR IGNORE INTO cdn_objects (content_hash, cdn_url) VALUES (?, ?)`
7. Return CDN URL

### 3. Race Condition Handling

Two concurrent uploads of the same image will both miss the cache, both upload to B2 (same key — B2 put is idempotent), and both attempt the insert. `INSERT OR IGNORE` makes the second insert a silent no-op. Both requests return the same CDN URL.

### 4. Error Handling

A DB failure during the dedup lookup (step 4) is non-fatal: log a warning and fall through to the normal upload path. The user's request succeeds; we may store a duplicate, but we never fail because of a dedup query error.

### 5. `StorageService` Changes

- Add `pool: SqlitePool` field
- `new()` gains a `pool: SqlitePool` parameter
- `upload_from_url` and `upload_bytes` implement the new flow above
- Replace `sha2`/`Sha256` with `blake3`

**Call sites** (`main.rs` / `app_state.rs`): one-line change each to pass the already-available pool into `StorageService::new()`. No other callers change.

### 6. Dependencies

| Change | Reason |
|--------|--------|
| Add `blake3` | Content hashing |
| Remove `sha2` | Only used in `storage.rs` for URL-based key; replaced by BLAKE3 |

---

## Migration

New file: `apps/server/migrations/{next_number}_create_cdn_objects.sql`

```sql
CREATE TABLE IF NOT EXISTS cdn_objects (
    content_hash TEXT PRIMARY KEY NOT NULL,
    cdn_url      TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
```

No changes to existing tables or migrations.

---

## Testing

- **Unit:** `upload_from_url` called twice with the same source returns the same CDN URL and only uploads to B2 once (mock the store and pool)
- **Unit:** DB lookup failure falls through to upload without error
- **Unit:** `upload_bytes` deduplicates correctly
- **Integration:** Two different source URLs whose bytes produce the same post-conversion hash resolve to the same `cdn_url`
