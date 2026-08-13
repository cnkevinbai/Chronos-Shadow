// 云端大模型 API 客户端
//
// 负责通过 HTTP 调用 DeepSeek / Kimi / GLM / OpenAI 等云端大模型 API
// 支持：流式响应、超时控制、指数退避重试、模型降级链、上下文窗口感知

use serde::{Deserialize, Serialize};
use crate::agent::billing::estimate_cost_from_model_name;

// ─── API 请求/响应类型 ────────────────────────────────────────────

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// API 调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// API 调用响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub content: String,
    pub model: String,
    pub tokens_used: u32,
    pub cost_estimate: f64,
    pub error: Option<String>,
}

/// 视觉 API 请求（多模态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRequest {
    pub model: String,
    pub messages: Vec<VisionMessage>,
    pub max_tokens: Option<u32>,
}

/// 多模态消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionMessage {
    pub role: String,
    pub content: Vec<VisionContent>,
}

/// 多模态内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VisionContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    Image { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

// ─── 重试与降级配置 ──────────────────────────────────────────────

/// 指数退避重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    /// 降级模型链 (按优先级排列，失败后依次尝试)
    pub fallback_models: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            fallback_models: vec![
                "deepseek-v4-flash".into(),
                "glm-5.1".into(),
                "ollama-local".into(),
            ],
        }
    }
}

/// 上下文窗口限制 (tokens, 保守估计: 1 token ≈ 4 chars)
const CONTEXT_WINDOW_LIMITS: &[(&str, usize)] = &[
    ("deepseek-v4-pro", 120_000),
    ("deepseek-v4-flash", 60_000),
    ("kimi-k3", 60_000),
    ("kimi-k2.7-code", 60_000),
    ("kimi-k2.7-code-highspeed", 60_000),
    ("glm-5.2", 120_000),
    ("glm-5v-turbo", 30_000),
    ("glm-5.1", 120_000),
    ("glm-4.7", 30_000),
    ("ollama-local", 8_000),
];

/// 获取模型上下文窗口大小 (tokens)
pub fn get_context_window(model: &str) -> usize {
    CONTEXT_WINDOW_LIMITS.iter()
        .find(|(prefix, _)| model.starts_with(prefix))
        .map(|(_, limit)| *limit)
        .unwrap_or(60_000)
}

/// 估算消息列表的 token 数 (粗略: 1 token ≈ 4 chars)
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| m.content.len() / 4).sum()
}

/// 智能截断上下文：当接近窗口限制时，保留 system + 最近 N 条
pub fn trim_context(mut messages: Vec<ChatMessage>, model: &str, reserve_for_output: usize) -> Vec<ChatMessage> {
    let limit = get_context_window(model).saturating_sub(reserve_for_output);
    let estimated = estimate_tokens(&messages);
    if estimated <= limit { return messages; }

    // 保留 system prompt + 从尾部开始裁剪
    let system_msg = if messages.first().map(|m| m.role.as_str()) == Some("system") {
        Some(messages.remove(0))
    } else {
        None
    };

    // 从尾部保留，直到接近限制
    let mut kept = Vec::new();
    let mut token_sum = 0;
    for msg in messages.into_iter().rev() {
        let t = msg.content.len() / 4;
        if token_sum + t > limit { break; }
        token_sum += t;
        kept.push(msg);
    }
    kept.reverse();

    if let Some(sys) = system_msg {
        kept.insert(0, sys);
    }
    kept
}

// ─── API 客户端 ────────────────────────────────────────────────────

/// 云端 API 客户端
pub struct ApiClient {
    /// HTTP 客户端（带超时和连接池）
    client: reqwest::Client,
    /// API 调用计数
    call_count: u64,
    /// 累计 Token 使用量
    total_tokens: u64,
    /// 累计预估费用
    total_cost: f64,
}

impl ApiClient {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(5)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            call_count: 0,
            total_tokens: 0,
            total_cost: 0.0,
        })
    }

    /// 调用文本大模型 API
    pub async fn chat(
        &mut self,
        endpoint: &str,
        api_key: &str,
        model: &str,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
    ) -> ApiResponse {
        self.call_count += 1;

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": 0.3,
            "stream": false,
        });

        // Auto-append standard OpenAI-compatible chat completions path
        let full_url = if endpoint.ends_with("/chat/completions") {
            endpoint.to_string()
        } else {
            format!("{}/chat/completions", endpoint.trim_end_matches('/'))
        };

        let result = self
            .client
            .post(&full_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();

                if status.is_success() {
                    // 解析 OpenAI 兼容格式
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let content = json["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or(&text)
                            .to_string();
                        let tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;
                        let cost = estimate_cost_from_model_name(model, tokens, 0);

                        self.total_tokens += tokens as u64;
                        self.total_cost += cost;

                        ApiResponse {
                            success: true,
                            content,
                            model: model.into(),
                            tokens_used: tokens,
                            cost_estimate: cost,
                            error: None,
                        }
                    } else {
                        ApiResponse {
                            success: false,
                            content: String::new(),
                            model: model.into(),
                            tokens_used: 0,
                            cost_estimate: 0.0,
                            error: Some(format!("Failed to parse response: {}", truncate_str(&text, 200))),
                        }
                    }
                } else {
                    ApiResponse {
                        success: false,
                        content: String::new(),
                        model: model.into(),
                        tokens_used: 0,
                        cost_estimate: 0.0,
                        error: Some(format!("HTTP {}: {}", status.as_u16(), truncate_str(&text, 200))),
                    }
                }
            }
            Err(e) => ApiResponse {
                success: false,
                content: String::new(),
                model: model.into(),
                tokens_used: 0,
                cost_estimate: 0.0,
                error: Some(format!("Request failed: {}", e)),
            },
        }
    }

    /// 流式调用文本大模型 API (SSE)
    /// 每收到一个 token chunk，调用 on_chunk 回调
    /// 返回完整累积内容 + token 统计
    pub async fn chat_stream(
        &mut self,
        endpoint: &str,
        api_key: &str,
        model: &str,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        mut on_chunk: impl FnMut(&str),
    ) -> ApiResponse {
        self.call_count += 1;

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": 0.3,
            "stream": true,
        });

        let full_url = if endpoint.ends_with("/chat/completions") {
            endpoint.to_string()
        } else {
            format!("{}/chat/completions", endpoint.trim_end_matches('/'))
        };

        let result = self
            .client
            .post(&full_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    let text = response.text().await.unwrap_or_default();
                    return ApiResponse {
                        success: false,
                        content: String::new(),
                        model: model.into(),
                        tokens_used: 0,
                        cost_estimate: 0.0,
                        error: Some(format!(
                            "HTTP {}: {}",
                            status.as_u16(),
                            truncate_str(&text, 200)
                        )),
                    };
                }

                // 逐行解析 SSE 流
                use futures_util::StreamExt;
                let mut stream = response.bytes_stream();
                let mut full_content = String::new();
                let mut buffer = String::new();
                let mut stream_error: Option<String> = None;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].trim().to_string();
                                buffer = buffer[line_end + 1..].to_string();

                                if line.is_empty() || line.starts_with(':') {
                                    continue;
                                }
                                if line == "data: [DONE]" {
                                    break;
                                }
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                                            full_content.push_str(delta);
                                            on_chunk(delta);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            stream_error = Some(format!("流中断: {}", e));
                            tracing::warn!("[API CLIENT] Stream chunk error: {}", e);
                            break;
                        }
                    }
                }

                let tokens = full_content.len() as u32 / 4;
                let cost = estimate_cost_from_model_name(model, tokens, 0);
                self.total_tokens += tokens as u64;
                self.total_cost += cost;

                if let Some(err) = stream_error {
                    ApiResponse {
                        success: false,
                        content: full_content,
                        model: model.into(),
                        tokens_used: tokens,
                        cost_estimate: cost,
                        error: Some(err),
                    }
                } else {
                    ApiResponse {
                        success: true,
                        content: full_content,
                        model: model.into(),
                        tokens_used: tokens,
                        cost_estimate: cost,
                        error: None,
                    }
                }
            }
            Err(e) => ApiResponse {
                success: false,
                content: String::new(),
                model: model.into(),
                tokens_used: 0,
                cost_estimate: 0.0,
                error: Some(format!("Stream request failed: {}", e)),
            },
        }
    }

    /// 带指数退避重试 + 模型降级链的流式调用
    pub async fn chat_stream_with_retry(
        &mut self,
        endpoint: &str,
        api_key: &str,
        model: &str,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        on_chunk: impl FnMut(&str) + Clone,
        retry: &RetryConfig,
    ) -> ApiResponse {
        let mut last_error = String::new();
        let models_to_try: Vec<&str> = std::iter::once(model)
            .chain(retry.fallback_models.iter().map(|s| s.as_str()))
            .collect();

        for &try_model in &models_to_try {
            for attempt in 0..=retry.max_retries {
                if attempt > 0 {
                    let delay = (retry.base_delay_ms * 2u64.pow(attempt - 1)).min(retry.max_delay_ms);
                    tracing::info!("[API] Retry {} for {} after {}ms", attempt, try_model, delay);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }

                let result = self.chat_stream(
                    if try_model == model { endpoint } else { get_fallback_endpoint(try_model, endpoint) },
                    api_key, try_model, messages.clone(), max_tokens, on_chunk.clone(),
                ).await;

                if result.success {
                    tracing::info!("[API] Success with model={} (attempt {})", try_model, attempt + 1);
                    return result;
                }
                last_error = result.error.unwrap_or_else(|| "Unknown error".into());
                if last_error.contains("401") || last_error.contains("403") {
                    break; // Auth errors won't be fixed by retrying
                }
            }
        }

        ApiResponse {
            success: false, content: String::new(), model: model.into(),
            tokens_used: 0, cost_estimate: 0.0,
            error: Some(format!("All retries exhausted. Last error: {}", last_error)),
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> ApiStats {
        ApiStats {
            call_count: self.call_count,
            total_tokens: self.total_tokens,
            total_cost: self.total_cost,
        }
    }
}

/// API 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStats {
    pub call_count: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
}

// ─── 工具函数 ─────────────────────────────────────────────────

/// 获取降级模型的端点 URL
fn get_fallback_endpoint(model: &str, _original: &str) -> &'static str {
    if model.starts_with("deepseek") { "https://api.deepseek.com" }
    else if model.starts_with("kimi") { "https://api.moonshot.cn/v1" }
    else if model.starts_with("glm") { "https://open.bigmodel.cn/api/paas/v4" }
    else if model.starts_with("ollama") { "http://localhost:11434" }
    else { "https://api.deepseek.com" }
}

/// UTF-8-safe string truncation for error messages
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars { return s.to_string(); }
    let preview: String = s.chars().take(max_chars).collect();
    format!("{}...", preview)
}

// ─── 费用估算 ─────────────────────────────────────────────────
// 已统一路由到 agent::billing::estimate_cost_from_model_name
// ChronosBillingEngine 是唯一权威费率来源（官方定价矩阵）
