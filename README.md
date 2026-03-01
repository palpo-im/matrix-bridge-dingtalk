# Matrix Bridge DingTalk

A Matrix <-> DingTalk bridge written in Rust.

[中文文档](README_CN.md)

Maintainer: `Palpo Team`

## Status

Usable baseline implemented. Core bidirectional text bridge, admin provisioning API, and dead-letter replay/cleanup are now available.

## Implemented

- Matrix Appservice transaction handling
- Matrix -> DingTalk text forwarding (by persisted room mapping)
- DingTalk callback -> Matrix text forwarding
- Per-conversation DingTalk webhook routing (token or full webhook URL)
- Dedup via `processed_events`
- Message mapping persistence via `message_mappings`
- Dead-letter recording, listing, replay, and cleanup
- Admin API: `status`, `mappings`, `bridge`, `unbridge`, `dead-letters/*`
- CLI management commands: `status`, `mappings`, `replay`, `dead-letter-cleanup`

## DingTalk Mode Notes

- DingTalk has two common robot modes:
  - Group custom webhook robot: outbound webhook send.
  - Enterprise app chatbot: callback/event + session webhook.
- This project supports webhook-based outbound and callback-based inbound text flow.

## Quick Start

1. Copy config:

```bash
cp config/config.sample.yaml config.yaml
```

2. Edit at least:
- `bridge.domain`
- `bridge.homeserver_url`
- `database.url`
- `registration.bridge_id`
- `registration.appservice_token`
- `registration.homeserver_token`

3. Optional env overrides:
- `DINGTALK_WEBHOOK_URL`
- `DINGTALK_ACCESS_TOKEN`
- `DINGTALK_SECRET`
- `DINGTALK_CALLBACK_TOKEN`
- `MATRIX_BRIDGE_DINGTALK_PROVISIONING_*_TOKEN`

4. Run:

```bash
cargo run --release
```

## Admin API

Base URL: `http://<bind_address>:<port>/admin`

- `GET /status`
- `GET /mappings?limit=100&offset=0`
- `POST /bridge`
- `POST /unbridge`
- `GET /dead-letters?status=pending&limit=100`
- `POST /dead-letters/<id>/replay`
- `POST /dead-letters/replay`
- `POST /dead-letters/cleanup`

## Current Limits

- Focuses on text path first.
- Rich media/event types are not fully bridged yet.

## License

Apache-2.0
