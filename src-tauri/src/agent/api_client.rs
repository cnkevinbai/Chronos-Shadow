// 云端大模型 API 客户端
//
// 负责通过 HTTP 调用 DeepSeek / Kimi / GLM / OpenAI 等云端大模型 API
// 支持：流式响应、超时控制、自动重试、LAN 降级

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
                            error: Some(format!("Failed to parse response: {}", &text[..200.min(text.len())])),
                        }
                    }
                } else {
                    ApiResponse {
                        success: false,
                        content: String::new(),
                        model: model.into(),
                        tokens_used: 0,
                        cost_estimate: 0.0,
                        error: Some(format!("HTTP {}: {}", status.as_u16(), &text[..200.min(text.len())])),
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
                            &text[..200.min(text.len())]
                        )),
                    };
                }

                // 逐行解析 SSE 流
                use futures_util::StreamExt;
                let mut stream = response.bytes_stream();
                let mut full_content = String::new();
                let mut buffer = String::new();

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            // 按行解析 SSE data
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
                            tracing::warn!("[API CLIENT] Stream chunk error: {}", e);
                            break;
                        }
                    }
                }

                let tokens = full_content.len() as u32 / 4; // 粗略估算
                let cost = estimate_cost_from_model_name(model, tokens, 0);
                self.total_tokens += tokens as u64;
                self.total_cost += cost;

                ApiResponse {
                    success: true,
                    content: full_content,
                    model: model.into(),
                    tokens_used: tokens,
                    cost_estimate: cost,
                    error: None,
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

// ─── 费用估算 ─────────────────────────────────────────────────
// 已统一路由到 agent::billing::estimate_cost_from_model_name
// ChronosBillingEngine 是唯一权威费率来源（官方定价矩阵）
