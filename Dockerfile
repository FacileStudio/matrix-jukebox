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
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/matrix-jukebox /app/matrix-jukebox

# Create data directory
RUN mkdir -p /app/data

# Default config location - override with volume mount
ENV CONFIG_PATH=/app/config.yaml

#test if there is a config file, if not create a default one
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
