# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:bookworm as builder

WORKDIR /build

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock ./
COPY matrix-jukebox ./matrix-jukebox
COPY matrix-rtc ./matrix-rtc
COPY patches ./patches
COPY vendor ./vendor

RUN cargo build --release --frozen --package matrix-jukebox

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox

COPY config.example.yaml /app/config.yaml

ENV CONFIG_PATH=/app/config.yaml

ENTRYPOINT ["/app/matrix-jukebox"]
