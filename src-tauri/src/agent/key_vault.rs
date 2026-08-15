// Chronos-Shadow 密钥保管箱子系统
// 内存缓存 → Windows 凭据管理器 (CredWriteW/CredReadW)
//
// 安全约束：API Key 绝不落盘明文（含 base64 等价物）——
// 密钥仅驻留内存缓存与 Windows 内核凭据保险箱。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// In-memory key cache — survives even if Windows Credential Manager is unavailable
static KEY_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Store key in memory cache (only). The authoritative store is Windows
/// Credential Manager (security_vault); the in-memory cache is a same-session
/// fast path. Keys are never written to disk in any reversible form.
pub fn cache_key(provider: &str, key: &str) {
    if let Ok(mut cache) = KEY_CACHE.lock() {
        cache.insert(provider.to_string(), key.to_string());
    }
    // 迁移清理：删除旧版本遗留的明文 base64 密钥文件（安全加固）
    purge_legacy_key_file();
}

/// 一次性删除旧版本遗留的 `.chronos_keys` 明文密钥文件。
/// 旧版本曾把 API Key 以 base64 落盘，与「零明文磁盘留存」安全模型相悖。
fn purge_legacy_key_file() {
    let dir = crate::agent::settings::CONFIG_DIR.lock().unwrap();
    let base = dir.as_ref().cloned().unwrap_or_else(|| PathBuf::from("."));
    let path = base.join(".chronos_keys");
    if path.exists() {
        let _ = std::fs::remove_file(&path);
        tracing::warn!("[VAULT] Removed legacy plaintext key file {}", path.display());
    }
}

/// Resolve API key: memory cache → Windows Credential Manager vault
pub fn resolve_key_from_vault(model: &str) -> String {
    let target = if model.contains("deepseek") { "deepseek" }
        else if model.contains("kimi") { "kimi" }
        else if model.contains("glm") { "glm" }
        else {
            tracing::warn!("[VAULT] Unknown model '{}', cannot resolve key", model);
            return String::new();
        };

    // 1. Try in-memory cache first (instant, same-session)
    if let Ok(cache) = KEY_CACHE.lock() {
        if let Some(key) = cache.get(target) {
            if !key.is_empty() {
                tracing::info!("[VAULT] Key resolved from memory cache for '{}'", target);
                return key.clone();
            }
        }
    }

    // 2. Fall back to Windows Credential Manager
    let vault = crate::agent::security_vault::NativeSecurityVault::new();
    match vault.fetch_api_key_native(target) {
        Ok(key) if !key.is_empty() => {
            tracing::info!("[VAULT] Key resolved from WinCred for '{}' — len={}", target, key.len());
            if let Ok(mut cache) = KEY_CACHE.lock() {
                cache.insert(target.to_string(), key.clone());
            }
            key
        }
        Ok(_) => {
            tracing::warn!("[VAULT] Key for '{}' is empty in WinCred — re-enter in Settings", target);
            String::new()
        }
        Err(e) => {
            tracing::error!("[VAULT] Failed to read key for '{}': {}", target, e);
            String::new()
        }
    }
}
