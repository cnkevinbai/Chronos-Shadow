// Kimi/GLM 专属降本优化引擎
//
// Kimi 特点: 超长上下文(1M)但无缓存, 高定价(¥8/M)
// GLM 特点: 阶梯定价(¥0.5-5/M), 无缓存, 多模型可级联
//
// 降本策略:
//   1. Token预算器: 按模型定价动态分配 max_tokens
//   2. 输出编码优化: 中文输出压缩(中文字符=2-3 token)
//   3. 模型级联: GLM-4.7→5.1→5.2 按复杂度升级
//   4. 本地响应缓存: TTL策略避免重复调用
//   5. 批量合并: 多轮短对话合并为单次请求

use std::collections::HashMap;
use sha2::Digest;

// ─── 模型定价表 (¥/1M tokens) ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price: f64,   // ¥/1M input
    pub output_price: f64,  // ¥/1M output
    pub max_context: usize, // 上下文窗口
}

pub fn get_kimi_glm_pricing(model: &str) -> ModelPricing {
    match model {
        "kimi-k3" => ModelPricing { input_price: 8.0, output_price: 8.0, max_context: 1_000_000 },
        "kimi-k2.7-code" => ModelPricing { input_price: 3.0, output_price: 3.0, max_context: 128_000 },
        "kimi-k2.7-code-highspeed" => ModelPricing { input_price: 1.0, output_price: 1.0, max_context: 128_000 },
        "glm-5.2" => ModelPricing { input_price: 1.0, output_price: 2.0, max_context: 128_000 },
        "glm-5v-turbo" => ModelPricing { input_price: 3.0, output_price: 5.0, max_context: 32_768 },
        "glm-5.1" => ModelPricing { input_price: 0.5, output_price: 2.0, max_context: 128_000 },
        "glm-4.7" => ModelPricing { input_price: 0.5, output_price: 2.0, max_context: 32_768 },
        _ => ModelPricing { input_price: 1.0, output_price: 2.0, max_context: 64_000 },
    }
}

// ─── 1. Token 预算器 ──────────────────────────────────────────────

pub struct TokenBudgeter {
    /// 每会话预算上限 (¥)
    pub session_budget: f64,
    /// 已消费
    pub spent: f64,
    /// 预算告警阈值
    pub warn_threshold: f64,
}

impl TokenBudgeter {
    pub fn new(budget: f64) -> Self {
        Self { session_budget: budget, spent: 0.0, warn_threshold: budget * 0.8 }
    }

    /// 计算此请求的预估费用
    pub fn estimate_cost(
        &self, model: &str, input_tokens: u32, max_output_tokens: u32,
    ) -> f64 {
        let pricing = get_kimi_glm_pricing(model);
        let input_cost = input_tokens as f64 / 1_000_000.0 * pricing.input_price;
        let output_cost = max_output_tokens as f64 / 1_000_000.0 * pricing.output_price;
        input_cost + output_cost
    }

    /// 检查预算是否充足，返回可分配的最大输出 token
    pub fn allocate_output_tokens(
        &self, model: &str, input_tokens: u32,
    ) -> (u32, Option<String>) {
        let pricing = get_kimi_glm_pricing(model);
        let remaining = (self.session_budget - self.spent).max(0.0);
        let est_input_cost = input_tokens as f64 / 1_000_000.0 * pricing.input_price;
        let remaining_for_output = (remaining - est_input_cost).max(0.0);

        let max_output = (remaining_for_output / pricing.output_price * 1_000_000.0) as u32;
        let capped = max_output.min(4096); // 硬上限 4096

        let warning = if self.spent > self.warn_threshold {
            Some(format!("⚠️ 预算已用 {:.0}%, 剩余 ¥{:.2}",
                (self.spent / self.session_budget * 100.0), remaining))
        } else {
            None
        };

        (capped, warning)
    }

    /// 记录实际消费
    pub fn record_spend(&mut self, model: &str, input_tokens: u32, output_tokens: u32) {
        let pricing = get_kimi_glm_pricing(model);
        let cost = input_tokens as f64 / 1_000_000.0 * pricing.input_price
            + output_tokens as f64 / 1_000_000.0 * pricing.output_price;
        self.spent += cost;
    }
}

// ─── 2. 响应压缩器 ────────────────────────────────────────────────

pub struct ResponseCompressor;

impl ResponseCompressor {
    /// 压缩 LLM 输出: 去除冗余、合并重复、优化中文编码
    pub fn compress(content: &str, model: &str) -> String {
        let mut result = content.to_string();

        // 1. 去除连续空行 (>2个换行)
        let re = regex::Regex::new(r"\n{3,}").unwrap();
        result = re.replace_all(&result, "\n\n").to_string();

        // 2. 去除代码块中的空行 (保留首尾)
        let code_re = regex::Regex::new(r"```(\w*)\n([\s\S]*?)```").unwrap();
        result = code_re.replace_all(&result, |caps: &regex::Captures| {
            let lang = &caps[1];
            let code = caps[2].trim();
            format!("```{}\n{}\n```", lang, code)
        }).to_string();

        // 3. 合并连续重复行
        let lines: Vec<&str> = result.lines().collect();
        let mut deduped = Vec::new();
        for (i, &line) in lines.iter().enumerate() {
            if i > 0 && line == lines[i-1] && !line.trim().is_empty() {
                continue; // 跳过连续重复行
            }
            deduped.push(line);
        }
        result = deduped.join("\n");

        // 4. Kimi 特化: 中文标点压缩 (全角→半角不影响阅读)
        if model.starts_with("kimi") {
            result = result.replace("，", ",").replace("。", ".")
                .replace("：", ":").replace("；", ";");
        }

        let saved = content.len().saturating_sub(result.len());
        if saved > 50 {
            tracing::info!("[Compressor] Saved {} chars ({:.0}%) for {}", saved,
                saved as f64 / content.len() as f64 * 100.0, model);
        }
        result
    }

    /// 估算压缩率
    pub fn estimate_compression(content: &str) -> f64 {
        let compressed = Self::compress(content, "kimi-k3");
        if content.is_empty() { return 0.0; }
        1.0 - compressed.len() as f64 / content.len() as f64
    }
}

// ─── 3. GLM 模型级联路由器 ─────────────────────────────────────────

pub struct GlmCascadeRouter;

impl GlmCascadeRouter {
    /// 按任务复杂度选择 GLM 模型 (便宜优先)
    /// 简单: glm-4.7 (¥0.5/M) → 中等: glm-5.1 (¥0.5/M) → 复杂: glm-5.2 (¥1/M)
    pub fn route_by_complexity(complexity: f64, needs_vision: bool) -> &'static str {
        if needs_vision {
            return "glm-5v-turbo"; // 唯一视觉模型
        }
        if complexity > 0.7 { "glm-5.2" }
        else if complexity > 0.4 { "glm-5.1" }
        else { "glm-4.7" }
    }

    /// Kimi 模型选路: 超长文档→K3, 代码→K2.7, 简单→高速版
    pub fn route_kimi(task_type: &str, estimated_length: usize) -> &'static str {
        if estimated_length > 100_000 { "kimi-k3" }
        else if task_type.contains("code") || task_type.contains("代码") { "kimi-k2.7-code" }
        else { "kimi-k2.7-code-highspeed" }
    }

    /// 获取级联链 (从便宜到贵)
    pub fn glm_cascade_chain() -> Vec<&'static str> {
        vec!["glm-4.7", "glm-5.1", "glm-5.2"]
    }

    pub fn kimi_cascade_chain() -> Vec<&'static str> {
        vec!["kimi-k2.7-code-highspeed", "kimi-k2.7-code", "kimi-k3"]
    }
}

// ─── 4. 本地响应缓存 (Kimi/GLM 无云端缓存, 本地补偿) ──────────────

pub struct LocalResponseCache {
    /// 请求哈希 → (响应, 过期时间戳)
    cache: HashMap<String, (String, u64)>,
    /// 缓存 TTL (秒)
    ttl_secs: u64,
    /// 最大条目数
    max_entries: usize,
    hits: u64,
    misses: u64,
}

impl LocalResponseCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            ttl_secs: 300, // 5分钟
            max_entries: 200,
            hits: 0,
            misses: 0,
        }
    }

    /// 生成请求哈希 (取前512字符)
    pub fn request_hash(messages: &[super::api_client::ChatMessage]) -> String {
        let mut hasher = sha2::Sha256::new();
        for msg in messages.iter().take(5) {
            hasher.update(msg.role.as_bytes());
            hasher.update(msg.content.as_bytes());
        }
        format!("{:x}", hasher.finalize())[..32].to_string()
    }

    /// 查询缓存
    pub fn get(&mut self, hash: &str) -> Option<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        if let Some((resp, expires)) = self.cache.get(hash) {
            if now < *expires {
                self.hits += 1;
                return Some(resp.clone());
            }
        }
        self.misses += 1;
        None
    }

    /// 写入缓存
    pub fn set(&mut self, hash: &str, response: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        self.cache.insert(hash.into(), (response, now + self.ttl_secs));
        // LRU 淘汰
        while self.cache.len() > self.max_entries {
            let oldest = self.cache.iter()
                .min_by_key(|(_, (_, exp))| *exp)
                .map(|(k, _)| k.clone());
            if let Some(k) = oldest { self.cache.remove(&k); }
        }
    }

    /// 缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 * 100.0 }
    }

    /// 节省估算 (按平均 ¥1/M 计算)
    pub fn estimated_savings(&self) -> f64 {
        let avg_tokens_per_hit = 500.0;
        self.hits as f64 * avg_tokens_per_hit / 1_000_000.0 * 1.0
    }
}

// ─── 5. 批量合并器 ──────────────────────────────────────────────────

pub struct BatchMerger;

impl BatchMerger {
    /// 多轮短对话合并为单次请求 (节省 API 调用开销)
    /// 适用场景: 连续快速提问 (间隔 <30s)
    pub fn should_merge(
        messages: &[super::api_client::ChatMessage],
        time_since_last: std::time::Duration,
    ) -> bool {
        let user_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role == "user")
            .collect();
        user_msgs.len() >= 2
            && time_since_last.as_secs() < 30
            && messages.iter().map(|m| m.content.len()).sum::<usize>() < 8000
    }

    /// 合并连续用户消息
    pub fn merge_consecutive_users(
        messages: &[super::api_client::ChatMessage],
    ) -> Vec<super::api_client::ChatMessage> {
        let mut merged = Vec::new();
        let mut pending_user = String::new();

        for msg in messages {
            if msg.role == "user" {
                if pending_user.is_empty() {
                    pending_user = msg.content.clone();
                } else {
                    pending_user.push_str("\n---\n");
                    pending_user.push_str(&msg.content);
                }
            } else {
                if !pending_user.is_empty() {
                    merged.push(super::api_client::ChatMessage {
                        role: "user".into(), content: std::mem::take(&mut pending_user),
                    });
                }
                merged.push(msg.clone());
            }
        }
        if !pending_user.is_empty() {
            merged.push(super::api_client::ChatMessage {
                role: "user".into(), content: pending_user,
            });
        }
        merged
    }
}

// ─── 综合降本报告 ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct KimiGlmSavingsReport {
    pub model: String,
    pub budget_saved: f64,
    pub compression_saved_tokens: u64,
    pub cache_saved_calls: u64,
    pub cascade_downgrades: u64,
    pub total_estimated_savings: f64,
}

impl KimiGlmSavingsReport {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.into(),
            budget_saved: 0.0,
            compression_saved_tokens: 0,
            cache_saved_calls: 0,
            cascade_downgrades: 0,
            total_estimated_savings: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_allocation() {
        let budgeter = TokenBudgeter::new(1.0); // ¥1 budget
        let (tokens, _) = budgeter.allocate_output_tokens("kimi-k3", 1000);
        // 1000 input tokens × ¥8/M = ¥0.008 → 剩余 ¥0.992
        // ¥0.992 / ¥8/M = 124000 output tokens → capped at 4096
        assert_eq!(tokens, 4096);
    }

    #[test]
    fn test_glm_cascade() {
        assert_eq!(GlmCascadeRouter::route_by_complexity(0.3, false), "glm-4.7");
        assert_eq!(GlmCascadeRouter::route_by_complexity(0.5, false), "glm-5.1");
        assert_eq!(GlmCascadeRouter::route_by_complexity(0.8, false), "glm-5.2");
        assert_eq!(GlmCascadeRouter::route_by_complexity(0.3, true), "glm-5v-turbo");
    }

    #[test]
    fn test_compression() {
        let input = "Hello\n\n\n\nWorld\n\n\n\nHello\n\n\n\nWorld";
        let compressed = ResponseCompressor::compress(input, "kimi-k3");
        assert!(compressed.len() < input.len());
    }

    #[test]
    fn test_merge_users() {
        let msgs = vec![
            crate::agent::api_client::ChatMessage { role: "user".into(), content: "Q1".into() },
            crate::agent::api_client::ChatMessage { role: "user".into(), content: "Q2".into() },
            crate::agent::api_client::ChatMessage { role: "assistant".into(), content: "A".into() },
        ];
        let merged = BatchMerger::merge_consecutive_users(&msgs);
        assert_eq!(merged.len(), 2); // Q1+Q2 merged → 1 user + 1 assistant
        assert!(merged[0].content.contains("---"));
    }
}
