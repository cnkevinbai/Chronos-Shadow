# Chronos-Shadow Security Policy

## Security Architecture

Chronos-Shadow employs a **five-layer defense-in-depth** security architecture:

### Layer 1: Permission Boundary (`security_boundary.rs`)
- **6 operation categories** with granular permission levels:
  - 🚫 Forbidden: DeleteProject, DeleteDatabase, DataExfiltration, SocialPostWrite, DataUploadExternal, SystemModification
  - 🛡️ RequireApproval: WebSearch, WebFetchReadonly, ApiCallReadonly, WorktreeMerge, PipelineAdvance, RemoteCommand, CostOverride, ConfigChange
  - ⚠️ RequireConfirmation: FileDelete, SessionDelete, CheckpointDelete
  - 📖 SandboxReadOnly: FileRead, ProjectList, SessionRead
  - ✏️ SandboxReadWrite: FileWrite, CodeGeneration, CheckpointCreate
  - 🟢 Allowed: ChatMessage, StatusQuery
- **Deny-by-Default**: All external network operations require explicit approval
- **Domain Whitelist**: Only pre-approved domains accessible for search/fetch

### Layer 2: Redline Guard (`redline.rs`)
- **Redline 1**: JSON Schema strict validation — all LLM outputs must conform to `AgentAction` schema
- **Redline 2**: Sandbox path whitelist — file operations locked within project root
- **Redline 3**: Self-healing circuit breaker — max 3 consecutive heal attempts before fuse
- WebSearch/WebFetch: HTTPS enforced, URL format validated, SQL injection detected

### Layer 3: Hallucination Guard (`hallucination_guard.rs`)
- **10-dimension detection**:
  1. Confidence markers (uncertainty language)
  2. Fake APIs (invented libraries/functions)
  3. Code consistency (mismatched braces, undeclared variables)
  4. Internal contradictions (self-contradicting statements)
  5. Dangerous commands (rm -rf, DROP DATABASE)
  6. Outdated references (deprecated tech stacks)
  7. Fake programming (TODO stubs, pseudocode)
  8. Fake completion (claims done without substance)
  9. Empty scaffold (mkdir without file creation)
  10. Fabricated facts (benchmark data, version numbers, authority claims)
- **Adaptive sensitivity**: False positive feedback loop with RL-style weight adjustment

### Layer 4: Approval Gate (`approval_gate.rs`)
- **4-dimension risk scoring**: Impact Scope × Reversibility × Cost Impact × Compliance
- Auto-approval for low-risk operations (below configured threshold)
- Web search/fetch: low risk profile (impact:1, reversible:10, cost:2)
- Auditor pre-screening for high-risk operations

### Layer 5: Content Safety (`web_intelligence.rs`)
- **Request sanitization**: API keys, file paths, personal info automatically redacted
- **Response distillation**: Web content distilled on-device before reaching LLM context
- **Full audit trail**: All external requests logged with timestamp, target, result
- **HTTPS-only**: Plain HTTP requests rejected at schema level

## Encryption

- **AES-256-GCM** session encryption with hardware fingerprint key binding
- **SHA-256** chain hashing for context caching markers
- **Windows Credential Manager** native FFI for API key storage

## Reporting a Vulnerability

Please report security vulnerabilities to [GitHub Issues](https://github.com/cnkevinbai/Chronos-Shadow/issues) with the `security` label.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.0   | ✅ Active          |
| 0.1.1   | ❌ End of life     |