FROM node:24-alpine AS web-build
WORKDIR /repo/apps/web
COPY apps/web/package*.json ./
RUN npm ci
COPY apps/web ./
RUN npm run build

FROM rust:1-bookworm AS server-build
WORKDIR /repo
COPY Cargo.toml Cargo.lock ./
COPY apps/server ./apps/server
RUN cargo build --release -p ezgif-server

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=server-build /repo/target/release/ezgif-server /app/ezgif-server
COPY --from=web-build /repo/apps/web/out /app/web
ENV STATIC_DIR=/app/web
ENV BIND_ADDR=0.0.0.0:8080
VOLUME ["/app/data"]
EXPOSE 8080
CMD ["/app/ezgif-server"]
