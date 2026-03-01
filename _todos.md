# Matrix Bridge DingTalk - 开发任务清单

## 项目对比分析

### 代码量对比
| 项目 | 总代码行数 | 主要模块 |
|------|-----------|---------|
| matrix-bridge-feishu | 11,779 行 | 完整实现 |
| matrix-bridge-dingtalk | ~5,000+ 行 | 第一阶段完成 |

### 模块实现对比

| 模块 | Feishu 实现 | DingTalk 实现 | 状态 |
|------|------------|--------------|------|
| bridge/ | 4,530 行 | ~2,500 行 | ✅ 基本完成 |
| dingtalk 客户端 | 2,909 行 | ~1,200 行 | ✅ 基本完成 |
| formatter/ | 569 行 | ~200 行 | ✅ 基本完成 |
| web/ | 1,296 行 | ~300 行 | ✅ 基本完成 |
| config/ | 407 行 | 577 行 | ✅ 已完成 |
| database/ | 1,810 行 | 0 行 | ❌ 需要重新实现 |

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

## 第二阶段: 数据库层实现 (进行中)

### 2.1 钉钉 Webhook 客户端 (src/dingtalk/client.rs)
- [ ] 实现 `DingTalkClient` 结构体
- [ ] 实现 Webhook URL 构建方法
- [ ] 实现签名计算 (HmacSHA256 + timestamp)
- [ ] 实现 `send_text_message` - 发送文本消息
- [ ] 实现 `send_markdown_message` - 发送 Markdown 消息
- [ ] 实现 `send_link_message` - 发送链接消息
- [ ] 实现 `send_action_card_message` - 发送 ActionCard 消息
- [ ] 实现 `send_feed_card_message` - 发送 FeedCard 消息
- [ ] 实现 HTTP 请求发送和响应处理
- [ ] 实现错误重试机制
- [ ] 实现请求限流控制

### 2.2 钉钉服务层 (src/dingtalk/service.rs)
- [ ] 实现 `DingTalkService` 结构体
- [ ] 实现消息接收服务器 (回调模式)
- [ ] 实现消息验证和解析
- [ ] 实现事件分发机制
- [ ] 实现用户信息获取
- [ ] 实现群组信息获取

---

## 第三阶段: Matrix 集成 (Phase 3)

### 3.1 Matrix Appservice 集成
- [ ] 实现 Matrix 客户端初始化
- [ ] 实现 Appservice 配置
- [ ] 实现 Bot Intent 创建
- [ ] 实现 Ghost 用户注册
- [ ] 实现 Ghost 用户资料设置

### 3.2 Matrix 事件处理
- [ ] 实现 `MatrixEventProcessor` - 事件处理器
- [ ] 实现 `m.room.message` 事件处理
- [ ] 实现 `m.room.member` 事件处理
- [ ] 实现 `m.room.redaction` 事件处理
- [ ] 实现消息编辑事件处理
- [ ] 实现消息回复事件处理

---

## 第四阶段: 桥接核心逻辑 (Phase 4)

### 4.1 主桥接器 (src/bridge/dingtalk_bridge.rs)
- [ ] 实现 `DingTalkBridge` 结构体
- [ ] 实现 `new()` 构造函数
- [ ] 实现 `start()` 启动方法
- [ ] 实现 `stop()` 停止方法
- [ ] 实现数据库初始化
- [ ] 实现 Matrix 客户端初始化
- [ ] 实现钉钉服务初始化
- [ ] 实现房间映射缓存
- [ ] 实现用户映射缓存
- [ ] 实现 Intent 缓存

### 4.2 消息流转 (src/bridge/message_flow.rs)
- [ ] 实现 `MessageFlow` 结构体
- [ ] 实现 `DingTalkInboundMessage` - 钉钉入站消息
- [ ] 实现 `MatrixInboundMessage` - Matrix 入站消息
- [ ] 实现 `OutboundDingTalkMessage` - 钉钉出站消息
- [ ] 实现 `OutboundMatrixMessage` - Matrix 出站消息
- [ ] 实现 Matrix -> DingTalk 消息转换
- [ ] 实现 DingTalk -> Matrix 消息转换
- [ ] 实现媒体文件处理

### 4.3 门户管理 (src/bridge/portal.rs)
- [ ] 实现 `BridgePortal` 结构体
- [ ] 实现房间创建
- [ ] 实现房间桥接
- [ ] 实现房间解桥
- [ ] 实现房间信息同步

### 4.4 用户管理 (src/bridge/user.rs)
- [ ] 实现 `BridgeUser` 结构体
- [ ] 实现用户同步策略
- [ ] 实现用户资料同步
- [ ] 实现用户头像同步

### 4.5 命令处理 (src/bridge/command_handler.rs)
- [ ] 实现 `MatrixCommandHandler` - Matrix 命令处理
- [ ] 实现 `!bridge` 命令
- [ ] 实现 `!unbridge` 命令
- [ ] 实现 `!help` 命令
- [ ] 实现权限验证

---

## 第五阶段: 消息格式转换 (Phase 5)

### 5.1 钉钉到 Matrix 转换 (src/formatter/dingtalk_to_matrix.rs)
- [ ] 实现文本消息转换
- [ ] 实现 Markdown 转 HTML
- [ ] 实现链接转换
- [ ] 实现 @ 用户转换
- [ ] 实现媒体消息转换

### 5.2 Matrix 到钉钉转换 (src/formatter/matrix_to_dingtalk.rs)
- [ ] 实现文本消息转换
- [ ] 实现 HTML 转 Markdown
- [ ] 实现 @ 用户转换
- [ ] 实现媒体消息转换
- [ ] 实现消息回复格式

---

## 第六阶段: Web 服务 (Phase 6)

### 6.1 Web 服务器 (src/web/mod.rs)
- [ ] 实现 Salvo 路由配置
- [ ] 实现服务器启动
- [ ] 实现状态注入

### 6.2 健康检查 (src/web/health.rs)
- [ ] 实现 `/health` 端点
- [ ] 实现健康状态检查

### 6.3 指标收集 (src/web/metrics.rs)
- [ ] 实现 `/metrics` 端点
- [ ] 实现消息计数指标
- [ ] 实现延迟指标
- [ ] 实现错误计数指标

### 6.4 配置 API (src/web/provisioning.rs)
- [ ] 实现 `ProvisioningApi` 结构体
- [ ] 实现房间桥接 API
- [ ] 实现房间解桥 API
- [ ] 实现状态查询 API
- [ ] 实现映射查询 API
- [ ] 实现死信队列管理

### 6.5 Appservice 端点
- [ ] 实现 `/_matrix/app/v1/transactions/{txnId}` 端点
- [ ] 实现 `/_matrix/app/v1/users/{userId}` 端点
- [ ] 实现 `/_matrix/app/v1/rooms/{roomAlias}` 端点

---

## 第七阶段: 主程序完善 (Phase 7)

### 7.1 main.rs 完善
- [ ] 实现完整的命令行参数解析
- [ ] 实现 `--generate-config` 选项
- [ ] 实现 `status` 子命令
- [ ] 实现 `mappings` 子命令
- [ ] 实现管理 API 调用
- [ ] 实现优雅关闭

### 7.2 配置示例
- [ ] 创建 `example-config.yaml`
- [ ] 添加详细配置说明

---

## 第八阶段: 部署与文档 (Phase 8)

### 8.1 Docker 支持
- [ ] 创建 `Dockerfile`
- [ ] 创建 `compose.yml`
- [ ] 创建 `.dockerignore`

### 8.2 测试
- [ ] 创建 `tests/mock_dingtalk_matrix.rs`
- [ ] 编写单元测试
- [ ] 编写集成测试

### 8.3 文档
- [ ] 完善 `README.md`
- [ ] 创建 `README_CN.md`

---

## 实施计划

### Sprint 1: 核心架构 (1-2 天)
- 完成第一阶段所有任务
- 提交: `feat: refactor project structure to match feishu bridge`

### Sprint 2: 钉钉客户端 (2-3 天)
- 完成第二阶段所有任务
- 提交: `feat: implement dingtalk client and service`

### Sprint 3: Matrix 集成 (2-3 天)
- 完成第三阶段所有任务
- 提交: `feat: implement matrix appservice integration`

### Sprint 4: 桥接核心 (3-4 天)
- 完成第四阶段所有任务
- 提交: `feat: implement bridge core logic`

### Sprint 5: 格式转换 (1-2 天)
- 完成第五阶段所有任务
- 提交: `feat: implement message formatters`

### Sprint 6: Web 服务 (2-3 天)
- 完成第六阶段所有任务
- 提交: `feat: implement web server and APIs`

### Sprint 7: 主程序 (1 天)
- 完成第七阶段所有任务
- 提交: `feat: complete main program and CLI`

### Sprint 8: 部署文档 (1 天)
- 完成第八阶段所有任务
- 提交: `feat: add docker support and documentation`

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
