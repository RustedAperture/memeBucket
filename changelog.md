# Changelog

## v0.1.3 - Jun 14, 2026

### Added

- Added Library search for saved GIFs and images across accessible pools, with filters for tags, pool, favorites, and random-enabled state.
- Added image metadata fields for title, tags, favorite status, random weight, and notes.
- Added metadata editing from image details and bulk editing for selected images.
- Added Klipy metadata suggestions so saved GIFs can start with a title and suggested tags.
- Added a Library card to the dashboard.

### Changed

- Improved random image selection with per-image weights and stronger recent-repeat avoidance.
- Renamed the global saved-media search surface to Library to distinguish it from searching Klipy for new GIFs.
- Expanded access checks and tests for library search across owned, subscribed, public, private, and whitelisted pools.

## v0.1.2 - Jun 11, 2026

### Added

- Added an "Add to Pool" Discord message context menu command to save images directly from messages into an "Added from Discord" pool.
- Added the ability to rename image pools.

### Changed

- Migrated the web dashboard's sidebar layout to use standard Shadcn UI components.
- Consolidated pool settings (rename, delete, unsubscribe) into a clean Settings modal.

## v0.1.1 - Jun 11, 2026

### Added

- Added homepage buttons for Ko-fi support and the GitHub repository.
- Added a footer theme selector with Light, Dark, and Auto modes.
- Added drag-and-drop support and a modal dropdown for moving images between pools.
- Added a GIF search feature powered by the Klipy API, accessible directly from the pool image form.

### Fixed

- Fixed theme selector styling so only one mode appears selected at a time.
- Fixed an issue in GIF search where "Load more" would append duplicate results.

### Changed

- Improved the GIF search layout by using a masonry-style columns layout to better preserve image aspect ratios.

## v0.1.0 - Jun 10, 2026

### Added

- Initial ezGif Discord user app and web dashboard.
- Discord OAuth sign-in and session-backed account management.
- Personal media pools for organizing image and GIF URLs.
- Discord commands for creating pools, adding images, listing pools, opening the dashboard, and sending a random image from a selected pool.
- Web dashboard for managing pools, images, notes, and account username.
- Pool sharing with share links, subscriptions, subscriber counts, and optional whitelist access.
- Account export endpoint for owned pools and image URLs.
- Account deletion endpoint for removing account-linked data.
- Terms of Service, Privacy Policy, Changelog, and GPLv3 License pages.

### Privacy and Security

- Discord user identity is stored as an HMAC-SHA256 user key rather than a raw Discord user ID.
- State-changing dashboard requests use CSRF protection.
- Selected routes use rate limiting.
