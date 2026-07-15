# Bluesky media links and Tenor resolution design

## Goal

Extend the existing social-link ingestion path so image and video posts from Bluesky can be added to a bucket, with automatic author/text credit in notes matching X/Twitter. Fix Tenor page links that currently fail because the submitted page URL is rewritten as though it were already a media asset.

## Scope

- Support `bsky.app/profile/{handle}/post/{rkey}` URLs and the equivalent `bsky.social` host.
- Resolve the handle with Bluesky's public `com.atproto.identity.resolveHandle` endpoint, then fetch the post with `app.bsky.feed.getPosts` using the resolved DID.
- Extract the first image's `fullsize` URL or the first video embed's HLS `playlist` URL.
- Format Bluesky notes as `@{handle}: {post text}`, falling back to `@{handle}` for blank text, using the existing X/Twitter formatting convention.
- Extract post hashtags into image tags for both X/Twitter and Bluesky. Strip the leading `#`, preserve the source spelling, and merge them case-insensitively with manually supplied tags without duplicates.
- Convert Bluesky video playlists to animated WebP with ffmpeg, preserving the existing MP4/WebM behavior.
- Keep submitted Tenor page URLs unchanged during the initial fetch. Normalize only discovered Tenor media URLs, including the current `media1.tenor.com/m/...AAAAC...gif` form and equivalent media host variants.
- Apply the behavior consistently to web/API and Discord ingestion paths, including the slow-media loading hint.

## Architecture and data flow

`resolve_image_url` will dispatch recognized platform URLs before generic direct-image and HTML scraping logic:

1. Parse a Bluesky profile/post URL into a handle and record key. Invalid or unrelated paths continue through the generic resolver.
2. Resolve the handle to a DID, then request the post record from the public Bluesky API. The HTTP helper will retain the existing SSRF protections, timeout, redirect limits, and bounded response reads.
3. Deserialize only the response fields needed for author handle, post text, and image/video views. Select the first supported media item; return `UnsupportedContentType` when no supported media is present.
4. Return a typed resolved result containing the media URL, optional notes, and extracted tags. Bluesky tags will prefer structured `app.bsky.richtext.facet#tag` values and fall back to hashtags parsed from post text; X/Twitter tags will be parsed from syndicated post text. The downstream API/Discord code will identify `.mp4`, `.webm`, and `.m3u8` media as video and route all of them through the converter.
5. For HLS URLs, ffmpeg will read the playlist URL directly and write the existing WebP output. Direct MP4/WebM downloads remain supported.

Tenor handling will change in two focused ways:

- Do not call `normalize_tenor_url` on the original submitted page URL before fetching it.
- Continue normalizing candidate media URLs and oEmbed results, rewriting old Tenor path conventions only where appropriate. The supplied Tenor page should resolve through its Open Graph image metadata.

## Error handling and security

Bluesky upstream failures, malformed JSON, invalid redirects, and missing required fields map to the existing `FetchFailed` or `UnsupportedContentType` user errors. Unsupported embed types do not fall through to arbitrary page scraping. URLs returned by Bluesky still pass the existing HTTP/media validation before being accepted, and all requests use the existing safe-host and bounded-fetch rules.

No privacy-policy update is required: this adds public Bluesky metadata/media fetching and fixes existing Tenor page resolution; it does not add user accounts, analytics, or new user data collection.

## Test-first verification

Implement in red/green/refactor cycles:

- URL parser tests for valid Bluesky profile/post URLs, query strings, trailing segments, and invalid hosts/paths.
- Mock-server tests for handle resolution and post fetching, covering image posts, video posts, missing media, malformed payloads, upstream errors, and notes formatting.
- Hashtag extraction tests for X/Twitter text and Bluesky facets/text fallback, including case-insensitive deduplication with user-supplied tags.
- Tenor regression tests proving a page URL is fetched as HTML and its `og:image`/`twitter:image` media candidate is selected and normalized.
- Video-converter tests for HLS input selection while retaining MP4/WebM behavior.
- API/Discord path tests for `.m3u8` conversion and web slow-media detection for Bluesky URLs.
- Run the focused Rust tests after each cycle, then the complete server test suite and web checks.

## Documentation

Add one feature/fix entry to both `changelog.md` and `apps/web/public/changelog.json`. Do not hand-edit `apps/web/app/changelog/page.tsx`. The version remains `0.2.7` unless implementation reveals a project convention requiring a bump.
