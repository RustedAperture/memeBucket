# Project Instructions

When modifying features in this project, keep the following in mind:

## Changelog
Whenever you add, change, or fix a feature, you must update the changelog in **two** places to ensure both the repository and the live website are up-to-date:
1. `/changelog.md` (root directory)
2. `/apps/web/public/changelog.json` (web app — `/apps/web/app/changelog/page.tsx` renders from this file and should not be hand-edited for new entries)

If updating the changelog involves bumping the version number, you must also bump the version strings in `apps/server/Cargo.toml` and `apps/web/package.json` to match.

## Privacy Policy
Whenever you add a new feature that involves data collection or relies on a third-party service (like Klipy, Discord, etc.), you must update the Privacy Policy in **two** places:
1. `/privacy.md` (root directory)
2. `/apps/web/app/privacy/page.tsx` (web app)
