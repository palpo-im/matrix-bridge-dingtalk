# Matrix 钉钉桥接器

使用 Rust 编写的 Matrix <-> 钉钉 桥接器。

维护者: `Palpo Team`  
联系: `chris@acroidea.com`

## 项目状态

🚧 **正在积极开发中** 🚧

本项目目前处于早期开发阶段。查看 [_todos.md](_todos.md) 了解进度。

## 概述

- 纯 Rust 实现
- Matrix 应用服务 + 钉钉 Webhook 桥接核心
- HTTP 端点用于健康检查/状态/指标和配置
- 数据库后端: PostgreSQL, SQLite, 和 MySQL (功能开关)
- Docker 支持 (即将推出)

## 功能特性

### 已实现
- ✅ 项目结构和配置系统
- ✅ 配置文件解析 (YAML)
- ✅ 环境变量覆盖
- ✅ 工具模块 (错误处理, 日志, 格式化)
- ✅ 基础项目骨架

### 进行中
- 🚧 数据库层 (PostgreSQL/SQLite/MySQL)
- 🚧 钉钉 Webhook 客户端
- 🚧 Matrix 应用服务集成
- 🚧 消息桥接核心逻辑

### 计划中
- 📋 钉钉消息类型支持 (text, markdown, link, actionCard, feedCard)
- 📋 用户在线状态同步
- 📋 媒体文件桥接
- 📋 Web 配置界面
- 📋 Docker 部署

## 快速开始

### 前置要求

- Rust 工具链 (edition 2024)
- 配置了应用服务的 Matrix 主服务器
- 钉钉 Webhook 访问令牌
- 数据库: PostgreSQL, SQLite, 或 MySQL

### 配置

1. 复制示例配置:

```bash
cp config/config.sample.yaml config.yaml
```

2. 编辑 `config.yaml` 并设置:
   - `bridge.domain`: 你的 Matrix 服务器域名
   - `bridge.homeserver_url`: 你的 Matrix 主服务器 URL
   - `auth.webhooks`: 钉钉 webhook 令牌映射
   - `database.url`: 数据库连接字符串

3. 生成注册文件 (即将推出):

```bash
cargo run -- --generate-registration
```

### 构建和运行

```bash
# 构建
cargo build --release

# 运行
cargo run --release
```

## 钉钉 Webhook 配置

1. 在钉钉群聊中,进入群设置 → 智能群助手
2. 添加自定义机器人
3. 配置安全设置:
   - **自定义关键词**: 消息必须包含关键词
   - **加签**: 消息必须有有效签名 (推荐)
   - **IP 地址白名单**: 只允许特定 IP
4. 复制 webhook URL 并提取 `access_token`
5. 添加到 `config.yaml`:

```yaml
auth:
  webhooks:
    "chat_id": "access_token"
  security:
    type: "sign"
    secret: "your_secret_here"
```

## 支持的消息类型

### 钉钉 → Matrix
- Text → m.text
- Markdown → m.text (带格式)
- Link → m.text (带预览)
- ActionCard → m.text (带按钮)
- FeedCard → m.text (多个链接)

### Matrix → 钉钉
- m.text → Text 或 Markdown
- m.image → Markdown (带图片 URL)
- m.video → Markdown (带视频 URL)
- m.file → Markdown (带文件 URL)

## 许可证

Apache-2.0

## 致谢

- [matrix-bridge-discord](https://github.com/palpo-im/matrix-bridge-discord) - 参考实现
- [matrix-bot-sdk](https://github.com/palpo-im/matrix-bot-sdk) - Matrix Rust SDK
- [钉钉开放平台](https://open.dingtalk.com/) - 钉钉 API 文档
