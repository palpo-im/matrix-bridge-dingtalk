# Matrix 钉钉桥接器

使用 Rust 编写的 Matrix <-> 钉钉桥接器。

维护者: `Palpo Team`

## 当前状态

已具备可用基础能力：双向文本桥接、管理接口、死信重放/清理。

## 已实现能力

- Matrix Appservice transaction 处理
- Matrix -> 钉钉 文本转发（基于持久化 room mapping）
- Matrix reply/edit/redaction 处理（受配置开关控制）
- Matrix 机器人被邀请后自动入房（`m.room.member` invite）
- 钉钉 Stream 模式（`/v1.0/im/bot/messages/get`）-> Matrix 文本转发
- 按会话 webhook 路由发送（支持 token 或完整 webhook URL）
- `processed_events` 去重
- `message_mappings` 映射落库
- dead-letter 记录、查询、重放、清理
- Admin API: `status`、`mappings`、`bridge`、`unbridge`、`dead-letters/*`
- CLI 管理命令: `status`、`mappings`、`replay`、`dead-letter-cleanup`

## 钉钉模式说明

钉钉常见两种机器人模式：

- 群自定义 webhook 机器人：主要用于 webhook 出站发送。
- 企业应用机器人：支持 Stream 事件与会话 webhook。

本项目当前使用 Stream 作为主入站链路，callback 作为兼容回退链路。

## 快速开始

1. 复制主配置和 registration 模板：

```bash
cp config/config.example.yaml config.yaml
mkdir -p appservices
cp appservices/dingtalk-registration.example.yaml appservices/dingtalk-registration.yaml
```

桥接器会从 `<config_dir>/appservices/dingtalk-registration.yaml` 加载 appservice registration。

2. 至少配置以下字段：
- `bridge.domain`
- `bridge.homeserver_url`
- `database.uri`（或 `database.url`）
- `appservices/dingtalk-registration.yaml: id`（或 `bridge_id`）
- `appservices/dingtalk-registration.yaml: as_token`（或 `appservice_token`）
- `appservices/dingtalk-registration.yaml: hs_token`（或 `homeserver_token`）
- `stream.client_id`
- `stream.client_secret`

3. 可选环境变量覆盖：
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

4. 运行：

```bash
cargo run --release
```

## 绑定钉钉群组到 Matrix 房间

有两种方法可以将钉钉会话绑定到 Matrix 房间：

### 方法 1：Matrix 房间命令

你可以在任何桥接机器人所在的 Matrix 房间中直接发送命令：

- `!dingtalk bridge <dingtalk_conversation_id>` - 将此 Matrix 房间链接到钉钉会话
- `!dingtalk unbridge` - 移除此房间的桥接
- `!dingtalk help` - 显示可用命令

**示例：**
```
!dingtalk bridge "yourconversationid"
```

机器人会回复一条 curl 命令，你可以使用该命令通过 HTTP API 完成桥接。

### 方法 2：HTTP 配置 API

基础地址：`http://<bind_address>:<port>/admin`

#### 创建桥接（将 Matrix 房间链接到钉钉会话）

```bash
curl -X POST http://localhost:9006/admin/bridge \
  -H "Authorization: Bearer YOUR_WRITE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "matrix_room_id": "!abc123:yourdomain.com",
    "dingtalk_conversation_id": "yourconversationid",
    "dingtalk_conversation_name": "可选的会话名称"
  }'
```

**响应示例：**
```json
{
  "status": "bridged",
  "mapping": {
    "matrix_room_id": "!abc123:yourdomain.com",
    "dingtalk_conversation_id": "yourconversationid"
  }
}
```

#### 移除桥接

```bash
curl -X POST http://localhost:9006/admin/unbridge \
  -H "Authorization: Bearer YOUR_WRITE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "matrix_room_id": "!abc123:yourdomain.com"
  }'
```

**响应示例：**
```json
{
  "status": "unbridged",
  "matrix_room_id": "!abc123:yourdomain.com"
}
```

#### 列出所有桥接

```bash
curl -X GET "http://localhost:9006/admin/mappings?limit=100&offset=0" \
  -H "Authorization: Bearer YOUR_READ_TOKEN"
```

#### 获取桥接状态

```bash
curl -X GET http://localhost:9006/admin/status \
  -H "Authorization: Bearer YOUR_READ_TOKEN"
```

### 如何查找钉钉会话 ID

钉钉会话 ID 可以在钉钉 Stream 模式事件中找到。当从钉钉群组发送消息时，桥接器会在调试输出中记录：

```
[DEBUG] DingTalk event received: conversation=...
```

你也可以检查桥接器日志，当钉钉消息到达时查看会话 ID。

## Admin API

基础地址：`http://<bind_address>:<port>/admin`

所有端点都需要 Bearer token 认证（除非未配置 token）。

- `GET /status` - 获取桥接状态和统计信息
- `GET /mappings?limit=100&offset=0` - 列出所有房间映射
- `POST /bridge` - 创建新桥接（详见上方）
- `POST /unbridge` - 移除桥接（详见上方）
- `GET /dead-letters?status=pending&limit=100` - 列出死信事件
- `POST /dead-letters/<id>/replay` - 重放特定死信事件
- `POST /dead-letters/replay` - 按状态批量重放死信
- `POST /dead-letters/cleanup` - 清理旧死信事件

## 当前限制

- 现阶段以文本链路为主。
- 富媒体和更多事件类型尚未完全桥接。
- callback 兼容模式当前仅做 token 校验，暂未实现 AES 解密链路。

## 许可证

Apache-2.0
