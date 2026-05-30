FROM rust:bookworm as builder

WORKDIR /build

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock ./
COPY matrix-jukebox ./matrix-jukebox
COPY matrix-rtc ./matrix-rtc
COPY patches ./patches
COPY vendor ./vendor

RUN cargo build --release --frozen --package matrix-jukebox && \
    mkdir -p /build/data

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && \
    curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux \
         -o /usr/local/bin/yt-dlp && \
    chmod +x /usr/local/bin/yt-dlp && \
    rm -rf /var/lib/apt/lists/*

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl unzip \
    && curl -fsSL https://deno.land/install.sh | DENO_INSTALL=/usr/local sh \
    && apt-get purge -y curl unzip && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 65532 nonroot \
    && useradd --uid 65532 --gid 65532 --no-create-home nonroot

WORKDIR /app

COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox
COPY --from=builder /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
COPY --from=builder --chown=65532:65532 /build/data /app/data

COPY config.example.yaml /app/config.yaml

ENV CONFIG_PATH=/app/config.yaml

USER nonroot

ENTRYPOINT ["/app/matrix-jukebox"]
