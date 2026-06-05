# ezGif

Discord user app for personal random image/GIF categories.

## Architecture

- `apps/server`: Rust Axum server, Discord interactions endpoint, OAuth/session routes, SQLite persistence.
- `apps/web`: Next.js static dashboard using shadcn/ui.
- Production: one Docker container, one Rust process, static web assets served by Rust.

## Local Server

```bash
cargo test
cargo run -p ezgif-server
```

## Web Build

```bash
cd apps/web
npm install
npm run build
```

## Docker

```bash
docker build -t ezgif .
docker run --env-file .env -p 8080:8080 -v "$PWD/data:/app/data" ezgif
```

## Discord Endpoint

Configure the Discord Interactions Endpoint URL to:

```text
https://your-cloudflare-tunnel-domain.example/discord/interactions
```

## Privacy

The app does not persist raw Discord user IDs. It stores an HMAC-SHA256 user key derived from the Discord user ID and `APP_USER_KEY_SECRET`.
