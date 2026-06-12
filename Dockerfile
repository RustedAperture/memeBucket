FROM node:24-alpine AS web-build
WORKDIR /repo/apps/web
COPY apps/web/package*.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm ci
COPY apps/web ./
RUN --mount=type=cache,target=/repo/apps/web/.next/cache \
    npm run build

FROM rust:1-bookworm AS server-build
WORKDIR /repo
COPY Cargo.toml Cargo.lock ./
COPY apps/server ./apps/server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/repo/target \
    cargo build --release -p ezgif-server && \
    cp /repo/target/release/ezgif-server /repo/ezgif-server-binary

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*
COPY --from=server-build /repo/ezgif-server-binary /app/ezgif-server
COPY --from=web-build /repo/apps/web/out /app/web
ENV STATIC_DIR=/app/web
ENV BIND_ADDR=0.0.0.0:8080
VOLUME ["/app/data"]
EXPOSE 8080
CMD ["/app/ezgif-server"]
