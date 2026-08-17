// 推理深度 — 控制大模型的思考深度与输出长度
//
// Low:    浅推理（快速精简，max_tokens 2048 / temperature 0.7）
// Medium: 中推理（默认，max_tokens 4096 / temperature 0.3）
// High:   深推理（深度思考、高确定性，max_tokens 8192 / temperature 0.1）
//
// 通过 max_tokens + temperature 两个真实 OpenAI 兼容参数落地，
// 不依赖任何厂商私有 API，杜绝技术幻觉。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningDepth {
    Low,
    Medium,
    High,
}

impl ReasoningDepth {
    /// 该深度对应的最大输出 Token 数（更多 token = 更深的推理空间）
    pub fn max_tokens(&self) -> u32 {
        match self {
            ReasoningDepth::Low => 2048,
            ReasoningDepth::Medium => 4096,
            ReasoningDepth::High => 8192,
        }
    }

    /// 该深度对应的采样温度（更低 = 更确定、更深）
    pub fn temperature(&self) -> f32 {
        match self {
            ReasoningDepth::Low => 0.7,
            ReasoningDepth::Medium => 0.3,
            ReasoningDepth::High => 0.1,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningDepth::Low => "low",
            ReasoningDepth::Medium => "medium",
            ReasoningDepth::High => "high",
        }
    }

    pub fn parse(s: &str) -> ReasoningDepth {
        match s.trim().to_lowercase().as_str() {
            "low" => ReasoningDepth::Low,
            "high" => ReasoningDepth::High,
            _ => ReasoningDepth::Medium,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ReasoningDepth::Low => "浅推理 Low",
            ReasoningDepth::Medium => "中推理 Medium",
            ReasoningDepth::High => "深推理 High",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ReasoningDepth::Low => "快速精简，输出较短",
            ReasoningDepth::Medium => "平衡速度与深度（默认）",
            ReasoningDepth::High => "深度思考，高确定性，输出较长",
        }
    }
}

impl Default for ReasoningDepth {
    fn default() -> Self {
        ReasoningDepth::Medium
    }
}

#[tauri::command]
pub async fn get_reasoning_depth(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    Ok(state.api_client.lock().await.reasoning_depth.as_str().to_string())
}

#[tauri::command]
pub async fn set_reasoning_depth(
    state: tauri::State<'_, crate::state::AppState>,
    depth: String,
) -> Result<String, String> {
    let d = ReasoningDepth::parse(&depth);
    state.api_client.lock().await.reasoning_depth = d;
    tracing::info!("[DEPTH] Reasoning depth set to: {}", d.as_str());
    Ok(d.as_str().to_string())
}
