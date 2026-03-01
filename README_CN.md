# Matrix 钉钉桥接器

使用 Rust 编写的 Matrix <-> 钉钉桥接器。

维护者: `Palpo Team`

## 当前状态

已具备可用基础能力：双向文本桥接、管理接口、死信重放/清理。

## 已实现能力

- Matrix Appservice transaction 处理
- Matrix -> 钉钉 文本转发（基于持久化 room mapping）
- 钉钉回调 -> Matrix 文本转发
- 按会话 webhook 路由发送（支持 token 或完整 webhook URL）
- `processed_events` 去重
- `message_mappings` 映射落库
- dead-letter 记录、查询、重放、清理
- Admin API: `status`、`mappings`、`bridge`、`unbridge`、`dead-letters/*`
- CLI 管理命令: `status`、`mappings`、`replay`、`dead-letter-cleanup`

## 钉钉模式说明

钉钉常见两种机器人模式：

- 群自定义 webhook 机器人：主要用于 webhook 出站发送。
- 企业应用机器人：支持回调事件与会话 webhook。

本项目当前支持 webhook 出站 + callback 入站的文本链路。

## 快速开始

1. 复制配置文件：

```bash
cp config/config.sample.yaml config.yaml
```

2. 至少配置以下字段：
- `bridge.domain`
- `bridge.homeserver_url`
- `database.url`
- `registration.bridge_id`
- `registration.appservice_token`
- `registration.homeserver_token`

3. 可选环境变量覆盖：
- `DINGTALK_WEBHOOK_URL`
- `DINGTALK_ACCESS_TOKEN`
- `DINGTALK_SECRET`
- `DINGTALK_CALLBACK_TOKEN`
- `MATRIX_BRIDGE_DINGTALK_PROVISIONING_*_TOKEN`

4. 运行：

```bash
cargo run --release
```

## Admin API

基础地址：`http://<bind_address>:<port>/admin`

- `GET /status`
- `GET /mappings?limit=100&offset=0`
- `POST /bridge`
- `POST /unbridge`
- `GET /dead-letters?status=pending&limit=100`
- `POST /dead-letters/<id>/replay`
- `POST /dead-letters/replay`
- `POST /dead-letters/cleanup`

## 当前限制

- 现阶段以文本链路为主。
- 富媒体和更多事件类型尚未完全桥接。

## 许可证

Apache-2.0
