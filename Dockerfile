# to be run with `docker build --output type=local,dest=~/.local/bin/

FROM docker.io/library/rust:1-slim AS builder
RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libgtksourceview-3.0-dev

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --all-features && \
    cp target/release/boop-gtk /tmp/boop-gtk

FROM scratch as exporter
COPY --from=builder /tmp/boop-gtk .