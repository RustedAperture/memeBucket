# Changelog

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
