# Changelog

## v0.1.4 - Jun 22, 2026

### Added

- Added an optional `target` parameter to the `/mb` slash command to send the GIF directly to a specific user.
- Added a "Reply with GIF" right-click message context menu command to instantly send a GIF directed at the author of the selected message.

### Fixed

- Fixed an issue causing right-click "Reply with GIF" modals to time out due to slow database reads by enabling SQLite WAL mode.
- Fixed a Discord API `400 Bad Request` error preventing the modal from opening by removing unsupported select menus.
- Added a friendly error message listing available buckets if an invalid bucket name is entered in the modal.

### Changed

- Updated Discord integration to embed GIFs so URLs are hidden instead of using zero-width spaces.
- Restored the user's specific Discord profile accent color to embeds sent from the bot.
- Updated dependencies to patch security vulnerabilities.

## v0.1.3 - Jun 14, 2026

### Added

- Added Library search for saved GIFs and images across accessible buckets, with filters for tags, bucket, favorites, and random-enabled state.
- Added image metadata fields for title, tags, favorite status, random weight, and notes.
- Added metadata editing from image details and bulk editing for selected images.
- Added Klipy metadata suggestions so saved GIFs can start with a title and suggested tags.
- Added a Library card to the dashboard.
- Added a "Disable usage" toggle to image buckets.
- Auto-injected a read-only "Favorites" bucket containing all starred media.
- Expanded the "Paste URL" field to natively double as a Klipy GIF search query.
- Added a star when hovering over an image to easily toggle favorite status.

### Changed

- Improved random image selection with per-image weights and stronger recent-repeat avoidance.
- Renamed the global saved-media search surface to Library to distinguish it from searching Klipy for new GIFs.
- Expanded access checks and tests for library search across owned, subscribed, public, private, and whitelisted buckets.
- Refactored bucket view and search pages to share a unified responsive layout.
- System buckets (like Favorites or Added from Discord) are now automatically hidden when empty.

## v0.1.2 - Jun 11, 2026

### Added

- Added an "Add to Bucket" Discord message context menu command to save images directly from messages into an "Added from Discord" bucket.
- Added the ability to rename image buckets.

### Changed

- Migrated the web dashboard's sidebar layout to use standard Shadcn UI components.
- Consolidated bucket settings (rename, delete, unsubscribe) into a clean Settings modal.

## v0.1.1 - Jun 11, 2026

### Added

- Added homepage buttons for Ko-fi support and the GitHub repository.
- Added a footer theme selector with Light, Dark, and Auto modes.
- Added drag-and-drop support and a modal dropdown for moving images between buckets.
- Added a GIF search feature powered by the Klipy API, accessible directly from the bucket image form.

### Fixed

- Fixed theme selector styling so only one mode appears selected at a time.
- Fixed an issue in GIF search where "Load more" would append duplicate results.

### Changed

- Improved the GIF search layout by using a masonry-style columns layout to better preserve image aspect ratios.

## v0.1.0 - Jun 10, 2026

### Added

- Initial memeBucket Discord user app and web dashboard.
- Discord OAuth sign-in and session-backed account management.
- Personal media buckets for organizing image and GIF URLs.
- Discord commands for creating buckets, adding images, listing buckets, opening the dashboard, and sending a random image from a selected bucket.
- Web dashboard for managing buckets, images, notes, and account username.
- Bucket sharing with share links, subscriptions, subscriber counts, and optional whitelist access.
- Account export endpoint for owned buckets and image URLs.
- Account deletion endpoint for removing account-linked data.
- Terms of Service, Privacy Policy, Changelog, and GPLv3 License pages.

### Privacy and Security

- Discord user identity is stored as an HMAC-SHA256 user key rather than a raw Discord user ID.
- State-changing dashboard requests use CSRF protection.
- Selected routes use rate limiting.
