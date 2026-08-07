# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Parallel billing engine (Official / Budget / Router three-tier)
- API Key vault: Windows Credential Manager + file persistence + memory cache
- API Key zero-plaintext: frontend never sees keys, Rust resolves from vault
- SSH StrictHostKeyChecking enforcement (remote_proxy + remote_cluster)
- Session ID path traversal validation (6 functions)
- Remote command injection hardening (validate_shell_arg + validate_tag, 16+ vectors)
- C-VFS path traversal fix (canonicalize + reject .. )
- CSP hardening (object-src, base-uri, frame-ancestors, form-action)
- React ErrorBoundary component
- TypeScript strict mode
- Donation channels (GitHub Sponsors, Afdian)
- .github/ISSUE_TEMPLATE (bug_report + feature_request)
- ROADMAP.md + SECURITY.md + FUNDING.yml
- Cargo.toml metadata (license/repository/homepage)
- ChatPanel status bar: per-model key status + smart hints

### Fixed
- UTF-8 byte-slice panic (7 locations: session_db, extractor, mcp_client, api_client, remote_proxy)
- DiffViewer silently dropping removed lines
- AppGlueBinder toggleHijack logic inversion
- ChatPanel stream listener leak (try/finally) + stale closure (messagesRef)
- Remote shell command injection (10+ vectors)
- C-VFS path traversal via canonicalize bypass
- Missing session_id validation in import_chat_session
- println! → tracing::info! in session_db
- [VAULT EMPTY] 401 error: keyring silent failure → 3-tier fallback (memory→file→WinCred)
- FooterBar fake savings multipliers removed (buddySaved×0.3/×0.42)
- App.tsx initial state magic numbers (0.342/1.82/84/0.52 → 0.0)
- Cost-cap UI end-to-end wiring (FooterBar → updateCostCap → billing_engine)
- SettingsPanel setTimeout-after-unmount tracked via useRef cleanup
- .gitignore: added chronos_vault/, config.json, .chronos_tmp/

### Changed
- API keys no longer stored in plaintext config.json
- chat_api/chat_api_stream: cost tracking uses parallel billing engine
- chat_api/chat_api_stream: key resolved server-side from vault (resolve_key_from_vault)
- ChatPanel: apiKey prop replaced with hasKeys object for per-model status
- README_EN.md fully synced with CN version (donations, dev status, correct counts)
- README.md/README_EN.md: module count 18/22→23, component count 10→11
- CHANGELOG format aligned with keepachangelog.com + compare links
- FooterBar: cost cap synced to backend via updateCostCap IPC
- Token billing: prompt/completion split estimated (split_tokens helper)

---

## [0.1.1] — 2026-08-07

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
- 前端面板: 6 → 9
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

[Unreleased]: https://github.com/cnkevinbai/Chronos-Shadow/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.1.1
