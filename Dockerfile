# Build stage
FROM rust:latest as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY matrix-jukebox ./matrix-jukebox
COPY matrix-rtc ./matrix-rtc
COPY vendor ./vendor

# Build the binary
RUN cargo build --release --package matrix-jukebox

# Runtime stage
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox

# Provide a default config in the image; can be overridden by volume mount.
COPY docs/config.example.yaml /app/config.yaml

# Default config location - override with volume mount
ENV CONFIG_PATH=/app/config.yaml

RUN if [ ! -f "$CONFIG_PATH" ]; then \
    echo "No config file found at $CONFIG_PATH, creating default config" && \
    echo "bot:" > $CONFIG_PATH && \
    echo "  command_prefix: \"!\"" >> $CONFIG_PATH && \
    echo "client:" >> $CONFIG_PATH && \
    echo "storage_base_dir: \"/app/data\"" >> $CONFIG_PATH; \
    else \
    echo "Config file found at $CONFIG_PATH, using existing config"; \
    fi
    

ENTRYPOINT ["/app/matrix-jukebox"]
