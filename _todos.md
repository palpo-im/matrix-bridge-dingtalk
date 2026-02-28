# Matrix Bridge DingTalk - 开发任务清单

## 项目概述
基于 Rust 实现的 Matrix <-> 钉钉 桥接器，参考 matrix-bridge-discord 项目架构。

## 第一阶段: 项目初始化与基础架构 (Phase 1)

### 1.1 项目结构搭建
- [x] 初始化 Cargo 项目
- [x] 创建基础目录结构 (src/bin, src/db, src/config, src/dingtalk, src/matrix, src/bridge, src/web, src/utils, src/parsers)
- [x] 创建 Cargo.toml 配置文件,添加必要依赖
- [x] 创建 .gitignore 文件
- [ ] 创建 README.md 和 README_CN.md

### 1.2 配置系统
- [x] 实现配置文件解析 (config.yaml)
- [x] 创建示例配置文件 config/config.sample.yaml
- [x] 实现环境变量覆盖配置
- [x] 实现配置验证器

### 1.3 数据库层
- [x] 设计数据库模型 (room_mapping, message_mapping, user_mapping)
- [x] 实现 PostgreSQL 支持 (90% 完成,需要修复可变引用)
- [x] 实现 SQLite 支持 (90% 完成,需要添加 diesel derive structs)
- [x] 实现 MySQL 支持 (占位符实现)
- [x] 创建数据库迁移脚本 migrations/001_initial.sql
- [x] 实现 DatabaseManager

**注**: 数据库层基本完成,但有编译错误需要修复:
- PostgreSQL 和 SQLite 实现需要将 `&conn` 改为 `&mut conn`
- SQLite 需要添加类似 PostgreSQL 的 DbXxxMapping 结构体

### 1.4 工具模块
- [x] 实现日志系统 (utils/logging.rs)
- [x] 实现错误处理 (utils/error.rs)
- [x] 实现格式化工具 (utils/formatting.rs)

## 第二阶段: 钉钉客户端实现 (Phase 2)

### 2.1 钉钉 Webhook API 客户端
- [ ] 实现钉钉 Webhook 客户端基础结构
- [ ] 实现消息签名验证 (HmacSHA256)
- [ ] 实现 Text 消息类型支持
- [ ] 实现 Markdown 消息类型支持
- [ ] 实现 Link 消息类型支持
- [ ] 实现 ActionCard 消息类型支持 (整体跳转/独立跳转)
- [ ] 实现 FeedCard 消息类型支持
- [ ] 实现 @ 用户功能
- [ ] 实现消息发送限流控制

### 2.2 钉钉消息接收 (可选)
- [ ] 实现钉钉回调服务器 (接收钉钉推送的消息)
- [ ] 实现消息验证和解析
- [ ] 实现事件处理器

### 2.3 钉钉用户和群组管理
- [ ] 实现获取用户信息
- [ ] 实现获取群组信息
- [ ] 实现用户头像处理

## 第三阶段: Matrix Appservice 实现 (Phase 3)

### 3.1 Matrix 客户端
- [ ] 集成 matrix-bot-sdk
- [ ] 实现 Matrix Appservice 初始化
- [ ] 实现事件处理器 (MatrixEventHandler)
- [ ] 实现事件处理器实现 (MatrixEventHandlerImpl)
- [ ] 实现事件处理器调度器 (MatrixEventProcessor)

### 3.2 Matrix 消息处理
- [ ] 实现发送文本消息
- [ ] 实现发送 Notice 消息
- [ ] 实现发送媒体消息 (图片、视频、音频、文件)
- [ ] 实现消息回复功能
- [ ] 实现消息编辑功能
- [ ] 实现消息删除/撤回功能
- [ ] 实现消息@功能

### 3.3 Matrix 用户管理
- [ ] 实现创建 Ghost 用户 (钉钉用户映射)
- [ ] 实现 Ghost 用户资料设置 (displayname, avatar)
- [ ] 实现 Ghost 用户权限管理
- [ ] 实现 Ghost 用户在线状态同步

### 3.4 Matrix 房间管理
- [ ] 实现创建房间
- [ ] 实现加入/离开房间
- [ ] 实现房间名称/主题设置
- [ ] 实现房间别名管理
- [ ] 实现房间成员管理 (邀请/踢出/封禁)

## 第四阶段: 桥接核心逻辑 (Phase 4)

### 4.1 消息流转 (Message Flow)
- [ ] 实现消息解析器 (Matrix -> DingTalk)
- [ ] 实现消息解析器 (DingTalk -> Matrix)
- [ ] 实现 Markdown 格式转换
- [ ] 实现媒体文件下载和上传
- [ ] 实现消息附件处理
- [ ] 实现消息回复/编辑/删除映射

### 4.2 桥接核心 (Bridge Core)
- [ ] 实现 BridgeCore 主结构
- [ ] 实现消息队列 (ChannelQueue)
- [ ] 实现 Matrix 消息处理 (handle_matrix_message)
- [ ] 实现钉钉消息处理 (handle_dingtalk_message)
- [ ] 实现消息映射存储 (message mapping)
- [ ] 实现房间映射管理 (room mapping)
- [ ] 实现用户映射管理 (user mapping)

### 4.3 用户同步 (User Sync)
- [ ] 实现钉钉用户到 Matrix Ghost 用户同步
- [ ] 实现用户资料同步 (昵称、头像)
- [ ] 实现用户在线状态同步
- [ ] 实现用户正在输入状态同步

### 4.4 在线状态处理 (Presence Handler)
- [ ] 实现钉钉在线状态到 Matrix 的同步
- [ ] 实现定时轮询机制
- [ ] 实现状态缓存

### 4.5 房间配置 (Provisioning)
- [ ] 实现房间桥接命令 (!bridge)
- [ ] 实现房间解除桥接命令 (!unbridge)
- [ ] 实现权限验证
- [ ] 实现桥接审批流程

## 第五阶段: Web 服务器与 API (Phase 5)

### 5.1 Web 服务器
- [ ] 集成 Salvo web 框架
- [ ] 实现健康检查端点 (/health)
- [ ] 实现状态端点 (/status)
- [ ] 实现 Metrics 端点 (/metrics)
- [ ] 实现第三方网络端点 (/thirdparty)

### 5.2 Appservice 端点
- [ ] 实现 Appservice 事务端点 (/_matrix/app/v1/transactions/{txnId})
- [ ] 实现用户查询端点 (/_matrix/app/v1/users/{userId})
- [ ] 实现房间查询端点 (/_matrix/app/v1/rooms/{roomAlias})

### 5.3 Provisioning API
- [ ] 实现房间桥接 API
- [ ] 实现房间解除桥接 API
- [ ] 实现获取桥接状态 API

## 第六阶段: 命令行与部署 (Phase 6)

### 6.1 命令行工具
- [ ] 集成 clap 命令行解析
- [ ] 实现配置文件路径参数
- [ ] 实现注册文件生成命令
- [ ] 实现版本信息显示

### 6.2 Docker 支持
- [ ] 创建 Dockerfile
- [ ] 创建 docker-compose.yml
- [ ] 创建 .dockerignore
- [ ] 优化镜像大小

### 6.3 文档
- [ ] 完善 README.md (安装、配置、使用)
- [ ] 创建配置文档
- [ ] 创建部署文档
- [ ] 创建开发文档

## 第七阶段: 测试与优化 (Phase 7)

### 7.1 单元测试
- [ ] 为钉钉客户端编写测试
- [ ] 为 Matrix 客户端编写测试
- [ ] 为桥接逻辑编写测试
- [ ] 为消息解析器编写测试

### 7.2 集成测试
- [ ] 编写端到端测试
- [ ] 编写数据库测试

### 7.3 性能优化
- [ ] 实现消息缓存
- [ ] 实现房间映射缓存
- [ ] 优化数据库查询
- [ ] 实现并发控制

### 7.4 错误处理
- [ ] 完善错误处理
- [ ] 添加重试机制
- [ ] 实现优雅关闭

## 第八阶段: 高级功能 (Phase 8)

### 8.1 消息格式增强
- [ ] 实现富文本消息转换
- [ ] 实现表情符号转换
- [ ] 实现自定义表情支持

### 8.2 媒体处理增强
- [ ] 实现媒体文件转码
- [ ] 实现媒体文件压缩
- [ ] 实现大文件分片上传

### 8.3 管理功能
- [ ] 实现管理员命令
- [ ] 实现日志级别动态调整
- [ ] 实现运行时配置重载

### 8.4 监控与告警
- [ ] 实现 Prometheus metrics
- [ ] 实现健康检查增强
- [ ] 实现错误告警

---

## 当前阶段: Phase 1 - 项目初始化与基础架构

### 下一步任务:
1. 创建基础目录结构
2. 配置 Cargo.toml
3. 实现配置系统
4. 实现数据库层

## 技术栈
- **语言**: Rust (edition 2024)
- **Web 框架**: Salvo 0.89
- **Matrix SDK**: matrix-bot-sdk 0.2.4
- **数据库**: Diesel 2.3.6 (支持 PostgreSQL/SQLite/MySQL)
- **异步运行时**: Tokio 1.40
- **HTTP 客户端**: Reqwest 0.13
- **序列化**: Serde 1.0
- **日志**: Tracing 0.1
- **命令行**: Clap 4.5

## 钉钉 API 特性
- **Webhook URL**: `https://oapi.dingtalk.com/robot/send?access_token=XXXX`
- **消息类型**:
  - text: 纯文本消息
  - markdown: Markdown 格式消息
  - link: 链接消息
  - actionCard: 卡片消息 (整体跳转/独立跳转)
  - feedCard: 多条链接流式消息
- **安全设置**:
  - 自定义关键词
  - 加签 (timestamp + sign)
  - IP 地址白名单
- **限流**: 每分钟 20 条消息

## 参考项目
- matrix-bridge-discord: 主要参考架构
- matrix-appservice-discord: Node.js 版本参考
- dingtalk-robot: 钉钉机器人 API 文档
