# Changelog

## v0.1.8 - Jun 26, 2026

### Added

- Added the **memeBucket Picker** desktop app (Tauri): a lightweight overlay that lives in the macOS menu bar or Windows/Linux system tray and never occupies the dock or taskbar.
- Added a global hotkey (`⌘ Shift M` / `Ctrl Shift M`) to summon and dismiss the Picker from any app.
- Added a `/picker` web page powering the overlay: masonry image grid with live search, bucket filter, keyboard navigation (↑↓ for linear, ←→ for column-aware jumping), and Enter-to-paste.
- Added a `/download` page that fetches the latest GitHub release via the API, auto-detects the visitor's OS, and shows a direct download button with file size alongside a Getting Started guide.
- Added a Download card to the dashboard and a Download link to the navigation bar.
- Added a GitHub Actions release workflow (`release-desktop.yml`) that builds and publishes signed-free binaries for macOS Apple Silicon, macOS Intel, Windows, and Linux on `desktop-v*` tag pushes.

### Changed

- Improved the Picker selection highlight to use a solid full-opacity ring with an offset gap for much higher contrast.
- Removed the item count badge from the Picker header to reduce visual noise.

## v0.1.7 - Jun 26, 2026

### Added

- Added a dedicated, thread-safe in-memory caching layer using `moka` in the backend to cache read-heavy database lookups (bucket lists, names, subscriptions, whitelists, and image lists) with precise write-time invalidation hooks.
- Added declarative request payload validation at the API boundary using the `validator` crate and a custom Axum extractor `ValidatedJson`.
- Added non-blocking asynchronous external process execution in `video_converter.rs` utilizing `tokio::process::Command` to prevent blocking worker threads during `ffmpeg` conversions.
- Added a `delete` method to the `UserRepo` trait and shifted the complex bulk database import/export transaction logic entirely to the repository layer, rendering the service layer fully decoupled and database-agnostic.
- Migrated the global frontend session and authentication state from React Context to a `zustand` store, optimizing component re-renders.

### Changed

- Refactored server startup to bind and run the HTTP listener first, scheduling Discord command registration asynchronously in a background task.
- Refactored `AccountService` to consume repository traits via Dependency Injection instead of holding a concrete `SqlitePool` and executing raw SQL queries.

## v0.1.6 - Jun 25, 2026

### Added

- Added bulk editing features to the bucket view, including **Bulk Delete** and **Bulk Move** with efficient transactional backend endpoints.
- Added **Bulk Add Links** allowing users to paste multiple image/video URLs at once with a step-by-step progress UI and error recovery (pre-filling with only failed URLs for easy retries).
- Added batch actions to **Select All** and **Copy Links** for all selected items.
- Added **Import Data** functionality in the Account Settings menu, allowing users to upload their exported backup JSON files to easily restore or duplicate their buckets and media with all metadata and tags fully preserved.
- Added client-side format validation and server-side duplicate prevention to ensure seamless imports.

### Changed

- Rebranded the Discord system bucket from `"Added from Discord"` to `"Inbox"`, including a database migration to automatically migrate existing records.
- Removed redundant drag count badges from selected cards for a cleaner visual aesthetic.

### Fixed

- Fixed the "Export Data" functionality which was failing due to outdated database table references (`pools`) from the rebranding.
- Updated the export data payload to include all newly added image metadata fields (title, favorite, random weight, tags, notes, and created_at).

## v0.1.5 - Jun 23, 2026

### Added

- Added mobile-specific touch gestures: single tap to copy media URLs to the clipboard, and long press (500ms) to open the detailed image properties modal in both the Bucket and Search views.
- Enabled clicking images on desktop viewports in the Search page to open the properties dialog, matching the Bucket view behavior.

### Changed

- Rebranded the entire application from **ezGif** to **memeBucket**, renaming all references to **pools** to **buckets** throughout the database, backend services, Discord commands, and frontend UI.
- Refactored the Bucket control toolbar to use a responsive two-row layout on mobile viewports, allowing the size slider to remain fully functional while the input box stretches to fill the screen width.
- Configured Sonner toast notifications to dynamically display at the top center of the screen on mobile devices.

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
- System buckets (like Favorites or Inbox) are now automatically hidden when empty.

## v0.1.2 - Jun 11, 2026

### Added

- Added an "Add to Bucket" Discord message context menu command to save images directly from messages into an "Inbox" bucket.
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
