# Chronos-Shadow Roadmap

> 免费开源 · 接受捐赠 · 社区驱动

## ✅ v0.1.1 — 已完成 (2026-08-07)

- [x] Tauri v2 + React 19 + Tailwind CSS 4 框架
- [x] 多模型混合路由 (DeepSeek / Kimi / GLM / Ollama)
- [x] 7-Agent SDLC 流水线编排
- [x] 三红线防幻觉拦截器
- [x] AES-256-GCM 会话加密框架
- [x] Windows Credential Manager 原生 FFI
- [x] 三层并行计费引擎
- [x] 远程 SSH 代理 + 集群管理
- [x] MCP JSON-RPC 2.0 协议客户端
- [x] GitHub Actions CI/CD (Windows MSI)
- [x] 9 个前端面板 + 11 个组件

## ✅ v0.2.0 — 已完成 (2026-08-09)

- [x] **端侧科学化分析引擎** — 统计/趋势/异常/预测/变点/相关性/健康评分
- [x] **统一持久化状态管理器** — 5模块注册 + auto-save + 版本追踪
- [x] **向量嵌入引擎** — TF-IDF + BM25 + LRU淘汰 + 持久化
- [x] **WorkBuddy 功能衍生** — 自动化规则 + 活动分析 + 智能建议
- [x] **第四红线审批门禁** — 十维风险评分 + 资费感知 + Auditor预检 + 演化建议
- [x] **防幻觉引擎 v2** — 6维→10维：假编程/假完成/空文件夹/编造谎言
- [x] **Agent 质量评分引擎** — 严谨度评分 + 幻觉→进化桥接
- [x] **C-VFS 持久化升级** — 真实文件快照 + 磁盘落盘 + 启动恢复 + 回滚
- [x] **数字底座加固** — 原子写入 + 速率限制统一 + TOCTOU修复 + 路径穿越防护
- [x] **Worktree 全命令暴露** — 7 Tauri命令 + 审批门禁集成
- [x] **会话↔项目联动** — 按项目过滤历史会话 + 项目作用域规则
- [x] **Context/Shadow 持久化** — 重启自动恢复状态
- [x] **项目管理器 v3** — 真实文件树 + 检查点管理 + 项目健康 + Worktree状态
- [x] **对话模块增强** — 项目指示器 + 消息成本 + 审批状态栏
- [x] **RemoteHub/SkillMcp 升级** — 审批门禁 + 实时数据渲染

## 🚧 开发中

- [ ] ONNX 端侧隐私遮罩模型集成（当前为占位文件）
- [ ] Buddy Scan 像素级视觉走查（Win32 钩子已就绪，回调逻辑待实现）
- [ ] 向量嵌入引擎（当前为哈希模拟，待接入 fastembed）
- [ ] Session AES-256-GCM 加密落盘（加密函数已实现，待接入 session_db）
- [ ] Skill 子系统可执行脚本补全（7/10 缺失 run.ps1）

## 📋 v0.2.0 — 计划中

- [ ] Router → HybridAgentRouter 迁移
- [ ] Orchestrator 事件总线重连
- [ ] SDLC 状态机 Blackboard 统一
- [ ] 前端 E2E 测试 (Vitest + React Testing Library)
- [ ] Rust Tauri 命令集成测试
- [ ] MCP 服务器真实脚本集成
- [ ] 内存管理优化（checkpoint 上限、MCP 进程清理）
- [ ] 国际化 (i18n) 全覆盖
- [ ] macOS / Linux 实验性支持

## 🌟 未来愿景

- [ ] MCP Skill 市场
- [ ] 移动端远程控制面板
- [ ] 多用户团队协作
- [ ] 云端 Agent 托管服务

---

> 优先级由社区投票决定。欢迎通过 [Issues](https://github.com/cnkevinbai/Chronos-Shadow/issues) 提议新功能！
