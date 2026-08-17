// 智能体运行模式 — 控制自主级别与安全校验强度
//
// Plan:   只生成计划，不执行任何动作（最安全）
// Review: 每个动作执行前都需人工审批
// Auto:   低风险自动执行、高风险审批（默认，现有行为）
// Yolo:   跳过所有红线/安全边界/审批，直接执行（最快、最危险）
//
// 与现有「四红线 + 审批门禁」架构融合：
//   - Auto  = 现有行为（红线校验 + 安全边界 + 外网动作审批）
//   - Review = Auto 基础上对所有动作强制审批
//   - Plan   = 只返回计划，阻断执行
//   - Yolo   = 关闭全部校验（仅用于可信环境 / 用户明确授权）

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentMode {
    Plan,
    Review,
    Auto,
    Yolo,
}

impl AgentMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentMode::Plan => "plan",
            AgentMode::Review => "review",
            AgentMode::Auto => "auto",
            AgentMode::Yolo => "yolo",
        }
    }

    pub fn parse(s: &str) -> AgentMode {
        match s.trim().to_lowercase().as_str() {
            "plan" => AgentMode::Plan,
            "review" => AgentMode::Review,
            "yolo" => AgentMode::Yolo,
            _ => AgentMode::Auto,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgentMode::Plan => "计划 Plan",
            AgentMode::Review => "审查 Review",
            AgentMode::Auto => "自动 Auto",
            AgentMode::Yolo => "YOLO",
        }
    }

    /// 模式说明（供前端提示）
    pub fn description(&self) -> &'static str {
        match self {
            AgentMode::Plan => "只生成执行计划，不执行任何动作",
            AgentMode::Review => "每个动作执行前需人工审批",
            AgentMode::Auto => "低风险自动执行，高风险审批",
            AgentMode::Yolo => "跳过所有安全校验直接执行（危险）",
        }
    }
}

impl Default for AgentMode {
    fn default() -> Self {
        AgentMode::Auto
    }
}

#[tauri::command]
pub fn get_agent_mode(state: tauri::State<crate::state::AppState>) -> String {
    state.agent_mode.lock().unwrap().as_str().to_string()
}

#[tauri::command]
pub fn set_agent_mode(state: tauri::State<crate::state::AppState>, mode: String) -> String {
    let m = AgentMode::parse(&mode);
    *state.agent_mode.lock().unwrap() = m;
    tracing::info!("[MODE] Agent mode set to: {}", m.as_str());
    m.as_str().to_string()
}
