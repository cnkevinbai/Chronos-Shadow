// Tauri IPC 命令层（web_intel_* / distill_*）
use super::types::*;
use tauri::Manager;

#[tauri::command]
pub async fn web_intel_search(
    state: tauri::State<'_, crate::state::AppState>,
    query: String,
    engine: Option<String>,
    max_results: Option<u32>,
) -> Result<Vec<WebSearchResult>, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.search(&query, engine.as_deref(), Some(max_results.unwrap_or(5))).await
}

#[tauri::command]
pub async fn web_intel_fetch(
    state: tauri::State<'_, crate::state::AppState>,
    url: String,
    distill: Option<bool>,
) -> Result<WebFetchResult, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.fetch(&url, distill.unwrap_or(true)).await
}

#[tauri::command]
pub async fn web_intel_research(
    state: tauri::State<'_, crate::state::AppState>,
    topic: String,
    sources: Option<Vec<String>>,
) -> Result<ResearchReport, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.research(&topic, sources.unwrap_or_default()).await
}

#[tauri::command]
pub async fn web_intel_add_domain(
    state: tauri::State<'_, crate::state::AppState>,
    domain: String,
    category: Option<String>,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.add_allowed_domain(&domain, category.as_deref().unwrap_or("custom"))
        .map(|()| format!("Domain {} added", domain))
}

#[tauri::command]
pub async fn web_intel_remove_domain(
    state: tauri::State<'_, crate::state::AppState>,
    domain: String,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.remove_allowed_domain(&domain)
        .map(|()| format!("Domain {} removed", domain))
}

#[tauri::command]
pub async fn web_intel_list_domains(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<(String, String)>, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.list_allowed_domains())
}

#[tauri::command]
pub async fn web_intel_get_audit_log(
    state: tauri::State<'_, crate::state::AppState>,
    limit: Option<usize>,
) -> Result<Vec<WebAuditEntry>, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.get_audit_log_owned(limit.unwrap_or(50)))
}

#[tauri::command]
pub async fn web_intel_get_stats(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<WebIntelStats, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.get_stats())
}

#[tauri::command]
pub async fn web_intel_save_state(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let wi = state.web_intelligence.lock().await;
    wi.save_state(&dir)
}

#[tauri::command]
pub async fn web_intel_load_state(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut wi = state.web_intelligence.lock().await;
    wi.load_state(&dir)
}

#[tauri::command]
pub async fn distill_evolution_report(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<serde_json::Value, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.distillation.evolution_report())
}

#[tauri::command]
pub async fn distill_feedback(
    state: tauri::State<'_, crate::state::AppState>,
    url: String,
    quality_score: f64,
    content_type: Option<String>,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.distillation.feedback(&url, quality_score, &content_type.unwrap_or_else(|| "documentation".into()));
    Ok(format!("Feedback recorded for {}: quality={:.2}", url, quality_score))
}