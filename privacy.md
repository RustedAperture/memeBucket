# Privacy Policy

This Privacy Policy explains what memeBucket collects and stores when you use the Discord app and web dashboard.

## Data We Collect

memeBucket stores only the information needed to provide its media bucket features:

- **Account identity:** an internal user ID and an HMAC-SHA256 user key derived from your Discord user ID. memeBucket does not intentionally persist raw Discord user IDs.
- **Discord profile display data:** display name and avatar URL returned by Discord OAuth, used to show your account in the dashboard.
- **memeBucket username:** the username you choose inside memeBucket, used for sharing and whitelist features.
- **Sessions and security tokens:** session records, CSRF token hashes, OAuth state cookies, and expiration timestamps used to keep the web dashboard signed in and protected.
- **Buckets and images:** bucket names, image or GIF URLs, optional image notes, creation timestamps, and related internal IDs.
- **Sharing data:** share tokens, bucket subscription records, subscriber counts, whitelist settings, and whitelist membership.
- **Command usage history:** records of random image sends, including bucket name, selected URL, visibility setting, and timestamp.

## How We Use Data

memeBucket uses this data to:

- authenticate you with Discord;
- show and manage your buckets in the web dashboard;
- support Discord commands such as sending a random image from a bucket;
- provide sharing, subscription, and whitelist features;
- maintain sessions, CSRF protection, rate limiting, and basic service security;
- export or delete account data when requested.

## Sharing and Visibility

Your private buckets are intended to be visible only to you. If you create a share link or allow subscriptions, other users with access may be able to see bucket names, image URLs, previews, notes, your memeBucket username, and subscriber-related information. If whitelist protection is enabled, access is limited to whitelisted memeBucket users, but you should still treat shared media as visible to the users you allow.

When you send an image through Discord, Discord and the recipients of that message can see the image URL or rendered media according to the message context and visibility option you chose.

## Third Parties

memeBucket relies on Discord for OAuth, application commands, user profile data, and message delivery. It also uses the Klipy API for the GIF search feature, meaning your search queries are sent to Klipy. When you add a raw video file (like an MP4) to your bucket, memeBucket may temporarily process that file and automatically upload it to ImgBB (api.imgbb.com) to convert it into a hosted GIF. Image and GIF URLs may point to third-party hosts; loading or viewing those URLs may contact the third-party host. Those services are governed by their own terms and privacy policies.

memeBucket also supports **Telegram Login** as an authentication provider. When you sign in or link your account with Telegram, the Telegram Login Widget sends your Telegram user ID, first name, username (if set), and profile photo URL to our server. This data is stored to identify your account and display your profile in the dashboard.

## Retention

memeBucket keeps account, bucket, image, sharing, subscription, whitelist, and command history data until it is deleted, the account is deleted, or the maintainers remove it for operational reasons. Session records expire and may be removed as part of normal maintenance.

## Export and Deletion

When signed in, you can export your owned buckets and image URLs from the account area of the dashboard. You can also request account deletion. Deleting an account removes the user record and data that is linked through database cascade rules, including owned buckets, images, sessions, subscriptions, and related records.

## Security

memeBucket uses a keyed hash for Discord user identity instead of intentionally storing raw Discord user IDs. The web dashboard uses secure session cookies, CSRF protection for state-changing requests, and rate limiting on selected routes. No system can guarantee perfect security, but the service is designed to limit unnecessary data collection.

## Changes

This Privacy Policy may be updated as memeBucket changes. Continued use of memeBucket after changes are posted means you accept the updated policy.

## Contact

Questions or deletion/export concerns can be sent through the project repository or the maintainer contact listed in the README.
