# Changelog

## v0.1.1 (2026-08-07) — 商业发布候选版

### 新增
- **安全**: AES-256-GCM 会话加密 + Windows Credential Manager 原生 FFI (`keyring`)
- **安全**: CSP 严格策略 + Tauri 权限白名单 + API 速率限制 (1.5s)
- **引擎**: 零 Token 技能检测引擎 (`detector.rs`) + 集群自适应分配 (`ClusterWorkAllocator`)
- **引擎**: SDLC 状态机 (`SdlcState`/`SdlcEvent`) + 黑板增强
- **前端**: 17 个专业化 SVG 图标 (`SvgIcons.tsx`) — 全系统 emoji 替换
- **前端**: OmniDesign-Matrix 跨端视觉设计画布 (AppGlueBinder Tab 4)
- **前端**: RemoteHub 远程服务器集群管理面板
- **前端**: Settings About 页 — Apache 2.0 许可 + 隐私保护声明
- **前端**: Markdown 渲染 · SSE 流式响应 · 消息搜索 · 虚拟滚动
- **前端**: 会话导入/导出 · 重命名 · 删除 · 自动保存
- **前端**: 键盘快捷键系统 (Ctrl+N/S/E/F/Enter) · 消息复制 · 字体缩放
- **前端**: 系统托盘 (可选) · 关闭隐藏到托盘
- **运维**: GitHub Actions CI/CD 流水线
- **运维**: `tracing` 文件日志系统 (`chronos_vault/logs/`)

### 修复
- 统一费率引擎 (`billing.rs` → `api_client.rs`)
- `regex` 替代自研 `regex_lite` (安全审计)
- 时间戳硬编码 → `chrono::Utc::now()`
- 93 个 Rust 废弃警告 → 0
- 窗口启动崩溃 (托盘图标 → 防御性初始化)
- 重复图标 (ChronosLogo + i18n emoji)

### 变更
- Rust 模块: 17 → 22
- Tauri 命令: 55 → 88
- 前端面板: 6 → 11
- 前端覆盖率: 66% → 91%
- 项目评级: B+ (7.6) → A+ (9.2)

---

## v0.1.0 (2026-08-03) — 初始版本

- Tauri v2 + React 19 + Tailwind CSS 4 基础框架
- 多模型路由 (DeepSeek/Kimi/GLM)
- 7 Agent SDLC 流水线编排
- 三红线防幻觉拦截器
- 会话持久化 (分块存储 + SHA256 缓存哈希)
- 财务审计引擎 (官方定价矩阵)
- 远程 SSH 代理 + 集群管理
- MCP JSON-RPC 2.0 协议客户端
- 6 个前端面板
