# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-08-09

### Added
- **端侧科学化分析引擎** — 滑动窗口统计 + 趋势检测 + 异常检测(Z-score) + 自适应阈值 + 指数平滑预测 + 贝叶斯变点检测 + Pearson相关性 + 综合健康评分
- **统一持久化状态管理器** — 5模块统一注册 + auto-save + 启动恢复 + 版本追踪 + 健康报告
- **向量嵌入引擎** — TF-IDF稀疏向量 + BM25增强相似度 + 余弦+BM25混合评分 + LRU智能淘汰 + 磁盘持久化
- **WorkBuddy 功能衍生引擎** — 自动化规则引擎 + 活动分析 + 智能建议生成
- **输入验证与完整性保护** — 路径/ID/成本/风险/标签/文本 6类校验 + 校验和
- **HybridAgentRouter 四维决策** — Agent角色+紧急度+成本+质量分 + 低质量自动降级LAN
- **SchedulingEngine v2** — 加权关键词评分(80+关键词) + 多意图检测 + 次要意图输出
- **计费引擎官方定价更新** — DeepSeek/Kimi/GLM 2026最新费率 + 微钱精度(6位小数) + 字符预估
- **Session AES-256-GCM 加密** — 消息体加密落盘 + 向后兼容旧格式
- **数字底座加固** — 原子写入 + TOCTOU修复 + 流式API截断修复 + 速率限制统一 + 路径穿越防护
- **智能调度可视化** — SDLC Pipeline事件指标 + EvolutionConsole实时数据
- **第四红线：审批门禁 v3** — 十维风险评分 + 资费感知 + Auditor预检 + 演化建议
- **防幻觉引擎 v2** — 6维→10维：假编程/假完成/空文件夹/编造谎言 + 自适应惩罚
- **Agent 质量评分引擎** — 严谨度评分 + 幻觉→进化桥接 + 跨角色经验共享
- **C-VFS v3 持久化** — 真实文件快照 + 磁盘落盘 + 启动恢复 + 回滚
- **ProjectExplorer v3** — 真实文件树 + 检查点管理 + 项目健康 + Worktree集成
- **Worktree 7 Tauri 命令** — 全量暴露 + 审批门禁集成
- **Context Glue 持久化** — save/load 重启恢复
- **Shadow 记忆持久化** — save/load 重启恢复
- Parallel billing engine (Official / Budget / Router three-tier)
- API Key vault: Windows Credential Manager + file persistence + memory cache

### Fixed
- Kimi K3 context window: 256K → 1M
- CRITICAL: advance_pipeline 审批门禁静默绕过 (中文标签)
- CRITICAL: lib.rs 审批命令 API 全部不匹配 (12个命令)
- 审批规则 v3 科学化重构 (OperationRiskProfile 4轴评分)
- 审批事件 Blackboard 集成
- 计费引擎联动 (submit_with_cost)
- UTF-8 byte-slice panic (7 locations)
- C-VFS path traversal via canonicalize bypass
- ChatPanel stream listener leak
- [VAULT EMPTY] 401 error → 3-tier fallback
- 前端后端字段名不匹配 (ApprovalPanel/tauri.ts)

[Unreleased]: https://github.com/cnkevinbai/Chronos-Shadow/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.2.0
[0.1.1]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.1.1