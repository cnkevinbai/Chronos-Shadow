# Chronos-Shadow (时空之影)

> 下一代工业级开源桌面智能体 — 将大模型潜能与 Windows 系统底层操控深度融合

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-v2.0-purple.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19.0-blue.svg)](https://react.dev)

Chronos-Shadow 是一款将大模型潜能与 Windows 系统底层操控完美缝合的工业级开源桌面智能体。内置标准 MCP 客户端总线、自适应分布式集群管理、AES-256-GCM 会话加密、Windows 原生凭据保险箱、零 Token 本地技能检测引擎，以及游戏化降本对账单。

> 🚧 **开发状态说明**：ONNX 隐私遮罩、Buddy Scan 视觉走查、向量嵌入等模块目前为框架占位，正在积极开发中。详见 [CHANGELOG.md](CHANGELOG.md)。

---

## 功能矩阵

### 🧠 多模型混合路由
- 支持 DeepSeek V4-Pro/Flash、Kimi K3/K2.7、GLM-5.2/5V-Turbo
- 自动路由：根据 Agent 角色（PM/Coder/Auditor）选最优模型
- LAN 热切换：云端超时 → 毫秒级降级到 Ollama 本地模型
- DeepSeek Context Caching 一折最大化命中

### 🔒 金融级安全
- **AES-256-GCM** 会话分块流式加密（硬件指纹绑定密钥）
- **Windows Credential Manager** 原生 FFI（`keyring` → `CredWriteW`）
- CSP 严格策略 · Tauri 权限白名单 · 速率限制 · 费用熔断
- 三红线防幻觉：Schema 强校验 + 沙盒路径拦截 + 自愈熔断

### 🖥️ 远程服务器集群
- SSH 隧道连接 · 文件浏览 · 远程编译 · Git 快照/回滚
- 多服务器异步注册 · 项目→服务器映射 · 集群 Ping

### 🎨 OmniDesign-Matrix
- 自然语言 → 跨端 UI/UX 代码（PC + 移动端）
- Vercel / Linear / Apple 三主题一键切换
- 双端视窗实时预览 + ONNX 像素级还原度走查 🚧

### 📊 全角色 SDLC 流水线
- 7 Agent 角色环形调度（PM → Designer → Architect → Planner → Coder → Auditor → Verifier）
- 任务 Kanban 创建/分配/完成/失败
- 零 Token 本地技能检测拦截
- 🚧 Buddy Scan 像素级视觉走查（开发中）
- 🚧 ONNX 端侧隐私遮罩（开发中）

---

## 技术栈

| 层 | 技术 |
|----|------|
| **桌面框架** | Tauri v2 (Rust) |
| **前端** | React 19 + TypeScript + Tailwind CSS 4 + Vite |
| **后端** | Rust (23 模块, 10,600+ 行) |
| **加密** | AES-256-GCM + SHA-256 + keyring (Windows FFI) |
| **AI 路由** | reqwest + SSE 流式 + Context Caching |
| **存储** | 分块 JSON (Chronos Vault) |
| **测试** | Rust `#[cfg(test)]` + TypeScript strict mode |

---

## 快速开始

### 前置要求
- **Windows 10/11**（WebView2 已内置）
- [Rust](https://rustup.rs) 1.80+
- [Node.js](https://nodejs.org) 22+
- [Tauri CLI](https://tauri.app) v2

### 开发运行

```bash
cd chronos-shadow
npm install
npm run tauri dev
```

### 生产构建

```bash
npm run tauri build
# 产物: src-tauri/target/release/bundle/msi/Chronos-Shadow_*.msi
```

### 绿色便携版

直接运行 `chronos-shadow.exe`（9.3 MB），无需安装。

---

## 项目结构

```
chronos-shadow/
├── src/                    # React 前端
│   ├── views/              # 9 个面板
│   ├── components/         # 11 个组件（含 SvgIcons）
│   └── lib/                # IPC 层 + 类型 + i18n
├── src-tauri/              # Rust 后端
│   ├── src/agent/          # 23 个核心模块
│   ├── skills/             # 专属 Skill 定义
│   └── resources/          # ONNX 模型 + MCP 配置
├── .github/workflows/      # CI/CD
└── dist/                   # 前端构建产物
```

---

## 面板一览

| 面板 | 图标 | 功能 |
|------|------|------|
| ChatPanel | `ChatIcon` | 流式对话 · 附件 · @/斜杠 · Markdown |
| SdlcPipelinePanel | `PipelineIcon` | 7 Agent 环形调度 · 任务 Kanban |
| AppGlueBinder | `GlueIcon` | 跨软件粘合 · OmniDesign 画布 |
| SkillMcpHub | `McpIcon` | Skill 管理 · MCP 连接 |
| RemoteHub | `RemoteIcon` | SSH 集群 · 文件浏览 · 编译 |
| ProjectExplorer | `ChronosFolderIcon` | C-VFS 项目创建 · 时光机快照 |
| SecurityShield | `ShieldIcon` | Shadow 开关 · 熔断重置 |
| RedlineGuard | `ShieldIcon` | 三红线防线 · Schema 测试 |
| EvolutionConsole | `EvolutionIcon` | 进化系统 · 科技树 |
| SettingsPanel | `SettingsIcon` | API/成本/LAN/安全/语言/About |

---

## 💝 支持项目

Chronos-Shadow 是免费开源软件。如果您觉得它有用，欢迎通过以下方式支持：

- ⭐ [GitHub Stars](https://github.com/cnkevinbai/Chronos-Shadow) — 免费的鼓励
- 💰 [GitHub Sponsors](https://github.com/sponsors/cnkevinbai) — 月度赞助
- ☕ [爱发电](https://afdian.com/a/chronos-shadow) — 一次性支持

每一份支持都帮助我们持续开发和维护这个项目。感谢！

## 开源协议

本项目基于 [Apache License 2.0](LICENSE) 发布。

Copyright 2026 Chronos-Shadow Open Source Team.

---

## 致谢

- [Tauri](https://tauri.app) — 轻量级桌面框架
- [React](https://react.dev) — UI 框架
- [Tailwind CSS](https://tailwindcss.com) — 样式系统
- [DeepSeek](https://deepseek.com) · [Kimi](https://kimi.moonshot.cn) · [GLM](https://bigmodel.cn) — 大模型 API
