// 云端大模型 API 客户端
//
// 负责通过 HTTP 调用 DeepSeek / Kimi / GLM / OpenAI 等云端大模型 API
// 支持：流式响应、超时控制、指数退避重试、模型降级链、上下文窗口感知

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use crate::agent::billing::estimate_cost_from_model_name;
use crate::agent::reasoning_depth::ReasoningDepth;

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
    /// 推理深度（控制 max_tokens + temperature）
    pub reasoning_depth: ReasoningDepth,
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
            reasoning_depth: ReasoningDepth::Medium,
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
            "max_tokens": max_tokens.unwrap_or_else(|| self.reasoning_depth.max_tokens()),
            "temperature": self.reasoning_depth.temperature(),
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
            "max_tokens": max_tokens.unwrap_or_else(|| self.reasoning_depth.max_tokens()),
            "temperature": self.reasoning_depth.temperature(),
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

// ─── Tauri Commands ──────────────────────────────────────────────

static LAST_API_CALL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static CANCEL_STREAM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 估算任务复杂度 (基于消息长度和关键词)
fn estimate_task_complexity(messages: &[ChatMessage]) -> f64 {
    let total_len: usize = messages.iter().map(|m| m.content.len()).sum();
    let has_architecture = messages.iter().any(|m| {
        let c = &m.content;
        c.contains("架构") || c.contains("设计") || c.contains("重构") || c.contains("architecture")
    });
    let base = (total_len as f64 / 5000.0).min(0.6);
    if has_architecture { (base + 0.3).min(1.0) } else { base }
}

/// 将 API 调用中的模型字符串映射到 ModelModel 枚举
fn parse_model_to_enum(model: &str) -> crate::agent::router::ModelModel {
    crate::agent::billing::parse_model_string(model)
}

/// Estimate prompt/completion split from total tokens and response content
fn split_tokens(total: u32, content: &str) -> (u32, u32) {
    let completion_est = (content.len() as f64 / 4.0).ceil() as u32;
    let completion = completion_est.min(total);
    let prompt = total.saturating_sub(completion);
    (prompt, completion)
}

/// 生成个性化系统提示词前缀（用户名字/昵称/人格/技能/工作模式）
fn personalization_prompt(profile: &crate::agent::user_profile::UserProfile) -> String {
    let tone = match profile.personality.as_str() {
        "professional" => "professional and concise",
        "playful" => "playful and energetic",
        _ => "warm and friendly",
    };
    format!(
        "## Your user\n- Name: {}\n- Nickname: {}\n- Tone: {}\n- Skill level: {}/100\n- Work mode: {}\n\nAddress the user warmly by their nickname when appropriate.",
        profile.display_name, profile.nickname, tone, profile.skill_level, profile.work_mode
    )
}

#[tauri::command]
pub async fn chat_api(
    state: tauri::State<'_, crate::state::AppState>,
    endpoint: String,
    api_key: String,
    model: String,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
) -> Result<ApiResponse, String> {
    let mut msgs: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
            role: m["role"].as_str().unwrap_or("user").into(),
            content: m["content"].as_str().unwrap_or("").into(),
        })
        .collect();

    // 注入系统指令：告知 LLM 可用的操作格式
    let action_instructions = r#"You are Chronos-Shadow, an AI coding assistant with file system access.
When you need to CREATE or MODIFY files, output a JSON action block:
{"action":"file_edit","params":{"path":"relative/path.ext","content":"file content here"}}

When you need to READ a file:
{"action":"file_read","params":{"path":"relative/path.ext"}}

When you need to SEARCH the web:
{"action":"web_search","params":{"query":"search terms"}}

When you need to FETCH a URL:
{"action":"web_fetch","params":{"url":"https://..."}}

When the user asks you to CREATE a PPT/PowerPoint presentation, output:
{"action":"pptx_generate","params":{"title":"Title","slides":[{"slide_type":"TitleSlide","title":"Title","subtitle":"Subtitle"},{"slide_type":"Content","title":"Slide","body":"Content","bullets":["Point1"]},{"slide_type":"ThankYou","title":"Thank You"}]}}
Templates: Corporate,TechMinimal,Creative,Academic,MinimalWhite,DarkMode

IMPORTANT: Put the JSON on its own line. Write REAL code, not placeholders. Create complete, working files."#;

    // 注入个性化画像前缀
    let full_instructions = {
        let profile = state.user_profile.lock().unwrap();
        format!("{}\n\n{}", personalization_prompt(&profile), action_instructions)
    };

    // 检查是否已有系统消息，有则追加指令
    if msgs.first().map(|m| m.role.as_str()) == Some("system") {
        msgs[0].content = format!("{}\n\n{}", msgs[0].content, full_instructions);
    } else {
        msgs.insert(0, ChatMessage { role: "system".into(), content: full_instructions });
    }

    // Rate limit: minimum 1.5s between calls
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let last = LAST_API_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if now - last < 1500 && last > 0 {
        return Err(format!(
            "[速率限制] 请等待 {}ms 后再发送。防止误触导致资费浪费。",
            1500 - (now - last)
        ));
    }
    LAST_API_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    // 熔断器检查
    if !state.api_circuit_breaker.allow() {
        return Err("[CIRCUIT BREAKER] API 熔断器已激活，暂时拒绝请求，请稍后重试".into());
    }

    // 原子化预占: 防止并发调用超额 (TOCTOU 修复)
    let estimated_cost = 0.05;
    if !state.billing_engine.try_reserve(estimated_cost) {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，API 调用已被阻断。",
            budget, cap
        ));
    }

    // Resolve API key from vault if frontend sent empty (key now stored server-side)
    let resolved_key = if api_key.is_empty() { crate::agent::key_vault::resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        state.billing_engine.settle(estimated_cost, 0.0); // 释放预留
        return Err("[VAULT EMPTY] API Key 未找到。".into());
    }
    let mut client = state.api_client.lock().await;
    let response = client.chat(&endpoint, &resolved_key, &model, msgs, max_tokens).await;

    // 结算实际费用
    let actual_cost = if response.success {
        let model_enum = parse_model_to_enum(&model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        let cost = state.billing_engine.estimate_cost(&model_enum, prompt, completion);
        state.billing_engine.record(&model_enum, prompt, completion, None);
        cost
    } else { 0.0 };
    state.billing_engine.settle(estimated_cost, actual_cost);

    // ── 进化总线 + 数据飞轮 定期同步 ──
    {
        let should_evolve = state.evolution_bus.lock().unwrap().should_evolve();
        if should_evolve {
            let mut wi = state.web_intelligence.lock().await;
            let mut evo = state.evolution_bus.lock().unwrap();
            let mut fw = state.flywheel.lock().unwrap();

            // 1. 从 WebIntelligence 采集实时指标
            let stats = wi.get_stats();
            fw.collect_from_web_intel(
                stats.total_searches, stats.total_fetches, stats.bytes_downloaded,
                stats.unified_cache_hits, stats.unified_cache_misses,
            );
            fw.collect_from_distillation(
                stats.total_distilled, stats.total_bytes_saved,
                stats.avg_compression_ratio,
                wi.distillation.avg_quality(),
            );

            // 2. 同步到进化总线
            wi.sync_to_evolution_bus(&mut evo);
            drop(wi);

            // 3. 使用飞轮实时指标替代硬编码评估值
            let distill_q = fw.metrics.get("distill_quality").map(|m| m.value / 100.0).unwrap_or(0.85);
            let cache_q = fw.metrics.get("cache_hit_rate").map(|m| m.value / 100.0).unwrap_or(0.78);
            let cache_stability = fw.metrics.get("cache_api_saved").map(|m| (m.value / 100.0).min(1.0)).unwrap_or(0.85);

            evo.assess_advancement(&[
                (crate::agent::evolution_bus::EngineId::Distillation, distill_q, 0.92),
                (crate::agent::evolution_bus::EngineId::CacheEngine, cache_q, cache_stability),
                (crate::agent::evolution_bus::EngineId::Scheduling, 0.82, 0.88),
                (crate::agent::evolution_bus::EngineId::HallucinationGuard, 0.80, 0.85),
                (crate::agent::evolution_bus::EngineId::AgentQuality, 0.83, 0.90),
                (crate::agent::evolution_bus::EngineId::Collaboration, 0.80, 0.87),
                (crate::agent::evolution_bus::EngineId::TaskIntelligence, 0.78, 0.85),
                (crate::agent::evolution_bus::EngineId::PredictiveAnalytics, 0.80, 0.86),
                (crate::agent::evolution_bus::EngineId::LocalAnalytics, 0.82, 0.90),
            ]);

            // 4. 飞轮旋转 — 累积收益并创建快照
            fw.spin();
            drop(evo);
            drop(fw);
        }
    }

    Ok(response)
}

#[tauri::command]
pub fn get_cache_hit_stats(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    let ctx = state.context_cache.lock().unwrap();
    let models = ["deepseek-v4-pro", "deepseek-v4-flash", "kimi-k3", "kimi-k2.7-code",
        "glm-5.2", "glm-5.1", "glm-4.7"];
    let entries: Vec<serde_json::Value> = models.iter().map(|&m| {
        let stats = ctx.get_stats(m);
        serde_json::json!({
            "model": m,
            "total_requests": stats.total_requests,
            "cache_hits": stats.cache_hits,
            "cached_tokens": stats.cached_tokens,
            "cost_saved": format!("{:.4}", stats.cost_saved),
            "hit_rate": format!("{:.1}", stats.hit_rate),
        })
    }).collect();
    let (total_tokens, total_cost) = ctx.total_savings();
    serde_json::json!({
        "models": entries,
        "total_cached_tokens": total_tokens,
        "total_cost_saved": format!("{:.4}", total_cost),
    })
}

#[tauri::command]
pub fn check_dev_environment() -> crate::agent::env_checker::EnvReport {
    crate::agent::env_checker::check_environment()
}

#[tauri::command]
pub fn auto_install_deps() -> Vec<String> {
    let report = crate::agent::env_checker::check_environment();
    crate::agent::env_checker::auto_install_missing(&report)
}

#[tauri::command]
pub fn cancel_chat_stream() {
    CANCEL_STREAM.store(true, std::sync::atomic::Ordering::Relaxed);
    tracing::info!("[STREAM] User cancelled active stream");
}

#[tauri::command]
pub async fn chat_api_stream(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
    endpoint: String,
    api_key: String,
    model: String,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
) -> Result<ApiResponse, String> {
    let mut msgs: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
            role: m["role"].as_str().unwrap_or("user").into(),
            content: m["content"].as_str().unwrap_or("").into(),
        })
        .collect();

    // 注入系统指令 — 全能力清单
    let action_instructions = r#"You are Chronos-Shadow. You can CREATE files, GENERATE PPTs, CHECK & INSTALL tools, SEARCH the web.

## Available Actions (output as JSON on its own line)

### File Operations
{"action":"file_edit","params":{"path":"file.ext","content":"..."}}
{"action":"file_read","params":{"path":"file.ext"}}

### Web Operations
{"action":"web_search","params":{"query":"search terms"}}
{"action":"web_fetch","params":{"url":"https://..."}}

### PPT Generation (auto-installs python-pptx if needed)
{"action":"pptx_generate","params":{"title":"Title","template":"Corporate","slides":[
  {"slide_type":"TitleSlide","title":"T","subtitle":"S"},
  {"slide_type":"Content","title":"T","body":"B","bullets":["p1","p2"]},
  {"slide_type":"ThankYou","title":"Thanks"}
]}}
Templates: Corporate,TechMinimal,Creative,Academic,MinimalWhite,DarkMode
SlideTypes: TitleSlide,SectionHeader,Content,TwoColumn,QuoteSlide,TableSlide,ChartSlide,ThankYou

### Environment Check & Auto-Install
{"action":"check_environment"}  → returns installed/missing tools
{"action":"auto_install_deps"}  → installs python-pptx etc.

### Code blocks (```language ... ```) are auto-saved as files.

IMPORTANT: JSON on its own line. Complete code, real content, no placeholders."#;

    // 注入个性化画像前缀
    let full_instructions = {
        let profile = state.user_profile.lock().unwrap();
        format!("{}\n\n{}", personalization_prompt(&profile), action_instructions)
    };

    if msgs.first().map(|m| m.role.as_str()) == Some("system") {
        msgs[0].content = format!("{}\n\n{}", msgs[0].content, full_instructions);
    } else {
        msgs.insert(0, ChatMessage { role: "system".into(), content: full_instructions });
    }

    // Rate limit — same 1.5s gate as chat_api
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let last = LAST_API_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if last > 0 {
        let elapsed_ms = (now - last) as u64;
        if elapsed_ms < 1500 {
            return Err(format!(
                "[RATE LIMIT] 请求过于频繁，请等待 {}ms",
                1500 - elapsed_ms
            ));
        }
    }
    LAST_API_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    // Reset cancel flag at start of new stream
    CANCEL_STREAM.store(false, std::sync::atomic::Ordering::Relaxed);

    // 原子化预占: 防止并发调用超额
    let estimated_cost = 0.05;
    if !state.billing_engine.try_reserve(estimated_cost) {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，流式调用已被阻断。",
            budget, cap
        ));
    }

    // 上下文窗口智能截断 (保留 system + 最近消息，不超窗口 80%)
    msgs = trim_context(msgs, &model, 2048);

    // Resolve API key from vault
    let resolved_key = if api_key.is_empty() { crate::agent::key_vault::resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        state.billing_engine.settle(estimated_cost, 0.0);
        return Err("[VAULT EMPTY] API Key 未找到。".into());
    }
    // 🔬 上下文缓存检测: DeepSeek 一折缓存前缀标记
    let session_id = format!("stream-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let (cached_tokens, _cache_mask) = {
        let mut ctx_cache = state.context_cache.lock().unwrap();
        ctx_cache.detect_cacheable_prefix(&session_id, &msgs, &model)
    };

    // 🔬 前缀稳定化: DeepSeek 缓存最优化消息顺序
    if model.starts_with("deepseek") {
        let ctx_cache = state.context_cache.lock().unwrap();
        msgs = ctx_cache.stabilize_prefix(&msgs);
    }

    // 🔬 提示词规范化: 剥离时间戳/UUID, 提升哈希命中率
    msgs = crate::agent::context_cache::ContextCacheEngine::canonicalize_messages(&msgs);

    // 🔬 Kimi/GLM 智能截断 (无原生缓存, 激进裁剪降本)
    if model.starts_with("kimi") || model.starts_with("glm") {
        let ctx_cache = state.context_cache.lock().unwrap();
        msgs = ctx_cache.optimize_kimi_context(&msgs);
        // 批量合并: 连续短提问合并为单次请求
        msgs = crate::agent::kimi_glm_optimizer::BatchMerger::merge_consecutive_users(&msgs);
    }

    // 🔬 模型级联: GLM 按任务复杂度自动选型 (降低成本)
    let effective_model = if model.starts_with("glm") && !model.contains("5v") {
        let complexity = estimate_task_complexity(&msgs);
        let cascade = crate::agent::kimi_glm_optimizer::GlmCascadeRouter::route_by_complexity(complexity, false);
        if cascade != model {
            tracing::info!("[GLM Cascade] {} → {} (complexity={:.2})", model, cascade, complexity);
        }
        cascade.to_string()
    } else {
        model.clone()
    };

    let mut client = state.api_client.lock().await;
    let app_handle2 = app_handle.clone();
    let retry_cfg = RetryConfig::default();
    let response = client
        .chat_stream_with_retry(&endpoint, &resolved_key, &effective_model, msgs.clone(), max_tokens,
            move |chunk| {
                if CANCEL_STREAM.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let _ = app_handle2.emit("chat-stream-chunk", chunk);
            },
            &retry_cfg,
        ).await;

    // 结算实际费用 (含缓存折扣)
    let actual_cost = if response.success {
        let model_enum = parse_model_to_enum(&effective_model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        let mut cost = state.billing_engine.estimate_cost(&model_enum, prompt, completion);
        if cached_tokens > 0 && effective_model.starts_with("deepseek") {
            let discount = cached_tokens as f64 / 1_000_000.0 * 0.9;
            cost = (cost - discount).max(0.0);
        }
        state.billing_engine.record(&model_enum, prompt, completion, None);
        cost
    } else { 0.0 };
    state.billing_engine.settle(estimated_cost, actual_cost);

    // ── 进化总线 + 数据飞轮 (stream) ──
    {
        let should_evolve = state.evolution_bus.lock().unwrap().should_evolve();
        if should_evolve {
            let mut wi = state.web_intelligence.lock().await;
            let mut evo = state.evolution_bus.lock().unwrap();
            let mut fw = state.flywheel.lock().unwrap();

            let stats = wi.get_stats();
            fw.collect_from_web_intel(
                stats.total_searches, stats.total_fetches, stats.bytes_downloaded,
                stats.unified_cache_hits, stats.unified_cache_misses,
            );
            fw.collect_from_distillation(
                stats.total_distilled, stats.total_bytes_saved,
                stats.avg_compression_ratio, wi.distillation.avg_quality(),
            );

            wi.sync_to_evolution_bus(&mut evo);
            drop(wi);

            let distill_q = fw.metrics.get("distill_quality").map(|m| m.value / 100.0).unwrap_or(0.85);
            let cache_q = fw.metrics.get("cache_hit_rate").map(|m| m.value / 100.0).unwrap_or(0.78);
            let cache_stability = fw.metrics.get("cache_api_saved").map(|m| (m.value / 100.0).min(1.0)).unwrap_or(0.85);

            evo.assess_advancement(&[
                (crate::agent::evolution_bus::EngineId::Distillation, distill_q, 0.92),
                (crate::agent::evolution_bus::EngineId::CacheEngine, cache_q, cache_stability),
                (crate::agent::evolution_bus::EngineId::Scheduling, 0.82, 0.88),
                (crate::agent::evolution_bus::EngineId::HallucinationGuard, 0.80, 0.85),
                (crate::agent::evolution_bus::EngineId::AgentQuality, 0.83, 0.90),
                (crate::agent::evolution_bus::EngineId::Collaboration, 0.80, 0.87),
                (crate::agent::evolution_bus::EngineId::TaskIntelligence, 0.78, 0.85),
                (crate::agent::evolution_bus::EngineId::PredictiveAnalytics, 0.80, 0.86),
                (crate::agent::evolution_bus::EngineId::LocalAnalytics, 0.82, 0.90),
            ]);

            fw.spin();
            drop(evo);
            drop(fw);
        }
    }

    Ok(response)
}

// ─── 费用估算 ─────────────────────────────────────────────────
// 已统一路由到 agent::billing::estimate_cost_from_model_name
// ChronosBillingEngine 是唯一权威费率来源（官方定价矩阵）
