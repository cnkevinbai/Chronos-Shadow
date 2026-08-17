// 七核心引擎 v2 能力的统一 IPC 出口 (engine_v2)
//
// 汇集 v0.5.0 升级的 7 个引擎新能力的 Tauri command，供前端面板调用。
// 蒸馏引擎 + 调度引擎的方法为无状态，on-demand 创建引擎实例；
// 其余引擎挂载在 AppState 中。

use crate::state::AppState;

/// 任务智能 v2：科学化工作量估算（PERT + 风险 + 关键路径）
#[tauri::command]
pub fn task_estimate_effort(
    state: tauri::State<'_, AppState>,
    task: String,
) -> Result<serde_json::Value, String> {
    let engine = state.task_intelligence.lock().unwrap();
    let effort = engine.estimate_effort(&task);
    serde_json::to_value(&effort).map_err(|e| e.to_string())
}

/// 多模型协作 v2：UCB 探索-利用模型选择
#[tauri::command]
pub fn collab_select_model_ucb(
    state: tauri::State<'_, AppState>,
    task_type: String,
    exploration_weight: Option<f64>,
) -> Result<serde_json::Value, String> {
    let engine = state.collaboration.blocking_lock();
    let model = engine.select_best_model_ucb(&task_type, exploration_weight.unwrap_or(2.0));
    Ok(serde_json::json!({ "task_type": task_type, "model": model }))
}

/// 预测分析 v2：CUSUM 变化点检测
#[tauri::command]
pub fn predictive_detect_change_points(
    state: tauri::State<'_, AppState>,
    data: Vec<f64>,
    threshold: f64,
    drift: f64,
) -> Result<Vec<serde_json::Value>, String> {
    let engine = state.predictive.lock().unwrap();
    Ok(engine.detect_change_points(&data, threshold, drift))
}

/// 进化总线 v2：系统化健康自评估
#[tauri::command]
pub fn evobus_self_assess(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bus = state.evolution_bus.lock().unwrap();
    serde_json::to_value(&bus.self_assess()).map_err(|e| e.to_string())
}

/// 蒸馏引擎 v2：实体关系图提取
#[tauri::command]
pub fn distill_entity_relations(
    _state: tauri::State<'_, AppState>,
    content: String,
) -> Result<serde_json::Value, String> {
    let engine = crate::agent::distillation_engine::DistillationEngine::new();
    let relations = engine.extract_entity_relations(&content);
    serde_json::to_value(&relations).map_err(|e| e.to_string())
}

/// Web 智能 v2：相关性重排序
#[tauri::command]
pub fn web_rerank_results(
    state: tauri::State<'_, AppState>,
    query: String,
    results_json: String,
) -> Result<serde_json::Value, String> {
    let results: Vec<crate::agent::web_intelligence::WebSearchResult> =
        serde_json::from_str(&results_json).map_err(|e| e.to_string())?;
    let engine = state.web_intelligence.blocking_lock();
    let reranked = engine.rerank_results(&query, &results);
    serde_json::to_value(&reranked).map_err(|e| e.to_string())
}

/// 调度引擎 v2：上下文感知路由
#[tauri::command]
pub fn scheduling_analyze_with_context(
    _state: tauri::State<'_, AppState>,
    message: String,
    context: Vec<String>,
) -> Result<serde_json::Value, String> {
    let engine = crate::agent::scheduling_engine::AgentSchedulingEngine::new();
    Ok(engine.analyze_with_context(&message, &context))
}
