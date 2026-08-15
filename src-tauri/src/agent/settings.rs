// 应用设置持久化 (App Settings)
// config.json 读写 + API 密钥 vault 回退 + 密钥脱敏（不落盘明文）

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    version: u32,
    pub cost_cap: f64,
    pub cost_cap_enabled: bool,
    ollama_url: String,
    lan_model: String,
    lan_timeout: u32,
    auto_fallback: bool,
    max_healing: u32,
    ast_audit: bool,
    block_gpl: bool,
    privacy_blur: bool,
    caching_priority: bool,
    pub accumulated_cost: f64,
    api_key_deepseek: String,
    api_key_kimi: String,
    api_key_glm: String,
    /// Vault presence flags — set by load_settings after keyring restore
    #[serde(default)]
    has_key_deepseek: bool,
    #[serde(default)]
    has_key_kimi: bool,
    #[serde(default)]
    has_key_glm: bool,
    #[serde(default = "default_project")]
    current_project: String,
}

fn default_project() -> String { "Chronos-Core-Demo".into() }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            cost_cap: 5.0,
            cost_cap_enabled: true,
            ollama_url: "http://localhost:11434".into(),
            lan_model: "deepseek-v4-flash".into(),
            lan_timeout: 3500,
            auto_fallback: true,
            max_healing: 3,
            ast_audit: true,
            block_gpl: true,
            privacy_blur: true,
            caching_priority: true,
            accumulated_cost: 0.0,
            api_key_deepseek: String::new(),
            api_key_kimi: String::new(),
            api_key_glm: String::new(),
            has_key_deepseek: false,
            has_key_kimi: false,
            has_key_glm: false,
            current_project: "Chronos-Core-Demo".into(),
        }
    }
}

static SETTINGS: std::sync::Mutex<Option<AppSettings>> = std::sync::Mutex::new(None);
pub(crate) static CONFIG_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn get_config_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    let mut guard = CONFIG_DIR.lock().unwrap();
    if let Some(ref dir) = *guard {
        return dir.clone();
    }
    let dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    *guard = Some(dir.clone());
    dir
}

pub(crate) fn ensure_settings_loaded() -> AppSettings {
    let mut guard = SETTINGS.lock().unwrap();
    if let Some(ref s) = *guard {
        return s.clone();
    }
    let dir_guard = CONFIG_DIR.lock().unwrap();
    let dir = dir_guard.as_ref().cloned().unwrap_or_else(|| PathBuf::from("."));
    drop(dir_guard);
    let path = dir.join("config.json");
    let loaded = if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str::<AppSettings>(&data).unwrap_or_default()
    } else {
        AppSettings::default()
    };
    *guard = Some(loaded.clone());
    loaded
}

#[tauri::command]
pub fn load_settings(app_handle: tauri::AppHandle) -> AppSettings {
    get_config_dir(&app_handle); // cache the dir
    let mut settings = ensure_settings_loaded();
    // Try to restore keys from Windows Credential Manager vault
    let vault = crate::agent::security_vault::NativeSecurityVault::new();
    if settings.api_key_deepseek.is_empty() || settings.api_key_deepseek == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("deepseek") {
            if !key.is_empty() { settings.has_key_deepseek = true; settings.api_key_deepseek = key; }
        }
    } else if !settings.api_key_deepseek.is_empty() {
        settings.has_key_deepseek = true;
    }
    if settings.api_key_kimi.is_empty() || settings.api_key_kimi == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("kimi") {
            if !key.is_empty() { settings.has_key_kimi = true; settings.api_key_kimi = key; }
        }
    } else if !settings.api_key_kimi.is_empty() {
        settings.has_key_kimi = true;
    }
    if settings.api_key_glm.is_empty() || settings.api_key_glm == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("glm") {
            if !key.is_empty() { settings.has_key_glm = true; settings.api_key_glm = key; }
        }
    } else if !settings.api_key_glm.is_empty() {
        settings.has_key_glm = true;
    }
    // Strip actual key values before sending to frontend
    settings.api_key_deepseek = String::new();
    settings.api_key_kimi = String::new();
    settings.api_key_glm = String::new();
    settings
}

#[tauri::command]
pub fn save_settings(app_handle: tauri::AppHandle, new_settings: AppSettings) -> Result<String, String> {
    let dir = get_config_dir(&app_handle);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("config.json");

    // Security: vault API keys to Windows Credential Manager, never write plaintext to disk
    let vault = crate::agent::security_vault::NativeSecurityVault::new();
    let mut disk_settings = new_settings.clone();
    if !new_settings.api_key_deepseek.is_empty() {
        crate::agent::key_vault::cache_key("deepseek", &new_settings.api_key_deepseek);
        let _ = vault.vault_api_key_native("deepseek", &new_settings.api_key_deepseek);
    }
    if !new_settings.api_key_kimi.is_empty() {
        crate::agent::key_vault::cache_key("kimi", &new_settings.api_key_kimi);
        let _ = vault.vault_api_key_native("kimi", &new_settings.api_key_kimi);
    }
    if !new_settings.api_key_glm.is_empty() {
        crate::agent::key_vault::cache_key("glm", &new_settings.api_key_glm);
        let _ = vault.vault_api_key_native("glm", &new_settings.api_key_glm);
    }
    // Mask keys before writing to disk — only store presence flag
    disk_settings.api_key_deepseek = if new_settings.api_key_deepseek.is_empty() { String::new() } else { "[stored in vault]".into() };
    disk_settings.api_key_kimi = if new_settings.api_key_kimi.is_empty() { String::new() } else { "[stored in vault]".into() };
    disk_settings.api_key_glm = if new_settings.api_key_glm.is_empty() { String::new() } else { "[stored in vault]".into() };

    let json = serde_json::to_string_pretty(&disk_settings).map_err(|e| e.to_string())?;
    // 原子化写入: temp + rename, 防止崩溃时配置文件损坏
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    *SETTINGS.lock().unwrap() = Some(new_settings);
    Ok(format!("Saved to {}", path.display()))
}
