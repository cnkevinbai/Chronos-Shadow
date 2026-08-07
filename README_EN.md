# Chronos-Shadow (Shadow of Time)

> Next-generation industrial-grade open-source desktop agent — deeply integrating LLM capabilities with Windows system-level control

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-v2.0-purple.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19.0-blue.svg)](https://react.dev)

Chronos-Shadow is an industrial-grade open-source desktop agent that deeply integrates LLM capabilities with Windows system-level control. Built-in MCP client bus, adaptive distributed cluster management, AES-256-GCM session encryption, native Windows Credential Manager key storage, zero-token local skill detection engine, and gamified cost-saving ledger.

> 🚧 **Development Status**: ONNX privacy masking, Buddy Scan visual inspection, and vector embeddings are currently framework placeholders under active development. See [CHANGELOG.md](CHANGELOG.md) for details.

---

## Feature Matrix

### 🧠 Multi-Model Hybrid Routing
- Support: DeepSeek V4-Pro/Flash, Kimi K3/K2.7, GLM-5.2/5V-Turbo
- Auto-routing: selects optimal model per Agent role (PM/Coder/Auditor)
- LAN hot-swap: cloud timeout → millisecond fallback to Ollama local model
- DeepSeek Context Caching maximized at 90% discount

### 🔒 Financial-Grade Security
- **AES-256-GCM** streaming session encryption (hardware-fingerprint-bound key)
- **Windows Credential Manager** native FFI (`keyring` → `CredWriteW/CredReadW`)
- Strict CSP · Tauri permission whitelist · rate limiting · cost circuit breaker
- Three Red Lines anti-hallucination: Schema validation + Sandbox path blocking + Healing fuse

### 🖥️ Remote Server Cluster
- SSH tunnel · file browser · remote compilation · Git snapshot/rewind
- Multi-server async registration · project→server mapping · cluster ping

### 🎨 OmniDesign-Matrix
- Natural language → cross-platform UI/UX code (PC + Mobile)
- Vercel / Linear / Apple theme one-click switching
- Dual-pane live preview + ONNX pixel-level fidelity inspection 🚧

### 📊 Full-Role SDLC Pipeline
- 7 Agent ring scheduling (PM → Designer → Architect → Planner → Coder → Auditor → Verifier)
- Task Kanban: create/assign/complete/fail
- Zero-Token local skill detection & interception
- 🚧 Buddy Scan pixel-level visual inspection (in development)
- 🚧 ONNX local privacy masking (in development)

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Desktop** | Tauri v2 (Rust) |
| **Frontend** | React 19 + TypeScript + Tailwind CSS 4 + Vite |
| **Backend** | Rust (23 modules, 10,600+ lines) |
| **Crypto** | AES-256-GCM + SHA-256 + keyring (Windows FFI) |
| **AI Routing** | reqwest + SSE streaming + Context Caching |
| **Storage** | Chunked JSON (Chronos Vault) |
| **Testing** | Rust `#[cfg(test)]` + TypeScript strict mode |

---

## Quick Start

### Prerequisites
- **Windows 10/11** (WebView2 built-in)
- [Rust](https://rustup.rs) 1.80+
- [Node.js](https://nodejs.org) 22+
- [Tauri CLI](https://tauri.app) v2

### Development

```bash
cd chronos-shadow
npm install
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/msi/Chronos-Shadow_*.msi
```

### Portable Edition

Run `chronos-shadow.exe` directly (9.3 MB), no installation required.

---

## Project Structure

```
chronos-shadow/
├── src/                    # React frontend
│   ├── views/              # 9 panels
│   ├── components/         # 11 components (incl. SvgIcons)
│   └── lib/                # IPC layer + types + i18n
├── src-tauri/              # Rust backend
│   ├── src/agent/          # 23 core modules
│   ├── skills/             # Skill definitions
│   └── resources/          # ONNX models + MCP configs
├── .github/workflows/      # CI/CD
└── dist/                   # Frontend build output
```

---

## Panels

| Panel | Icon | Function |
|-------|------|----------|
| ChatPanel | `ChatIcon` | Streaming chat · attachments · @/slash · Markdown |
| SdlcPipelinePanel | `PipelineIcon` | 7 Agent ring · task Kanban |
| AppGlueBinder | `GlueIcon` | Cross-app glue · OmniDesign canvas |
| SkillMcpHub | `McpIcon` | Skill management · MCP connect |
| RemoteHub | `RemoteIcon` | SSH cluster · file browse · compile |
| ProjectExplorer | `ChronosFolderIcon` | C-VFS projects · Chronos timeline |
| SecurityShield | `ShieldIcon` | Shadow toggle · fuse reset |
| RedlineGuard | `ShieldIcon` | Three Red Lines · schema test |
| EvolutionConsole | `EvolutionIcon` | Evolution system · tech tree |
| SettingsPanel | `SettingsIcon` | API/cost/LAN/security/lang/About |

---

## 💝 Support the Project

Chronos-Shadow is free and open-source. If you find it useful, support us via:

- ⭐ [GitHub Stars](https://github.com/cnkevinbai/Chronos-Shadow) — free encouragement
- 💰 [GitHub Sponsors](https://github.com/sponsors/cnkevinbai) — monthly sponsorship
- ☕ [Afdian](https://afdian.com/a/chronos-shadow) — one-time support

Every contribution helps us continue developing and maintaining this project. Thank you!

## License

This project is released under [Apache License 2.0](LICENSE).

Copyright 2026 Chronos-Shadow Open Source Team.

---

## Acknowledgments

- [Tauri](https://tauri.app) — Lightweight desktop framework
- [React](https://react.dev) — UI framework
- [Tailwind CSS](https://tailwindcss.com) — Styling system
- [DeepSeek](https://deepseek.com) · [Kimi](https://kimi.moonshot.cn) · [GLM](https://bigmodel.cn) — LLM APIs
