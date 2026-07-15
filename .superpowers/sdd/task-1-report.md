# Task 1 implementer report: Bluesky/Tenor resolver and social metadata

## Status

Implemented Task 1 in `apps/server/src/services/images.rs`. The implementation is limited to the resolver and metadata layer; API, Discord, and converter consumption of `ResolvedImage.tags` remain Task 2 scope.

## Implementation

- Extended `ResolvedImage` with `tags: Vec<String>` while retaining `url` and `notes` behavior.
- Added strict Bluesky post parsing for both `bsky.app` and `bsky.social` profile/post URLs. Query strings are accepted; malformed paths and extra segments are rejected.
- Added Bluesky handle resolution through `com.atproto.identity.resolveHandle`, then post retrieval through `app.bsky.feed.getPosts` using an `at://{did}/app.bsky.feed.post/{rkey}` URI.
- Added minimal serde response models for author, record, facets, image embeds, and video embeds.
- Selects the first image `fullsize` URL or a video `playlist` URL, validates the selected media through the existing safe fetch helper, and accepts image/video and HLS MIME types.
- Preserved the existing SSRF, DNS-safe-address, timeout, redirect, and bounded metadata-read behavior by using `fetch_success` and `read_limited_text` for Bluesky API requests.
- Formats Bluesky notes with the existing `@handle: text` / `@handle` convention.
- Added social tags: Bluesky prefers structured `app.bsky.richtext.facet#tag` tags and falls back to post-text hashtags; X/Twitter tags come from syndicated text. Tags remove a leading `#`, retain source spelling, and deduplicate ASCII-case-insensitively.
- Stopped normalizing the submitted URL before direct-image/page fetching. Tenor normalization remains limited to discovered oEmbed and HTML metadata candidates, including `media1.tenor.com/m/...AAAAC...gif` and equivalent media hosts.

## TDD evidence

### RED

Command:

```text
cargo test -p memebucket-server services::images
```

After correcting only Rust raw-string delimiters in the new JSON fixtures, the expected missing-feature RED output was:

```text
error[E0425]: cannot find function `resolve_bluesky_post_from_api_urls` in this scope
error[E0609]: no field `tags` on type `TwitterMedia`
error: could not compile `memebucket-server` (lib test) due to 8 previous errors
```

This demonstrated that the new resolver seam and social tag metadata did not exist before implementation.

### GREEN

Command:

```text
cargo test -p memebucket-server services::images -- --test-threads=1
```

Exact summary:

```text
running 42 tests
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out; finished in 4.05s
```

Additional verification:

```text
cargo fmt --check
```

Exit code `0`; no output.

```text
git diff --check
```

Exit code `0`; no output.

## Tests added/covered

- Bluesky URL parsing for `bsky.app`, `bsky.social`, query strings, unrelated hosts, malformed paths, and trailing segments.
- Mock Bluesky handle/post API resolution for image and video embeds.
- Bluesky notes, facet-tag precedence, text-hashtag fallback, missing media, malformed JSON, and upstream error behavior.
- X/Twitter text-hashtag extraction.
- Tenor media-host normalization and HTML Open Graph metadata candidate normalization.

## Files changed

- `apps/server/src/services/images.rs`
- `.superpowers/sdd/task-1-report.md`

## Self-review

- Verified recognized Bluesky links dispatch before generic scraping and unsupported embeds return `UnsupportedContentType` instead of falling through to page scraping.
- Confirmed Bluesky handle and post responses use the existing safe fetch and bounded text-read helpers.
- Confirmed direct submitted URLs are no longer passed through `normalize_tenor_url`; only discovered candidate/oEmbed URLs are normalized.
- Confirmed no API, Discord, or converter changes were made, per Task 1 scope.

## Concerns

- The default parallel focused command currently fails in this sandbox because concurrent Axum mock servers cannot bind loopback sockets. Its exact result was `26 passed; 16 failed`, with each failure panicking on `TcpListener::bind("127.0.0.1:0")` and `Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }`. The unchanged suite passes serially with `--test-threads=1`, so this is an environment resource restriction rather than a resolver assertion failure.
- `ResolvedImage.tags` is produced by this task but intentionally not merged into API/Discord user tags yet; that integration belongs to Task 2.
