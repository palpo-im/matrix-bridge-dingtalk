# Matrix Bridge DingTalk

A Matrix <-> DingTalk bridge written in Rust.

[中文文档](README_CN.md)

Maintainer: `Palpo Team`

## Status

Usable baseline implemented. Core bidirectional text bridge, admin provisioning API, and dead-letter replay/cleanup are now available.

## Implemented

- Matrix Appservice transaction handling
- Matrix -> DingTalk text forwarding (by persisted room mapping)
- Matrix reply/edit/redaction handling (with policy switches)
- Matrix bot auto-join on invite (`m.room.member` invite)
- DingTalk Stream mode (`/v1.0/im/bot/messages/get`) -> Matrix text forwarding
- Per-conversation DingTalk webhook routing (token or full webhook URL)
- Dedup via `processed_events`
- Message mapping persistence via `message_mappings`
- Dead-letter recording, listing, replay, and cleanup
- Admin API: `status`, `mappings`, `bridge`, `unbridge`, `dead-letters/*`
- CLI management commands: `status`, `mappings`, `replay`, `dead-letter-cleanup`

## DingTalk Mode Notes

- DingTalk has two common robot modes:
  - Group custom webhook robot: outbound webhook send.
  - Enterprise app chatbot: stream/event + session webhook.
- This project uses Stream mode as the primary inbound path and keeps callback as compatibility fallback.

## Quick Start

1. Copy config and registration template:

```bash
cp config/config.example.yaml config.yaml
mkdir -p appservices
cp appservices/dingtalk-registration.example.yaml appservices/dingtalk-registration.yaml
```

The bridge loads appservice registration from `<config_dir>/appservices/dingtalk-registration.yaml`.

2. Edit at least:
- `bridge.domain`
- `bridge.homeserver_url`
- `database.uri` (or `database.url`)
- `appservices/dingtalk-registration.yaml: id` (or `bridge_id`)
- `appservices/dingtalk-registration.yaml: as_token` (or `appservice_token`)
- `appservices/dingtalk-registration.yaml: hs_token` (or `homeserver_token`)
- `stream.client_id`
- `stream.client_secret`

3. Optional env overrides:
- `DINGTALK_WEBHOOK_URL`
- `DINGTALK_ACCESS_TOKEN`
- `DINGTALK_SECRET`
- `DINGTALK_CLIENT_ID`
- `DINGTALK_CLIENT_SECRET`
- `DINGTALK_STREAM_OPENAPI_HOST`
- `DINGTALK_STREAM_KEEP_ALIVE_IDLE_SECS`
- `DINGTALK_STREAM_RECONNECT_INTERVAL_SECS`
- `DINGTALK_STREAM_AUTO_RECONNECT`
- `DINGTALK_STREAM_ENABLED`
- `DINGTALK_CALLBACK_TOKEN`
- `MATRIX_BRIDGE_DINGTALK_PROVISIONING_*_TOKEN`

4. Run:

```bash
cargo run --release
```

## Binding DingTalk Rooms to Matrix

There are two ways to bind/link DingTalk conversations to Matrix rooms:

### Method 1: Matrix Room Commands

You can send commands directly in any Matrix room where the bridge bot is present:

- `!dingtalk bridge <dingtalk_conversation_id>` - Link this Matrix room to a DingTalk conversation
- `!dingtalk unbridge` - Remove the bridge from this room
- `!dingtalk help` - Show available commands

**Example:**
```
!dingtalk bridge "yourconversationid"
```

The bot will respond with a curl command you can use to complete the bridging via the HTTP API.

### Method 2: HTTP Provisioning API

Base URL: `http://<bind_address>:<port>/admin`

#### Create a Bridge (Link Matrix Room to DingTalk Conversation)

```bash
curl -X POST http://localhost:9006/admin/bridge \
  -H "Authorization: Bearer YOUR_WRITE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "matrix_room_id": "!abc123:yourdomain.com",
    "dingtalk_conversation_id": "yourconversationid",
    "dingtalk_conversation_name": "Optional Conversation Name"
  }'
```

**Response:**
```json
{
  "status": "bridged",
  "mapping": {
    "matrix_room_id": "!abc123:yourdomain.com",
    "dingtalk_conversation_id": "yourconversationid"
  }
}
```

#### Remove a Bridge

```bash
curl -X POST http://localhost:9006/admin/unbridge \
  -H "Authorization: Bearer YOUR_WRITE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "matrix_room_id": "!abc123:yourdomain.com"
  }'
```

**Response:**
```json
{
  "status": "unbridged",
  "matrix_room_id": "!abc123:yourdomain.com"
}
```

#### List All Bridges

```bash
curl -X GET "http://localhost:9006/admin/mappings?limit=100&offset=0" \
  -H "Authorization: Bearer YOUR_READ_TOKEN"
```

#### Get Bridge Status

```bash
curl -X GET http://localhost:9006/admin/status \
  -H "Authorization: Bearer YOUR_READ_TOKEN"
```

### Finding Your DingTalk Conversation ID

The DingTalk conversation ID can be found in the DingTalk Stream mode events. When a message is sent from a DingTalk group, the bridge will log it in debug output:

```
[DEBUG] DingTalk event received: conversation=...
```

You can also check the bridge logs when messages arrive from DingTalk to see the conversation ID.

## Admin API

Base URL: `http://<bind_address>:<port>/admin`

All endpoints require Bearer token authentication (except when tokens are not configured).

- `GET /status` - Get bridge status and statistics
- `GET /mappings?limit=100&offset=0` - List all room mappings
- `POST /bridge` - Create a new bridge (see details above)
- `POST /unbridge` - Remove a bridge (see details above)
- `GET /dead-letters?status=pending&limit=100` - List dead-letter events
- `POST /dead-letters/<id>/replay` - Replay a specific dead-letter event
- `POST /dead-letters/replay` - Batch replay dead-letters by status
- `POST /dead-letters/cleanup` - Clean up old dead-letter events

## Current Limits

- Focuses on text path first.
- Rich media/event types are not fully bridged yet.
- Callback compatibility mode currently validates token only (no AES decrypt path yet).

## License

Apache-2.0
