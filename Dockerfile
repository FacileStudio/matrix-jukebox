# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:latest as builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY matrix-jukebox ./matrix-jukebox
COPY matrix-rtc ./matrix-rtc
COPY vendor ./vendor

RUN cargo fetch --locked
RUN cargo build --release --locked --package matrix-jukebox --offline

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox

COPY config.example.yaml /app/config.yaml

ENV CONFIG_PATH=/app/config.yaml

ENTRYPOINT ["/app/matrix-jukebox"]
