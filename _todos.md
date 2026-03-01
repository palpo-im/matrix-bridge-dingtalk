# Matrix Bridge DingTalk - 开发任务清单

## 项目对比分析

### 代码量对比
| 项目 | 总代码行数 | 主要模块 |
|------|-----------|---------|
| matrix-bridge-feishu | 11,779 行 | 完整实现 |
| matrix-bridge-dingtalk | 4,496 行 | 核心功能完成 |

### 模块实现对比

| 模块 | Feishu 实现 | DingTalk 实现 | 状态 |
|------|------------|--------------|------|
| bridge/ | 4,530 行 | ~2,500 行 | ✅ 基本完成 |
| dingtalk 客户端 | 2,909 行 | ~600 行 | ✅ 基本完成 |
| formatter/ | 569 行 | ~140 行 | ✅ 基本完成 |
| web/ | 1,296 行 | ~250 行 | ✅ 基本完成 |
| database/ | 1,810 行 | ~1,100 行 | ✅ 已完成 |
| config/ | 407 行 | 577 行 | ✅ 已完成 |

---

## ✅ 第一阶段: 核心架构重构 (已完成)

### 1.1 项目结构调整 ✅
- [x] 创建 `src/dingtalk/mod.rs` 模块入口
- [x] 创建 `src/dingtalk/client.rs` - 钉钉 API 客户端
- [x] 创建 `src/dingtalk/service.rs` - 钉钉服务层
- [x] 创建 `src/dingtalk/types.rs` - 钉钉 API 类型定义
- [x] 创建 `src/bridge/mod.rs` 模块入口
- [x] 创建 `src/bridge/dingtalk_bridge.rs` - 主桥接器
- [x] 创建 `src/bridge/event_processor.rs` - Matrix 事件处理器
- [x] 创建 `src/bridge/message_flow.rs` - 消息流转
- [x] 创建 `src/bridge/portal.rs` - 门户房间管理
- [x] 创建 `src/bridge/puppet.rs` - 傀儡用户管理
- [x] 创建 `src/bridge/user.rs` - 用户管理
- [x] 创建 `src/bridge/command_handler.rs` - 命令处理
- [x] 创建 `src/bridge/presence_handler.rs` - 在线状态处理
- [x] 创建 `src/bridge/provisioning.rs` - 配置供应
- [x] 创建 `src/bridge/matrix_event_parser.rs` - Matrix 事件解析
- [x] 创建 `src/bridge/matrix_to_dingtalk_dispatcher.rs` - Matrix 到钉钉调度
- [x] 创建 `src/bridge/message.rs` - 消息类型定义
- [x] 创建 `src/formatter/mod.rs` - 格式转换模块入口
- [x] 创建 `src/formatter/dingtalk_to_matrix.rs` - 钉钉到 Matrix 格式转换
- [x] 创建 `src/formatter/matrix_to_dingtalk.rs` - Matrix 到钉钉格式转换
- [x] 创建 `src/web/mod.rs` - Web 模块入口
- [x] 创建 `src/web/health.rs` - 健康检查
- [x] 创建 `src/web/metrics.rs` - 指标收集
- [x] 创建 `src/web/provisioning.rs` - 配置 API
- [x] 创建 `src/lib.rs` - 库入口

### 1.2 Cargo.toml 依赖调整 ✅
- [x] 精简 salvo features
- [x] 添加 `urlencoding` 依赖
- [x] 添加 `lru` 依赖

---

## ✅ 第二阶段: 数据库层实现 (已完成)

### 2.1 数据库核心
- [x] 创建 `src/database/mod.rs` - 数据库模块入口
- [x] 创建 `src/database/error.rs` - 数据库错误类型
- [x] 创建 `src/database/models.rs` - 数据模型定义
- [x] 创建 `src/database/stores.rs` - Store trait 定义
- [x] 创建 `src/database/sqlite_stores.rs` - SQLite 实现

### 2.2 数据模型
- [x] RoomMapping - 房间映射
- [x] UserMapping - 用户映射
- [x] MessageMapping - 消息映射
- [x] ProcessedEvent - 已处理事件
- [x] DeadLetterEvent - 死信事件
- [x] MediaCacheEntry - 媒体缓存

### 2.3 Store 实现
- [x] RoomStore - 房间存储
- [x] UserStore - 用户存储
- [x] MessageStore - 消息存储
- [x] EventStore - 事件存储
- [x] DeadLetterStore - 死信存储
- [x] MediaStore - 媒体存储

---

## ✅ 第三阶段: 钉钉客户端实现 (已完成)

### 3.1 钉钉 Webhook 客户端 (src/dingtalk/client.rs)
- [x] 实现 `DingTalkClient` 结构体
- [x] 实现 Webhook URL 构建方法
- [x] 实现签名计算 (HmacSHA256 + timestamp)
- [x] 实现 `send_text` - 发送文本消息
- [x] 实现 `send_markdown` - 发送 Markdown 消息
- [x] 实现 `send_link` - 发送链接消息
- [x] 实现 `send_action_card` - 发送 ActionCard 消息
- [x] 实现 `send_feed_card` - 发送 FeedCard 消息
- [x] 实现 HTTP 请求发送和响应处理
- [x] 实现错误重试机制
- [x] 实现请求限流控制

### 3.2 钉钉服务层 (src/dingtalk/service.rs)
- [x] 实现 `DingTalkService` 结构体
- [x] 实现消息回调处理
- [x] 实现消息验证和解析
- [x] 实现事件分发机制
- [x] 实现文本消息处理

### 3.3 钉钉类型定义 (src/dingtalk/types.rs)
- [x] DingTalkUser - 用户类型
- [x] DingTalkChat - 群组类型
- [x] DingTalkMessage - 消息类型
- [x] DingTalkResponse - 响应类型
- [x] DingTalkWebhookMessage - Webhook 消息

---

## ✅ 第四阶段: Matrix 集成 (已完成)

### 4.1 Matrix Appservice 集成
- [x] 实现 Matrix 客户端初始化
- [x] 实现 Appservice 配置
- [x] 实现 Bot Intent 创建
- [x] 实现 BridgeHandler 事务处理

### 4.2 Matrix 事件处理
- [x] 实现 `MatrixEventProcessor` - 事件处理器
- [x] 实现 `MatrixEvent` - 事件类型定义
- [x] 实现 `MatrixEventParser` - 事件解析
- [x] 实现 `ParsedEvent` - 解析结果

---

## ✅ 第五阶段: 桥接核心逻辑 (已完成)

### 5.1 主桥接器 (src/bridge/dingtalk_bridge.rs)
- [x] 实现 `DingTalkBridge` 结构体
- [x] 实现 `new()` 构造函数
- [x] 实现 `start()` 启动方法
- [x] 实现 `stop()` 停止方法
- [x] 实现钉钉服务初始化
- [x] 实现用户同步维护循环

### 5.2 消息流转 (src/bridge/message_flow.rs)
- [x] 实现 `MessageFlow` 结构体
- [x] 实现 `DingTalkInboundMessage` - 钉钉入站消息
- [x] 实现 `MatrixInboundMessage` - Matrix 入站消息
- [x] 实现 `OutboundDingTalkMessage` - 钉钉出站消息
- [x] 实现 `OutboundMatrixMessage` - Matrix 出站消息

### 5.3 门户管理 (src/bridge/portal.rs)
- [x] 实现 `BridgePortal` 结构体
- [x] 实现 `PortalManager` 管理器
- [x] 实现房间映射缓存
- [x] 实现房间添加/删除

### 5.4 用户管理 (src/bridge/user.rs)
- [x] 实现 `BridgeUser` 结构体
- [x] 实现 `UserSyncPolicy` 同步策略
- [x] 实现用户同步检测

### 5.5 命令处理 (src/bridge/command_handler.rs)
- [x] 实现 `MatrixCommandHandler` - Matrix 命令处理
- [x] 实现 `!bridge` 命令
- [x] 实现 `!unbridge` 命令
- [x] 实现 `!help` 命令
- [x] 实现 `DingTalkCommandHandler` - 钉钉命令处理

### 5.6 配置供应 (src/bridge/provisioning.rs)
- [x] 实现 `ProvisioningCoordinator` - 配置协调器
- [x] 实现 `PendingBridgeRequest` - 待处理请求
- [x] 实现请求审批/拒绝

### 5.7 在线状态 (src/bridge/presence_handler.rs)
- [x] 实现 `PresenceHandler` - 在线状态处理器
- [x] 实现 `DingTalkPresence` - 钉钉在线状态
- [x] 实现状态缓存

---

## ✅ 第六阶段: 消息格式转换 (已完成)

### 6.1 钉钉到 Matrix 转换 (src/formatter/dingtalk_to_matrix.rs)
- [x] 实现 `DingTalkToMatrixFormatter` - 格式化器
- [x] 实现文本消息转换
- [x] 实现 Markdown 转 HTML
- [x] 实现 @ 用户转换

### 6.2 Matrix 到钉钉转换 (src/formatter/matrix_to_dingtalk.rs)
- [x] 实现 `MatrixToDingTalkFormatter` - 格式化器
- [x] 实现文本消息转换
- [x] 实现 HTML 转 Markdown
- [x] 实现 @ 用户转换
- [x] 实现文本截断

---

## ✅ 第七阶段: Web 服务 (已完成)

### 7.1 Web 服务器 (src/web/mod.rs)
- [x] 实现 health_endpoint
- [x] 实现 metrics_endpoint
- [x] 实现 ProvisioningApi

### 7.2 健康检查 (src/web/health.rs)
- [x] 实现 `/health` 端点
- [x] 返回 JSON 状态

### 7.3 指标收集 (src/web/metrics.rs)
- [x] 实现 `/metrics` 端点
- [x] 实现消息计数指标
- [x] 实现错误计数指标
- [x] 实现 `ScopedTimer` - 计时器

### 7.4 配置 API (src/web/provisioning.rs)
- [x] 实现 `ProvisioningApi` 结构体
- [x] 实现 `get_status` - 状态查询 API
- [x] 实现 `mappings` - 映射查询 API
- [x] 实现 `bridge_room` - 房间桥接 API
- [x] 实现 Token 验证

---

## ✅ 第八阶段: 主程序完善 (已完成)

### 8.1 main.rs 完善
- [x] 实现命令行参数解析 (Clap)
- [x] 实现 `--generate-config` 选项
- [x] 实现 `status` 子命令
- [x] 实现 `mappings` 子命令
- [x] 实现 `replay` 子命令
- [x] 实现 `dead-letter-cleanup` 子命令
- [x] 实现管理 API 调用
- [x] 集成数据库初始化
- [x] 集成 Web 服务器启动
- [x] 实现优雅关闭

### 8.2 配置示例
- [x] 已有 `config/config.sample.yaml`

---

## ✅ 第九阶段: 部署与文档 (待实现)

### 9.1 Docker 支持
- [ ] 创建 `Dockerfile`
- [ ] 创建 `compose.yml`
- [ ] 创建 `.dockerignore`

### 9.2 测试
- [ ] 创建 `tests/mock_dingtalk_matrix.rs`
- [ ] 编写单元测试
- [ ] 编写集成测试

### 9.3 文档
- [ ] 完善 `README.md`
- [ ] 更新 `README_CN.md`

---

## 当前状态

### 代码量对比
| 项目 | 代码行数 |
|------|---------|
| matrix-bridge-feishu | 11,779 行 |
| matrix-bridge-dingtalk | 5,175 行 |

### 已完成模块
- ✅ bridge/ - 桥接核心
- ✅ dingtalk/ - 钉钉客户端
- ✅ formatter/ - 消息格式转换
- ✅ web/ - Web API
- ✅ database/ - 数据库层
- ✅ config/ - 配置系统
- ✅ utils/ - 工具模块
- ✅ main.rs - 主程序

---

## 实施进度

### ✅ 已完成
- Sprint 1: 核心架构 - 提交: `626ab3b`
- Sprint 2: 数据库层 - 提交: `92a4f31`
- Sprint 3: 主程序集成 - 提交: `41850d5`

### 📋 待开始
- Sprint 4: 部署文档

---

## 技术栈
- **语言**: Rust (edition 2024)
- **Web 框架**: Salvo 0.89
- **Matrix SDK**: matrix-bot-sdk 0.2.4
- **数据库**: Diesel 2.3.6 (SQLite)
- **异步运行时**: Tokio 1
- **HTTP 客户端**: Reqwest 0.13
- **序列化**: Serde 1.0
- **日志**: Tracing 0.1
- **命令行**: Clap 4

## 钉钉 API 特性
- **Webhook URL**: `https://oapi.dingtalk.com/robot/send?access_token=XXXX`
- **消息类型**: text, markdown, link, actionCard, feedCard
- **安全设置**: 自定义关键词, 加签, IP 白名单
- **限流**: 每分钟 20 条消息
