// Chronos-Shadow 密钥保管箱子系统
// 内存缓存 → 文件持久化(base64) → Windows 凭据管理器回退

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// In-memory key cache — survives even if Windows Credential Manager is unavailable
static KEY_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Store key in memory cache AND persist to file as reliable fallback
pub fn cache_key(provider: &str, key: &str) {
    if let Ok(mut cache) = KEY_CACHE.lock() {
        cache.insert(provider.to_string(), key.to_string());
    }
    // Also persist to file — reliable cross-restart storage
    let _ = save_key_file(provider, key);
}

/// File-based key persistence (base64) — reliable fallback when keyring is unavailable
fn key_file_path() -> PathBuf {
    let dir = crate::CONFIG_DIR.lock().unwrap();
    dir.as_ref().cloned().unwrap_or_else(|| PathBuf::from("."))
        .join(".chronos_keys")
}

fn save_key_file(provider: &str, key: &str) -> std::io::Result<()> {
    let path = key_file_path();
    let mut map: HashMap<String, String> = if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    };
    map.insert(provider.to_string(), simple_encode(key));
    std::fs::write(&path, serde_json::to_string(&map).unwrap_or_default())
}

pub fn load_key_file(provider: &str) -> Option<String> {
    let path = key_file_path();
    if !path.exists() { return None; }
    let data = std::fs::read_to_string(&path).ok()?;
    let map: HashMap<String, String> = serde_json::from_str(&data).ok()?;
    map.get(provider).map(|v| simple_decode(v))
}

fn simple_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

fn simple_decode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s)
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
}

/// Resolve API key: memory cache → file → Windows Credential Manager vault
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

    // 2. Try file-based persistence (reliable cross-restart)
    if let Some(key) = load_key_file(target) {
        if !key.is_empty() {
            tracing::info!("[VAULT] Key resolved from file for '{}'", target);
            // Restore to memory cache
            if let Ok(mut cache) = KEY_CACHE.lock() {
                cache.insert(target.to_string(), key.clone());
            }
            return key;
        }
    }

    // 3. Fall back to Windows Credential Manager
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
