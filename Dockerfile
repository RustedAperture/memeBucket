FROM node:24-alpine AS web-build
WORKDIR /repo/apps/web
COPY apps/web/package*.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm ci
COPY apps/web ./
RUN --mount=type=cache,target=/repo/apps/web/.next/cache \
    npm run build

FROM rust:1-alpine AS server-build
RUN apk add --no-cache musl-dev gcc
WORKDIR /repo
COPY Cargo.toml Cargo.lock ./
COPY apps/server ./apps/server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/repo/target \
    cargo build --release -p memebucket-server && \
    cp /repo/target/release/memebucket-server /repo/memebucket-server-binary

FROM alpine:latest AS runtime
WORKDIR /app
RUN apk add --no-cache ca-certificates ffmpeg
COPY --from=server-build /repo/memebucket-server-binary /app/memebucket-server
COPY --from=web-build /repo/apps/web/out /app/web
ENV STATIC_DIR=/app/web
ENV BIND_ADDR=0.0.0.0:8080
VOLUME ["/app/data"]
EXPOSE 8080
CMD ["/app/memebucket-server"]
