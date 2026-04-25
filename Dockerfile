# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:latest as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY matrix-jukebox ./matrix-jukebox
COPY matrix-rtc ./matrix-rtc
COPY vendor ./vendor

# Make cargo more resilient to transient network issues.
ENV CARGO_NET_RETRY=10
ENV CARGO_HTTP_TIMEOUT=120
ENV CARGO_HTTP_MULTIPLEXING=false

# Fetch dependencies with host networking to avoid container DNS issues,
# then build offline for determinism.
RUN --network=host cargo fetch --locked
RUN cargo build --release --locked --package matrix-jukebox --offline

# Runtime stage
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox

# Provide a default config in the image; can be overridden by volume mount.
COPY config.example.yaml /app/config.yaml

# Default config location - override with volume mount
ENV CONFIG_PATH=/app/config.yaml

ENTRYPOINT ["/app/matrix-jukebox"]
