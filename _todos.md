# Matrix Bridge DingTalk - 缺失点与实施清单（2026-03-01）

## 对齐依据

### 钉钉官方/官方生态文档（在线核对）
- [x] 应用机器人能力概览（事件回调 + 会话 webhook 回复）
- [x] 机器人接收消息类型（text/image/voice/file/link/oa/action_card）
- [x] 自定义群机器人能力边界（Webhook 发消息能力、历史策略变更）
- [x] FAQ：Webhook/Stream 两种模式并存

### 对标项目
- [x] `D:\Works\palpo-im\matrix-bridge-discord`
- [x] `D:\Works\palpo-im\matrix-bridge-feishu`

---

## 真实缺失点（当前代码）

### A. 核心链路缺失
- [ ] Matrix Appservice 路由未真正挂载，`on_transaction` 仅日志打印，消息未转发
- [ ] DingTalk 回调入口缺失，`callback.enabled` 配置未生效
- [ ] `message_flow`/`event_processor` 多处空实现，未形成可运行双向链路

### B. 映射与持久化未接通
- [ ] Room mapping 仅内存缓存，未与数据库读写打通
- [ ] 进程重启后映射无法恢复到内存索引
- [ ] message mapping / processed event / dead-letter 虽有表结构但主流程未使用

### C. Admin/Provisioning 未完成
- [ ] `/admin/status`、`/admin/mappings`、`/admin/bridge` 返回占位数据
- [ ] CLI `replay` / `dead-letter-cleanup` 对应服务端接口缺失
- [ ] token 权限粒度（read/write/delete/admin）未完整落地

### D. 钉钉侧能力不完整
- [ ] 仅支持单 webhook 发送，未支持按会话映射 webhook
- [ ] 回调消息字段覆盖不足（如 `msgId`、`sessionWebhook`、camelCase 别名）
- [ ] 未将钉钉入站消息投递到 Matrix 房间

### E. 健壮性与安全
- [ ] `cleanup_dead_letters` 存在字符串拼接 SQL，需改为参数化
- [ ] 错误路径未统一写入 dead-letter，无法可靠重放

### F. 测试与文档
- [ ] 缺少关键单测（回调字段解析、URL 签名、dead-letter 清理、mapping API）
- [ ] README 与现状不一致（例如“已完成/进行中”描述）

---

## 实施阶段（按阶段提交）

### 第 1 阶段：基础能力与安全
- [x] 重写 `_todos.md`（本文件）并冻结阶段范围
- [x] DingTalk 客户端支持按会话 webhook 发送（兼容 token/full URL）
- [x] 修复 dead-letter cleanup 的 SQL 注入风险（参数化）

### 第 2 阶段：Admin/Provisioning 可用化
- [x] 接通数据库 stores 到 bridge
- [x] 实现 `/admin/status`、`/admin/mappings`、`/admin/bridge`、`/admin/unbridge`
- [x] 实现 dead-letter 查询/重放/清理接口并与 CLI 对齐
- [x] 落地 read/write/delete/admin token 校验

### 第 3 阶段：双向桥接主链路
- [ ] 挂载 Matrix Appservice router，接管 transaction
- [ ] Matrix -> DingTalk：按 mapping 转发、去重、记录 processed/message mapping
- [ ] DingTalk 回调路由与服务接入（token 校验 + 事件分发）
- [ ] DingTalk -> Matrix：按 mapping 回发文本并记录 message mapping
- [ ] 失败路径统一写入 dead-letter

### 第 4 阶段：测试与文档
- [ ] 增加关键单测（签名 URL、回调字段解析、dead-letter cleanup、mapping API）
- [ ] 更新 README / README_CN（能力边界、部署、已实现特性）

---

## 阶段提交记录
- [x] Phase 1 提交: `05e229b`
- [ ] Phase 2 提交: `TBD`
- [ ] Phase 3 提交: `TBD`
- [ ] Phase 4 提交: `TBD`
