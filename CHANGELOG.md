# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-08-09

### Added
- **端侧科学化分析引擎** (`local_analytics.rs`) — 滑动窗口统计 + 趋势检测 + 异常检测(Z-score) + 自适应阈值 + 指数平滑预测 + 贝叶斯变点检测 + Pearson相关性 + 综合健康评分
- **统一持久化状态管理器** (`state_manager.rs`) — 5模块统一注册 + auto-save + 启动恢复 + 版本追踪 + 健康报告
- **向量嵌入引擎** (`embedding.rs`) — TF-IDF稀疏向量 + BM25增强相似度 + 余弦+BM25混合评分 + LRU智能淘汰 + 磁盘持久化
- **WorkBuddy 功能衍生引擎** (`workbuddy_engine.rs`) — 自动化规则引擎 + 活动分析 + 智能建议生成
- **输入验证与完整性保护** (`input_guard.rs`) — 路径/ID/成本/风险/标签/文本 6类校验 + 校验和
- **HybridAgentRouter 四维决策** — Agent角色+紧急度+成本+质量分 → 智能模型选择 + 低质量自动降级LAN
- **SchedulingEngine v2** — 加权关键词评分(80+关键词) + 多意图检测 + 次要意图输出
- **计费引擎官方定价更新** — DeepSeek/Kimi/GLM 2026最新费率 + 微钱精度(6位小数) + 字符预估 + 缓存命中预估
- **Session AES-256-GCM 加密** — 消息体加密落盘 + 向后兼容旧格式
- **数字底座加固** — session_db原子写入 + 配置原子写入 + TOCTOU修复(try_reserve/settle) + 流式API截断修复 + 速率限制统一 + 检查点路径穿越防护
- **智能调度可视化** — SDLC Pipeline事件指标(📡事件数/✉️死信/📋任务) + EvolutionConsole实时数据
- **Context Glue 数据变换** — Format/FieldMap/RegexExtract/Custom 真实变换实现
- **第四红线：审批门禁 v3** — 十维风险评分 + 资费感知动态调整 + Auditor 预筛查 + 演化建议 + 端到端审批流程
- **ApprovalPanel 审批仪表盘** — 四视图（待审批/审计/规则/建议）+ 内联提交表单 + 风险进度条
- **防幻觉引擎 v2** — 6维→10维：假编程/假完成/空文件夹/编造谎言检测
- **Agent 质量评分引擎** — 严谨度评分 + 幻觉→进化桥接 + 跨角色经验共享
- **C-VFS v3 持久化升级** — 项目池 + 检查点磁盘落盘 (JSON)，启动自动恢复；检查点存储实际文件内容，支持真实回滚
- **C-VFS 文件树** — `list_project_files` 遍历真实目录返回 VfsNode 树，排除 .git/node_modules/target
- **检查点管理完整生命周期** — `cvfs_capture_checkpoint_v2` (内容快照) + `cvfs_restore_checkpoint` (回滚) + `cvfs_delete_checkpoint`
- **项目管理命令** — `cvfs_delete_project` + `cvfs_get_project_health` (文件数/大小/Git/检查点)
- **ProjectExplorer v3** — 三 Tab (真实文件树/检查点时间线/项目健康+Worktree) 替换硬编码 VFS_TREE
- **Worktree 状态集成** — ProjectExplorer 健康面板展示活跃/完成/已合并 Worktree 统计
- **第四红线：审批门禁 v2** (`approval_gate.rs`) — 风险评分 1-10 + 自动放行 + 人工审批 + 审计日志 + 资费感知动态风险 + 项目作用域规则 + Auditor 预筛查 + 演化学习建议
- **ApprovalPanel v2 审批仪表盘** — 四视图（待审批/审计/规则/建议）+ 风险进度条 + 行内规则编辑器 + 项目作用域标签
- **资费感知审批** (`submit_for_approval_with_cost`) — ¥1-5-10 三档自动风险升级
- **Auditor 预筛查** (`auditor_pre_screen_approval`) — 高风险操作自动代码审计，通过则降级风险 2 级
- **演化建议引擎** — 基于审批历史自动推荐规则阈值调优
- **项目作用域规则** — 审批规则可按项目隔离，支持全局+项目双层覆盖
- **Worktree 合并审批集成** — `merge_worktree` 前强制检查审批状态
- **SDLC 流水线审批集成** — Coder→Auditor / Auditor→ComplianceOfficer 阶段跃迁需人工核准
- **会话↔项目联动**: ChatPanel 接入真实 currentProject，会话自动绑定到当前项目名；新增 list_sessions_by_project 按项目过滤历史
- **Worktree 7 Tauri 命令**: create/activate/complete/merge/prune/list/get_stats 全量暴露，前端可操作 Git Worktree 隔离沙盒
- **Context Glue 持久化**: save/load 绑定到 glue_bindings.json，启动自动恢复；AppGlueBinder 切换后自动保存
- **Shadow 记忆持久化**: save/load 影子状态到 shadow_state.json，启动自动恢复建议历史
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

### Fixed
- Kimi K3 context window: 256K → 1M (billing_engine context_window + scheduling_engine 4处文案)
- Kimi K3 recommend_for_length 阈值: 32000 → 100000 tokens，避免在 DeepSeek 128K 窗口内过早切换
- **CRITICAL: advance_pipeline 审批门禁静默绕过** — AgentRole.label() 返回中文标签导致 `== "Coder"` 永远不成立，改为枚举匹配
- **CRITICAL: lib.rs 审批命令 API 全部不匹配** — 12 个命令对齐实际 ApprovalGate 方法签名
- **MODERATE: 前端后端字段名不匹配** — ApprovalPanel/tauri.ts 全部对齐 Rust 序列化名 (risk_level/submitted_at/decided_by/decision_comment)
- **审批规则 v3 科学化重构** — OperationRiskProfile 4轴评分 (影响范围/可逆性/资费影响/合规需求) 替代硬编码风险
- **审批事件 Blackboard 集成** — submit/decide/expire 发布 RedlineViolation 事件到 Orchestrator 事件总线
- **计费引擎联动** — submit_with_cost 读取 ChronosParallelBillingEngine 实时预算，超 80% 自动升级风险
- **ApprovalPanel 审批提交表单** — 内联 select+input 表单替换 browser prompt，支持资费感知提交

### Changed
- Kimi K3 best_for 描述更新为 "1M 超长文档分析 · 合同审查 · 项目全局理解"
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

[Unreleased]: https://github.com/cnkevinbai/Chronos-Shadow/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.2.0
[0.1.1]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.1.1
