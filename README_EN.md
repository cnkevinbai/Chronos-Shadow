# Chronos-Shadow

> Next-gen Industrial Open-Source Desktop Agent — Deeply integrating LLM potential with Windows system-level control

[**🇨🇳 中文**](README.md) | [**🇬🇧 English**](README_EN.md)

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-v2.0-purple.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19.0-blue.svg)](https://react.dev)

Chronos-Shadow is an industrial-grade open-source desktop agent that deeply integrates LLM capabilities with Windows system-level control. Features 41 Rust backend modules, 12 frontend panels, multi-model hybrid routing, on-device evolution engine cluster, AES-256-GCM session encryption, and a full-chain security approval system.

> ✅ **v0.3.0 Released** — Evolution persistence, dynamic metrics, real MCP servers, security hardening, i18n auto-detection, and a full `lib.rs` command refactor. See [CHANGELOG.md](CHANGELOG.md).

### 🚀 What's New in v0.3.0
- **Evolution persistence**: learned skills / memory pool / embeddings survive restart
- **Dynamic metrics**: token / complexity / cost / risk estimation from the actual task text
- **Real MCP servers**: 3 Node.js stdio servers (audit-vault / win32-registry / local-vector-glue) auto-registered at startup
- **Security hardening**: path traversal ×2, SSRF, plaintext key persistence, terminal whitelist, GCM nonce reuse
- **i18n**: browser-language auto-detection + fully localized command palette
- **Frontend testing**: Vitest + React Testing Library (29 tests)
- **Codebase refactor**: 53 Tauri commands migrated out of `lib.rs` into 30+ agent modules

---

## Features

### 🧠 Multi-Model Hybrid Routing
- DeepSeek V4-Pro/Flash, Kimi K3/K2.7, GLM-5.2/5V-Turbo
- Auto-routing: 27 keyword rules → optimal Agent + Model
- Multi-model collaboration: Parallel / Voting / Cascade / Divide-and-Conquer
- LAN hot-swap: Cloud timeout → millisecond fallback to Ollama local
- DeepSeek Context Caching 90% discount maximization
- Quality feedback loop: EMA model profile updates + auto-degradation

### 🔒 Financial-Grade Security
- AES-256-GCM session streaming encryption (hardware-fingerprint key binding)
- Windows Credential Manager native FFI
- CSP strict policy · Tauri permission whitelist · Rate limiting · Cost circuit breaker
- Four redline protection: Schema validation + Sandbox path lock + Self-healing fuse + Approval gate
- 10-dimension anti-hallucination engine with adaptive sensitivity
- Domain whitelist: read-only external search/fetch, data exfiltration permanently forbidden

### 🌐 Web Intelligence (v0.2.0)
- Domain whitelist: 30+ preloaded official docs + tech community domains
- Web search: Bing / DuckDuckGo dual engines with result caching
- Web fetch: HTML→Markdown auto-conversion with content sanitization
- Multi-source research: Search→Fetch→Distill→Summarize pipeline
- Full audit trail for all external requests

### 🧪 Semantic Distillation Engine (v0.2.0)
- Three-level: Light / Medium / Deep
- 7-dimension RL weight auto-evolution
- 12 entity types auto-extraction: versions, dates, crates, packages, repos, licenses
- Adaptive strategy table per content type (code/doc/blog)
- LRU cache + quality feedback loop

### 🔄 Unified Cache Engine (v0.2.0)
- 5 categories with independent TTLs
- Adaptive TTL: adjusts based on hit rate patterns
- LRU eviction + disk persistence
- Full-dimension statistics

### 🤖 Multi-Model Collaboration (v0.2.0)
- 4 collaboration modes with 5 model profiles
- EMA quality feedback + auto fallback
- Cost optimization: cheapest model above quality threshold

### 🔮 Predictive Analytics (v0.2.0)
- Holt-Winters seasonal token forecasting
- SPC cost anomaly detection
- Bayesian budget optimization
- K-means usage pattern clustering

### 🧬 Evolution Bus (v0.2.0)
- 9-engine unified evolution management
- Feedback loop with safety clamp protection
- Cross-engine knowledge transfer
- Hourly advancement assessment

---

## Tech Stack

| Layer | Technology | Scale |
|-------|-----------|-------|
| Desktop | Tauri v2 (Rust) | — |
| Frontend | React 19 + TS + Tailwind CSS 4 + Vite | 1817 modules, 508 KB JS |
| Backend | Rust | 41 modules, ~936 KB source |
| Crypto | AES-256-GCM + SHA-256 + keyring | — |
| AI Routing | reqwest + SSE streaming + Context Caching | — |
| Storage | Chunked JSON (Chronos Vault) | — |
| i18n | Chinese + English | 369 keys |

---

## Quick Start

### Prerequisites
- Windows 10/11 (WebView2 built-in)
- Rust 1.80+
- Node.js 22+
- Tauri CLI v2

### Development

```bash
cd chronos-shadow
npm install
npx tauri dev
```

### Production Build

```bash
build-tauri.bat
# Output:
#   src-tauri/target/release/chronos-shadow.exe  (11.7 MB)
#   src-tauri/target/release/bundle/msi/*.msi    (5.7 MB)
```

---

## Architecture

```
Security Layer → security_boundary + redline + approval_gate + hallucination_guard
Intelligence Layer → scheduling_engine + task_intelligence + collaboration + predictive
Information Layer → web_intelligence + distillation + cache_engine
Evolution Layer → evolution_bus + agent_quality
Infrastructure Layer → router + billing_engine + orchestrator
```

---

## License

Apache License 2.0

Copyright 2026 Chronos-Shadow Open Source Team.