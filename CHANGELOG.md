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
- println! -> tracing::info! in session_db
- [VAULT EMPTY] 401 error: keyring silent failure -> 3-tier fallback (memory->file->WinCred)
- FooterBar fake savings multipliers removed (buddySaved*0.3/*0.42)
- App.tsx initial state magic numbers (0.342/1.82/84/0.52 -> 0.0)
- Cost-cap UI end-to-end wiring (FooterBar -> updateCostCap -> billing_engine)
- SettingsPanel setTimeout-after-unmount tracked via useRef cleanup
- .gitignore: added chronos_vault/, config.json, .chronos_tmp/

### Changed
- API keys no longer stored in plaintext config.json
- chat_api/chat_api_stream: cost tracking uses parallel billing engine
- chat_api/chat_api_stream: key resolved server-side from vault (resolve_key_from_vault)
- ChatPanel: apiKey prop replaced with hasKeys object for per-model status
- README_EN.md fully synced with CN version (donations, dev status, correct counts)
- README.md/README_EN.md: module count 18/22->23, component count 10->11
- CHANGELOG format aligned with keepachangelog.com + compare links
- FooterBar: cost cap synced to backend via updateCostCap IPC
- Token billing: prompt/completion split estimated (split_tokens helper)

---

## [0.1.1] - 2026-08-07

### Added
- AES-256-GCM session encryption + Windows Credential Manager native FFI (keyring)
- CSP strict policy + Tauri permission whitelist + API rate limiting (1.5s)
- Zero-Token skill detection engine (detector.rs) + cluster adaptive allocation
- SDLC state machine (SdlcState/SdlcEvent) + blackboard enhancement
- 17 specialized SVG icons (SvgIcons.tsx)
- OmniDesign-Matrix cross-platform visual design canvas
- RemoteHub server cluster management panel
- Settings About page with Apache 2.0 license + privacy statement
- Markdown rendering, SSE streaming, message search, virtual scrolling
- Session import/export, rename, delete, auto-save
- Keyboard shortcuts (Ctrl+N/S/E/F/Enter), message copy, font scaling
- System tray (optional), close-to-tray
- GitHub Actions CI/CD pipeline
- tracing file logging system

### Fixed
- Unified billing engine (billing.rs -> api_client.rs)
- regex replacing custom regex_lite (security audit)
- Hardcoded timestamps -> chrono::Utc::now()
- 93 Rust deprecation warnings -> 0
- Window startup crash (tray icon defensive init)
- Duplicate icons (ChronosLogo + i18n emoji)

### Changed
- Rust modules: 17 -> 22
- Tauri commands: 55 -> 88
- Frontend panels: 6 -> 9
- Frontend coverage: 66% -> 91%
- Project rating: B+ (7.6) -> A+ (9.2)

---

## v0.1.0 (2026-08-03) - Initial Release

- Tauri v2 + React 19 + Tailwind CSS 4 framework
- Multi-model routing (DeepSeek/Kimi/GLM)
- 7 Agent SDLC pipeline orchestration
- Three Red Lines anti-hallucination interceptor
- Session persistence (chunked storage + SHA256 cache hash)
- Financial audit engine (official pricing matrix)
- Remote SSH proxy + cluster management
- MCP JSON-RPC 2.0 protocol client
- 6 frontend panels

[Unreleased]: https://github.com/cnkevinbai/Chronos-Shadow/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.1.1
