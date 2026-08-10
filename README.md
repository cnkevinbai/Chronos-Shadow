# Chronos-Shadow (时空之影)

> 下一代工业级开源桌面智能体 — 将大模型潜能与 Windows 系统底层操控深度融合

[**🇨🇳 中文**](README.md) | [**🇬🇧 English**](README_EN.md)

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-v2.0-purple.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19.0-blue.svg)](https://react.dev)

Chronos-Shadow 是一款将大模型潜能与 Windows 系统底层操控完美缝合的工业级开源桌面智能体。内置 41 个 Rust 后端模块、9 个前端可视化面板、多模型混合路由中枢、端侧进化引擎集群、AES-256-GCM 会话加密，以及全链路安全审批体系。

> ✅ **v0.2.0 已发布** — 新增 Web 智能搜索、多级语义蒸馏、统一缓存引擎、多模型协作、任务智能分解、预测分析、9引擎进化总线。详见 [CHANGELOG.md](CHANGELOG.md)。

---

## 功能矩阵

### 🧠 多模型混合路由
- 支持 DeepSeek V4-Pro/Flash、Kimi K3/K2.7、GLM-5.2/5V-Turbo
- 自动路由：27条关键词规则 → 最优 Agent + 模型
- 多模型协作引擎：并行对比 / 投票仲裁 / 级联接力 / 分工协作
- LAN 热切换：云端超时 → 毫秒级降级到 Ollama 本地模型
- DeepSeek Context Caching 一折最大化命中
- 质量反馈闭环：EMA 更新模型画像 + 自动降级

### 🔒 金融级安全
- **AES-256-GCM** 会话分块流式加密（硬件指纹绑定密钥）
- **Windows Credential Manager** 原生 FFI（`keyring` → `CredWriteW`）
- CSP 严格策略 · Tauri 权限白名单 · 速率限制 · 费用熔断
- 四红线防护：Schema 强校验 + 沙盒路径拦截 + 自愈熔断 + 审批门禁
- 10维防幻觉引擎：置信度/虚构API/假编程/假完成/编造谎言等，自适应灵敏度调节
- 域名白名单：外网只读搜索/抓取，数据外泄永远禁止

### 🌐 Web 智能搜索与分析 (v0.2.0 新增)
- 域名白名单管理：预置 30+ 官方文档 + 技术社区域名
- Web 搜索引擎查询：Bing / DuckDuckGo 双引擎
- 网页抓取 + HTML→Markdown 自动转换
- 多源聚合研究：搜索→抓取→蒸馏→总结全自动
- 内容安全脱敏：API Keys / 文件路径自动移除
- 全量审计日志：所有外网请求可追溯

### 🧪 多级语义蒸馏引擎 (v0.2.0 新增)
- 三级蒸馏：Light(结构保持) / Medium(语义提取) / Deep(知识压缩)
- 7维强化学习权重自动进化
- 12种实体自动提取：版本/日期/Crate/包名/仓库/许可证等
- 自适应策略表：按内容类型(code/doc/blog)自动调参
- LRU 缓存 + 质量反馈闭环

### 🔄 统一缓存引擎 (v0.2.0 新增)
- 5分类独立 TTL：搜索(10min) / 抓取(1h) / 蒸馏(1d) / LLM响应(1h)
- 自适应 TTL：根据命中率动态调整(高命中×1.5 / 低命中×0.7)
- LRU 淘汰 + 磁盘持久化
- 全维度统计：命中率/API节省/分类型统计

### 🤖 多模型协作引擎 (v0.2.0 新增)
- 4种协作模式：Single / Parallel / Voting / Cascade
- 5模型能力画像：质量/延迟/成本/成功率 + 分任务质量
- EMA 质量反馈 + 自动降级切换
- 成本优化：在质量阈值下选最便宜模型

### 📊 任务智能引擎 (v0.2.0 新增)
- 7种任务模板：代码实现/修复/设计/重构/测试/调研/安全审计
- 5级复杂度估算：4维特征评分
- 智能分解：依赖图 + 拓扑并行组检测
- 成本/时间预估

### 🔮 预测分析引擎 (v0.2.0 新增)
- Holt-Winters 季节性 Token 用量预测
- SPC 统计过程控制成本异常检测
- 贝叶斯预算优化
- K-means 使用模式聚类

### 🧬 端侧进化总线 (v0.2.0 新增)
- 9引擎统一进化管理
- 反馈环路：自动调参 + 安全 clamp 保护
- 跨引擎知识迁移
- 每小时自动评估先进性

### 🎯 调度引擎 v3 (增强)
- TF-IDF 加权意图分类 (13类 × 100+ 关键词)
- 贝叶斯置信度更新 (Beta 分布平滑)
- Bigram/Trigram 精确模式匹配
- 紧急度估算 (15种信号)

### 🖥️ 远程服务器集群
- SSH 隧道连接 · 文件浏览 · 远程编译 · Git 快照/回滚
- 多服务器异步注册 · 项目→服务器映射 · 集群 Ping

### 🎨 OmniDesign-Matrix
- 自然语言 → 跨端 UI/UX 代码（PC + 移动端）
- Vercel / Linear / Apple 三主题一键切换

### 📊 全角色 SDLC 流水线
- 7 Agent 角色环形调度
- 任务 Kanban 创建/分配/完成/失败
- 零 Token 本地技能检测拦截

---

## 技术栈

| 层 | 技术 | 规模 |
|----|------|------|
| **桌面框架** | Tauri v2 (Rust) | — |
| **前端** | React 19 + TypeScript + Tailwind CSS 4 + Vite | 1815 modules, 469 KB JS |
| **后端** | Rust | 41 模块, ~936 KB 源码 |
| **加密** | AES-256-GCM + SHA-256 + keyring (Windows FFI) | — |
| **AI 路由** | reqwest + SSE 流式 + Context Caching | — |
| **存储** | 分块 JSON (Chronos Vault) | — |
| **测试** | Rust `#[cfg(test)]` + TypeScript strict mode | — |
| **i18n** | 中英文双语 | 333 个翻译键 |

---

## 快速开始

### 前置要求
- **Windows 10/11**（WebView2 已内置）
- [Rust](https://rustup.rs) 1.80+
- [Node.js](https://nodejs.org) 22+
- [Tauri CLI](https://tauri.app) v2
- [WiX Toolset](https://wixtoolset.org/) v3 (MSI 打包)

### 开发运行

```bash
cd chronos-shadow
npm install
npx tauri dev
```

### 生产构建

```bash
# 使用构建脚本
build-tauri.bat

# 或手动
npm run build
npx tauri build

# 产物:
#   src-tauri/target/release/chronos-shadow.exe       (9.34 MB)
#   src-tauri/target/release/bundle/msi/*.msi         (4.2 MB)
```

---

## 项目结构

```
chronos-shadow/
├── src/                        # React 前端
│   ├── views/                  # 12 个面板
│   │   ├── ChatPanel.tsx              # 沉浸式对话 + 行动调度引擎
│   │   ├── SdlcPipelinePanel.tsx      # 7 Agent 环形调度
│   │   ├── AppGlueBinder.tsx          # 跨软件粘合
│   │   ├── SkillMcpHub.tsx            # Skill + MCP 管理
│   │   ├── WebIntelligencePanel.tsx   # Web 搜索/抓取/研究/域名/审计
│   │   ├── AutoRoutingPanel.tsx       # 全局路由规则可视化
│   │   ├── RemoteHub.tsx              # 远程集群管理
│   │   ├── ProjectExplorer.tsx        # 项目时光机
│   │   ├── SecurityShieldPanel.tsx    # 安全风控
│   │   ├── RedlineGuardPanel.tsx      # 红线监控
│   │   ├── ApprovalPanel.tsx          # 审批门禁
│   │   ├── EvolutionConsole.tsx       # 进化系统 + 进化总线健康
│   │   └── SettingsPanel.tsx          # 全局配置 + 语言切换
│   ├── components/             # 11 个通用组件
│   └── lib/                    # IPC 层 + 类型 + i18n (333键)
├── src-tauri/                  # Rust 后端
│   ├── src/agent/              # 41 个核心模块
│   │   ├── web_intelligence.rs        # Web 智能搜索抓取分析
│   │   ├── distillation_engine.rs     # 多级语义蒸馏 + 强化学习进化
│   │   ├── cache_engine.rs            # 统一缓存 + 自适应TTL
│   │   ├── collaboration_engine.rs    # 多模型协作引擎
│   │   ├── task_intelligence.rs       # 任务智能分解引擎
│   │   ├── predictive_analytics.rs    # 预测分析引擎
│   │   ├── evolution_bus.rs           # 9引擎统一进化总线
│   │   ├── scheduling_engine.rs       # 调度引擎 v3 (TF-IDF+Bayesian)
│   │   ├── hallucination_guard.rs     # 10维防幻觉 + 自适应灵敏度
│   │   ├── security_boundary.rs       # 安全边界 (精细权限)
│   │   ├── approval_gate.rs           # 审批门禁 (4维风险评分)
│   │   └── [31 more modules]
│   ├── skills/                 # 专属 Skill 定义 (10个)
│   └── resources/              # ONNX 模型 + MCP 配置
├── .github/workflows/          # CI/CD
└── dist/                       # 前端构建产物
```

---

## 面板一览

| # | 面板 | 功能 |
|---|------|------|
| 1 | **ChatPanel** | 流式对话 · 附件上传 · @子智能体 · /宏指令 · 行动调度引擎 |
| 2 | **SdlcPipelinePanel** | 7 Agent 环形调度 · 任务 Kanban · 阶段跃迁 |
| 3 | **AutoRoutingPanel** 🆕 | 27条路由规则可视化 · 模型矩阵 · Agent映射 · 搜索过滤 |
| 4 | **WebIntelligencePanel** 🆕 | Web搜索 · 页面抓取 · 多源研究 · 域名白名单 · 审计日志 |
| 5 | **SkillMcpHub** | Skill 管理 · MCP 连接 · JSON Schema 查看 |
| 6 | **RemoteHub** | SSH 集群 · 远程编译 · 文件浏览 |
| 7 | **ProjectExplorer** | C-VFS 项目创建 · 时光机快照 · 回滚 |
| 8 | **AppGlueBinder** | 跨软件粘合 · Context Glue · Buddy Scan |
| 9 | **EvolutionConsole** | 错题本 · 科技树 · Agent质量 · 进化总线健康 🆕 |
| 10 | **ApprovalPanel** | 审批门禁 · 风险评分 · 规则管理 |
| 11 | **RedlineGuardPanel** | 三红线防线 · Schema 测试 · 沙盒状态 |
| 12 | **SecurityShieldPanel** | 安全风控 · 熔断重置 |
| 13 | **SettingsPanel** | API密钥 · 成本控制 · LAN网关 · 安全红线 · 语言切换 |

---

## 核心引擎架构

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: 安全体系                                           │
│ ├─ security_boundary  (6操作类型, 精细权限)                  │
│ ├─ redline            (三红线 Schema强校验)                  │
│ ├─ approval_gate      (4维风险评分 + 自动放行)              │
│ └─ hallucination_guard (10维检测 + 自适应灵敏度)            │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: 智能引擎                                           │
│ ├─ scheduling_engine  (TF-IDF + Bayesian + N-gram)         │
│ ├─ task_intelligence  (7模板智能分解 + 拓扑并行)            │
│ ├─ collaboration      (4模式多模型协作)                     │
│ └─ predictive         (Holt-Winters + SPC + 预算优化)       │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: 信息获取                                           │
│ ├─ web_intelligence   (搜索/抓取/研究 + 域名白名单)         │
│ ├─ distillation       (3级蒸馏 + 7维RL权重进化)             │
│ └─ cache_engine       (5分类自适应TTL + LRU)                │
├─────────────────────────────────────────────────────────────┤
│ Layer 4: 进化系统                                           │
│ ├─ evolution_bus      (9引擎统一进化 + 知识迁移)            │
│ └─ agent_quality      (严谨评分 + 进化桥接)                 │
├─────────────────────────────────────────────────────────────┤
│ Layer 5: 基础设施                                           │
│ ├─ router             (多模型混合路由 + LAN降级)            │
│ ├─ billing_engine     (3轨并行计费 + 熔断)                  │
│ └─ orchestrator       (SDLC 7角色环形调度)                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 💝 支持项目

Chronos-Shadow 是免费开源软件。如果您觉得它有用，欢迎通过以下方式支持：

- ⭐ [GitHub Stars](https://github.com/cnkevinbai/Chronos-Shadow) — 免费的鼓励
- 💰 [GitHub Sponsors](https://github.com/sponsors/cnkevinbai) — 月度赞助
- ☕ [爱发电](https://afdian.com/a/chronos-shadow) — 一次性支持

---

## 开源协议

本项目基于 [Apache License 2.0](LICENSE) 发布。

Copyright 2026 Chronos-Shadow Open Source Team.

---

## 致谢

- [Tauri](https://tauri.app) — 轻量级桌面框架
- [React](https://react.dev) — UI 框架
- [Tailwind CSS](https://tailwindcss.com) — 样式系统
- [DeepSeek](https://deepseek.com) · [Kimi](https://kimi.moonshot.cn) · [GLM](https://bigmodel.cn) — 大模型 API