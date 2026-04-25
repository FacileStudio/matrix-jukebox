# matrix-jukebox

Matrix bot workspace with experimental MatrixRTC and LiveKit integration.

## What this repo contains

- `matrix-jukebox/`: main bot binary
- `matrix-rtc/`: RTC integration library used by the bot
- `docs/config.example.yaml`: config template

## Prerequisites

- Rust toolchain (stable, with edition 2024 support)
- `cargo`
- Build tools for native dependencies (Linux)

On Debian/Ubuntu, if you hit native build errors, install:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libasound2-dev
```

## Quick setup

1. Clone the repo and enter it.

```bash
git clone <your-fork-or-repo-url>
cd matrix-jukebox
```

2. Create your config file from the example.

```bash
cp docs/config.example.yaml config.yaml
```

3. Edit `config.yaml`:

- `bot.command_prefix`: command prefix (for example `!`)
- `client.server_name`: homeserver URL or server name (for example `https://matrix.example.org` or `example.org`)
- `client.user_name`: login username (not full MXID)
- `client.password`: account password
- `storage_base_dir`: writable directory for local bot data (for example `data`)

4. Run the bot from the workspace root.

```bash
cargo run -p matrix-jukebox
```

## Optional: custom config path

By default, the bot loads `config.yaml` from the current working directory.

To use a different file:

```bash
CONFIG_PATH=docs/config.yaml cargo run -p matrix-jukebox
```

## First-run behavior

On first run, the bot:

- logs in with `client.user_name` / `client.password`
- creates the storage directory (`storage_base_dir`) if missing
- creates local files:
  - `<storage_base_dir>/storage.db`
  - `<storage_base_dir>/session.json`
- bootstraps cross-signing encryption identity

On later runs, it restores session from `session.json`.

## Runtime behavior

Current built-in behavior includes:

- auto-join invited rooms
- reply with `pong!` when a room message contains `!ping`
- attempt to manage MatrixRTC sessions in joined rooms

Music control is available in rooms with an active MatrixRTC session:

- `!play <youtube-url>` downloads audio with `yt-dlp` and queues it into the LiveKit call

Requirements for music playback:

- `yt-dlp` must be installed and on `PATH`
- The URL must resolve to an audio format that rodio can decode, or you may need `ffmpeg` available for yt-dlp fallbacks

## Logging

Default log filter is `matrix_jukebox=debug,warn`.

Override with either variable:

```bash
JUKEBOX_LOG=info cargo run -p matrix-jukebox
# or
RUST_LOG=matrix_jukebox=debug cargo run -p matrix-jukebox
```

## Troubleshooting

- Config parse/open errors:
  - Ensure the file exists and is valid YAML.
  - Ensure you are running from the directory containing `config.yaml`, or set `CONFIG_PATH`.
- Login fails:
  - Verify homeserver, username, and password.
- Native dependency build issues:
  - Install system packages listed in Prerequisites.
- No reply to ping:
  - Confirm the bot account is joined in the room.
  - Send a message containing `!ping`.
