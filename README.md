# Matrix Bridge DingTalk

A Matrix <-> DingTalk bridge written in Rust.

[中文文档](README_CN.md)

Maintainer: `Palpo Team`  
Contact: `chris@acroidea.com`

## Status

🚧 **Under Active Development** 🚧

This project is currently in early development stage. See [_todos.md](_todos.md) for progress.

## Overview

- Rust-only implementation
- Matrix appservice + DingTalk webhook bridge core
- HTTP endpoints for health/status/metrics and provisioning
- Database backends: PostgreSQL, SQLite, and MySQL (feature-gated)
- Docker support (coming soon)

## Features

### Implemented
- ✅ Project structure and configuration system
- ✅ Configuration file parsing (YAML)
- ✅ Environment variable overrides
- ✅ Utils modules (error handling, logging, formatting)
- ✅ Basic project skeleton

### In Progress
- 🚧 Database layer (PostgreSQL/SQLite/MySQL)
- 🚧 DingTalk webhook client
- 🚧 Matrix appservice integration
- 🚧 Message bridge core logic

### Planned
- 📋 DingTalk message types support (text, markdown, link, actionCard, feedCard)
- 📋 User presence synchronization
- 📋 Media file bridging
- 📋 Web UI for provisioning
- 📋 Docker deployment

## Quick Start

### Prerequisites

- Rust toolchain (edition 2024)
- A Matrix homeserver configured for appservices
- DingTalk webhook access tokens
- Database: PostgreSQL, SQLite, or MySQL

### Configuration

1. Copy the sample configuration:

```bash
cp config/config.sample.yaml config.yaml
```

2. Edit `config.yaml` and set:
   - `bridge.domain`: Your Matrix server domain
   - `bridge.homeserver_url`: Your Matrix homeserver URL
   - `auth.webhooks`: DingTalk webhook tokens mapping
   - `database.url`: Database connection string

3. Generate registration file (coming soon):

```bash
cargo run -- --generate-registration
```

### Build and Run

```bash
# Build
cargo build --release

# Run
cargo run --release
```

## Configuration

### DingTalk Webhook Setup

1. In DingTalk group chat, go to Group Settings → Smart Group Assistant
2. Add a custom robot
3. Configure security settings:
   - **Keyword**: Messages must contain the keyword
   - **Sign**: Messages must have valid signature (recommended)
   - **IP Whitelist**: Only allow specific IPs
4. Copy the webhook URL and extract the `access_token`
5. Add to `config.yaml`:

```yaml
auth:
  webhooks:
    "chat_id": "access_token"
  security:
    type: "sign"
    secret: "your_secret_here"
```

### Matrix Appservice Setup

Add to your Matrix homeserver configuration:

```yaml
app_service_config_files:
  - /path/to/dingtalk-registration.yaml
```

## Architecture

```
src/
├── config/          # Configuration parsing and validation
├── db/              # Database layer (models, migrations)
├── dingtalk/        # DingTalk webhook client
├── matrix/          # Matrix appservice client
├── bridge/          # Bridge core logic
│   ├── message_flow.rs    # Message transformation
│   ├── user_sync.rs       # User synchronization
│   └── provisioning.rs    # Room provisioning
├── web/             # HTTP server endpoints
├── utils/           # Utilities (error, logging, formatting)
└── parsers/         # Message format parsers
```

## Supported Message Types

### DingTalk → Matrix
- Text → m.text
- Markdown → m.text (with formatting)
- Link → m.text (with preview)
- ActionCard → m.text (with buttons)
- FeedCard → m.text (multiple links)

### Matrix → DingTalk
- m.text → Text or Markdown
- m.image → Markdown (with image URL)
- m.video → Markdown (with video URL)
- m.file → Markdown (with file URL)

## Development

### Running Tests

```bash
cargo test
```

### Code Style

```bash
cargo fmt
cargo clippy
```

## License

Apache-2.0

## Contributing

Contributions are welcome! Please read the contributing guidelines first.

## Acknowledgments

- [matrix-bridge-discord](https://github.com/palpo-im/matrix-bridge-discord) - Reference implementation
- [matrix-bot-sdk](https://github.com/palpo-im/matrix-bot-sdk) - Matrix Rust SDK
- [DingTalk Open Platform](https://open.dingtalk.com/) - DingTalk API documentation
