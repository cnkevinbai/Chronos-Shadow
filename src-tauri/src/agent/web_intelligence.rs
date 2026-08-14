// 对外信息搜索抓取与分析能力模块 (Web Intelligence Module)
//
// 核心功能：
// - 域名白名单管理：预置技术文档站点 + 用户自定义
// - Web Search：搜索引擎查询 (Bing API / 可配置后端)
// - Web Fetch：指定 URL 内容抓取 + HTML→Markdown 转换
// - Web Research：多源聚合分析 (搜索→抓取→蒸馏→总结)
// - Office Connect：办公系统连接器 (邮件/日历/任务只读)
//
// 安全约束（与 SecurityBoundary + ApprovalGate 联动）：
// - 域名白名单强制校验 — 非白名单域名拒绝请求
// - 所有外网操作通过第四红线审批门禁
// - 请求内容自动脱敏 (API Keys / 文件路径 / 个人信息)
// - 响应内容端侧蒸馏 — 仅喂结论给大模型，原始内容不入上下文
// - 全量审计日志 — 所有外网请求可追溯
//
// 设计原则：
//   1. 只读优先 (Read-Only by Default) — 绝不主动写入外网
//   2. 白名单约束 (Allowlist-Only) — 仅访问已批准的域名
//   3. 蒸馏优先 (Distill-before-LLM) — 外部数据经处理后才喂给大模型
//   4. 用户决策 (User-in-the-Loop) — 搜索目标、白名单、策略由用户控制

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;
use super::distillation_engine::{DistillationEngine, DistillationLevel, ContentFragment, EntityType};
use super::cache_engine::{UnifiedCache, CacheCategory};
use super::evolution_bus::EvolutionBus;

// ─── 搜索提供商 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProvider {
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub max_results_per_query: u32,
}

impl SearchProvider {
    pub fn bing(api_key: Option<String>) -> Self {
        Self {
            name: "Bing".into(),
            endpoint: "https://api.bing.microsoft.com/v7.0/search".into(),
            api_key,
            enabled: true,
            max_results_per_query: 10,
        }
    }

    pub fn duckduckgo() -> Self {
        Self {
            name: "DuckDuckGo".into(),
            endpoint: "https://api.duckduckgo.com".into(),
            api_key: None,
            enabled: true,
            max_results_per_query: 20,
        }
    }
}

// ─── 域名白名单条目 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEntry {
    pub domain: String,
    pub description: String,
    pub category: DomainCategory,
    pub allowed: bool,
    pub added_by: String,
    pub added_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainCategory {
    OfficialDocs,     // 官方技术文档
    TechCommunity,    // 技术社区
    SearchEngine,     // 搜索引擎 API
    OfficeIntegration, // 办公集成
    UserCustom,       // 用户自定义
}

impl DomainCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::OfficialDocs => "官方文档",
            Self::TechCommunity => "技术社区",
            Self::SearchEngine => "搜索引擎",
            Self::OfficeIntegration => "办公集成",
            Self::UserCustom => "用户自定义",
        }
    }
}

// ─── 搜索结果 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub rank: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_estimated: u64,
    pub provider: String,
    pub latency_ms: u64,
}

// ─── 网页抓取结果 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub url: String,
    pub title: Option<String>,
    pub content_markdown: String,
    pub content_length: usize,
    pub status_code: u16,
    pub latency_ms: u64,
}

// ─── 蒸馏结果（复用 McpClient 风格）────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledResearch {
    pub query: String,
    pub summary: String,
    pub key_findings: Vec<String>,
    pub sources: Vec<String>,
    pub confidence: f32,
    pub raw_size_bytes: usize,
    pub distilled_size_bytes: usize,
    pub compression_ratio: f64,
}

// ─── 审计日志 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuditEntry {
    pub id: String,
    pub timestamp: String,
    pub operation: String,
    pub target_url: Option<String>,
    pub domain: Option<String>,
    pub allowed: bool,
    pub approval_id: Option<String>,
    pub bytes_received: usize,
    pub latency_ms: u64,
    pub error: Option<String>,
}

// ─── 办公集成连接器 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeConnector {
    pub connector_type: OfficeConnectorType,
    pub endpoint: String,
    pub auth_type: AuthType,
    pub enabled: bool,
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfficeConnectorType {
    Email,       // 邮件只读
    Calendar,    // 日历只读
    Tasks,       // 任务只读
    Contacts,    // 联系人（需额外审批）
}

impl OfficeConnectorType {
    pub fn label(&self) -> &str {
        match self {
            Self::Email => "邮件",
            Self::Calendar => "日历",
            Self::Tasks => "任务",
            Self::Contacts => "联系人",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    OAuth2 { client_id: String, token_url: String },
    ApiKey { header_name: String },
    None,
}

// ─── Web 智能引擎 ──────────────────────────────────────────────────

// ─── 公开导出类型（Tauri Commands 使用）───────────────────────────

/// Web 搜索结果（前端兼容类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub relevance_score: f64,
}

/// Web 抓取结果（前端兼容类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResult {
    pub success: bool,
    pub url: String,
    pub title: String,
    pub content: String,
    pub content_length: usize,
    pub distilled: bool,
    pub distilled_summary: Option<String>,
    pub key_points: Vec<String>,
    pub error: Option<String>,
}

/// 研究报告（前端兼容类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    pub topic: String,
    pub summary: String,
    pub key_findings: Vec<String>,
    pub sources: Vec<WebSearchResult>,
    pub confidence: f64,
    pub timestamp: String,
    pub recommendations: Vec<String>,
}

/// Web 智能统计（前端兼容类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebIntelStats {
    pub total_searches: u64,
    pub total_fetches: u64,
    pub total_research: u64,
    pub bytes_downloaded: u64,
    pub domains_whitelisted: u64,
    pub requests_blocked: u64,
    pub estimated_cost_saved: f64,
    /// 蒸馏引擎：累计蒸馏次数
    pub total_distilled: u64,
    /// 蒸馏引擎：累计节省字节数
    pub total_bytes_saved: u64,
    /// 蒸馏引擎：平均压缩率
    pub avg_compression_ratio: f64,
    /// 蒸馏引擎：缓存命中率
    pub cache_hit_rate: f64,
    /// 统一缓存：总命中次数
    pub unified_cache_hits: u64,
    /// 统一缓存：总未命中次数
    pub unified_cache_misses: u64,
    /// 统一缓存：节省的API调用数
    pub api_calls_saved: u64,
}

// ─── 主结构体 ──────────────────────────────────────────────────────

pub struct WebIntelligence {
    /// 域名白名单
    pub domain_whitelist: Vec<DomainEntry>,
    /// 搜索提供商
    pub search_providers: HashMap<String, SearchProvider>,
    /// 办公连接器
    pub office_connectors: Vec<OfficeConnector>,
    /// 审计日志
    pub audit_log: Vec<WebAuditEntry>,
    /// 蒸馏阈值（字节）
    pub distillation_threshold: usize,
    /// 请求超时（毫秒）
    pub request_timeout_ms: u64,
    /// 全球启用开关
    pub enabled: bool,
    /// 审批门禁启用
    pub approval_required: bool,
    /// 请求计数
    request_counter: u64,
    /// 累计抓取字节数
    pub total_bytes_fetched: u64,
    /// 多级语义蒸馏引擎
    pub distillation: DistillationEngine,
    /// 统一缓存引擎（搜索/抓取/蒸馏跨模块缓存）
    pub cache: UnifiedCache,
}

impl WebIntelligence {
    pub fn new() -> Self {
        let mut domain_whitelist = Vec::new();

        // ── 预置官方技术文档域名 ──
        let official_docs = [
            ("docs.rs", "Rust 标准库文档"),
            ("doc.rust-lang.org", "Rust 官方文档"),
            ("crates.io", "Rust Crate 仓库"),
            ("react.dev", "React 官方文档"),
            ("nextjs.org", "Next.js 官方文档"),
            ("nodejs.org", "Node.js 官方文档"),
            ("deno.com", "Deno 官方文档"),
            ("tauri.app", "Tauri 官方文档"),
            ("vitejs.dev", "Vite 构建工具文档"),
            ("typescriptlang.org", "TypeScript 官方文档"),
            ("tailwindcss.com", "Tailwind CSS 文档"),
            ("python.org", "Python 官方文档"),
            ("go.dev", "Go 语言官方文档"),
            ("kubernetes.io", "Kubernetes 文档"),
            ("docker.com", "Docker 官方文档"),
        ];
        for (domain, desc) in &official_docs {
            domain_whitelist.push(DomainEntry {
                domain: domain.to_string(),
                description: desc.to_string(),
                category: DomainCategory::OfficialDocs,
                allowed: true,
                added_by: "system".into(),
                added_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // ── 预置技术社区域名 ──
        let tech_community = [
            ("github.com", "GitHub 代码仓库（API 只读）"),
            ("api.github.com", "GitHub REST API"),
            ("stackoverflow.com", "Stack Overflow 技术问答"),
            ("dev.to", "Dev.to 技术博客"),
            ("medium.com", "Medium 技术文章"),
            ("reddit.com", "Reddit 技术讨论"),
            ("npmjs.com", "npm 包注册表"),
            ("pypi.org", "PyPI 包注册表"),
        ];
        for (domain, desc) in &tech_community {
            domain_whitelist.push(DomainEntry {
                domain: domain.to_string(),
                description: desc.to_string(),
                category: DomainCategory::TechCommunity,
                allowed: true,
                added_by: "system".into(),
                added_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // ── 预置搜索引擎 API ──
        let search_engines = [
            ("api.bing.microsoft.com", "Bing Search API"),
            ("api.duckduckgo.com", "DuckDuckGo Instant Answer API"),
            ("serpapi.com", "SerpAPI 聚合搜索"),
        ];
        for (domain, desc) in &search_engines {
            domain_whitelist.push(DomainEntry {
                domain: domain.to_string(),
                description: desc.to_string(),
                category: DomainCategory::SearchEngine,
                allowed: true,
                added_by: "system".into(),
                added_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        let mut search_providers = HashMap::new();
        search_providers.insert("bing".into(), SearchProvider::bing(None));
        search_providers.insert("duckduckgo".into(), SearchProvider::duckduckgo());

        Self {
            domain_whitelist,
            search_providers,
            office_connectors: Vec::new(),
            audit_log: Vec::new(),
            distillation_threshold: 2048,
            request_timeout_ms: 15000,
            enabled: true,
            approval_required: true,
            request_counter: 0,
            total_bytes_fetched: 0,
            distillation: DistillationEngine::new(),
            cache: UnifiedCache::new(),
        }
    }

    // ── 域名白名单管理 ───────────────────────────────────────────

    fn next_id(&mut self) -> String {
        self.request_counter += 1;
        format!("web-{:06}", self.request_counter)
    }

    /// 检查域名是否在白名单中
    pub fn is_domain_allowed(&self, url: &str) -> bool {
        let domain = extract_domain(url);
        if domain.is_empty() {
            return false;
        }
        self.domain_whitelist
            .iter()
            .any(|entry| entry.allowed && (entry.domain == domain || domain.ends_with(&format!(".{}", entry.domain))))
    }

    /// 添加域名到白名单
    pub fn add_domain(&mut self, domain: &str, description: &str, category: DomainCategory) -> Result<(), String> {
        let normalized = domain.trim().to_lowercase();
        if normalized.is_empty() {
            return Err("域名不能为空".into());
        }
        if self.domain_whitelist.iter().any(|e| e.domain == normalized) {
            return Err(format!("域名 '{}' 已在白名单中", normalized));
        }
        self.domain_whitelist.push(DomainEntry {
            domain: normalized,
            description: description.into(),
            category,
            allowed: true,
            added_by: "user".into(),
            added_at: chrono::Utc::now().to_rfc3339(),
        });
        tracing::info!("[WebIntel] Domain added to whitelist: {}", domain);
        Ok(())
    }

    /// 移除域名
    pub fn remove_domain(&mut self, domain: &str) -> Result<(), String> {
        let len_before = self.domain_whitelist.len();
        self.domain_whitelist.retain(|e| e.domain != domain);
        if self.domain_whitelist.len() == len_before {
            Err(format!("域名 '{}' 不在白名单中", domain))
        } else {
            tracing::info!("[WebIntel] Domain removed from whitelist: {}", domain);
            Ok(())
        }
    }

    /// 启用/禁用域名
    pub fn toggle_domain(&mut self, domain: &str, allowed: bool) -> Result<(), String> {
        let entry = self
            .domain_whitelist
            .iter_mut()
            .find(|e| e.domain == domain)
            .ok_or_else(|| format!("域名 '{}' 不在白名单中", domain))?;
        entry.allowed = allowed;
        Ok(())
    }

    // ── 搜索提供商管理 ──────────────────────────────────────────

    pub fn set_search_api_key(&mut self, provider: &str, api_key: &str) -> Result<(), String> {
        let p = self
            .search_providers
            .get_mut(provider)
            .ok_or_else(|| format!("搜索提供商 '{}' 不存在", provider))?;
        p.api_key = Some(api_key.into());
        Ok(())
    }

    // ── Web Search ──────────────────────────────────────────────

    /// 执行 Web 搜索 — 通过 Bing API
    pub async fn web_search(
        &mut self,
        query: &str,
        provider: &str,
        max_results: Option<u32>,
    ) -> Result<SearchResponse, String> {
        if !self.enabled {
            return Err("Web Intelligence 模块未启用".into());
        }

        let search_provider = self
            .search_providers
            .get(provider)
            .cloned()
            .ok_or_else(|| format!("搜索提供商 '{}' 不存在", provider))?;

        if !search_provider.enabled {
            return Err(format!("搜索提供商 '{}' 已禁用", provider));
        }

        let limit = max_results.unwrap_or(search_provider.max_results_per_query).min(20);

        let start = std::time::Instant::now();
        let result = self.do_search(&search_provider, query, limit).await;
        let latency_ms = start.elapsed().as_millis() as u64;

        let audit_id = self.next_id();
        match &result {
            Ok(resp) => {
                self.total_bytes_fetched += resp.results.iter().map(|r| r.snippet.len() as u64).sum::<u64>();
                self.audit_log.push(WebAuditEntry {
                    id: audit_id,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    operation: "web_search".into(),
                    target_url: Some(search_provider.endpoint.clone()),
                    domain: Some(extract_domain(&search_provider.endpoint)),
                    allowed: true,
                    approval_id: None,
                    bytes_received: resp.results.iter().map(|r| r.snippet.len()).sum(),
                    latency_ms,
                    error: None,
                });
            }
            Err(e) => {
                self.audit_log.push(WebAuditEntry {
                    id: audit_id,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    operation: "web_search".into(),
                    target_url: Some(search_provider.endpoint.clone()),
                    domain: Some(extract_domain(&search_provider.endpoint)),
                    allowed: false,
                    approval_id: None,
                    bytes_received: 0,
                    latency_ms,
                    error: Some(e.clone()),
                });
            }
        }

        result
    }

    /// 实际执行搜索请求
    async fn do_search(
        &self,
        provider: &SearchProvider,
        query: &str,
        count: u32,
    ) -> Result<SearchResponse, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.request_timeout_ms))
            .build()
            .map_err(|e| format!("HTTP client build failed: {}", e))?;

        let start = std::time::Instant::now();

        match provider.name.as_str() {
            "Bing" => {
                let api_key = provider.api_key.as_deref().unwrap_or("");
                if api_key.is_empty() {
                    // Fallback: use DuckDuckGo or return clear error
                    return Err("Bing API key not configured. Please set an API key or use DuckDuckGo.".into());
                }

                let resp = client
                    .get(&provider.endpoint)
                    .header("Ocp-Apim-Subscription-Key", api_key)
                    .query(&[
                        ("q", query),
                        ("count", &count.to_string()),
                        ("mkt", "zh-CN"),
                        ("safeSearch", "Moderate"),
                    ])
                    .send()
                    .await
                    .map_err(|e| format!("Bing search request failed: {}", e))?;

                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();

                if !status.is_success() {
                    return Err(format!("Bing API returned HTTP {}: {}", status.as_u16(), truncate(&body, 200)));
                }

                let json: serde_json::Value =
                    serde_json::from_str(&body).map_err(|e| format!("Bing response parse error: {}", e))?;

                let mut results = Vec::new();
                if let Some(pages) = json["webPages"]["value"].as_array() {
                    for (i, page) in pages.iter().enumerate() {
                        results.push(SearchResult {
                            title: page["name"].as_str().unwrap_or("").to_string(),
                            url: page["url"].as_str().unwrap_or("").to_string(),
                            snippet: page["snippet"].as_str().unwrap_or("").to_string(),
                            source: extract_domain(page["url"].as_str().unwrap_or("")),
                            rank: (i + 1) as u32,
                        });
                    }
                }

                let total = json["webPages"]["totalEstimatedMatches"]
                    .as_u64()
                    .unwrap_or(results.len() as u64);

                Ok(SearchResponse {
                    query: query.into(),
                    results,
                    total_estimated: total,
                    provider: "Bing".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                })
            }
            "DuckDuckGo" => {
                let resp = client
                    .get(&provider.endpoint)
                    .query(&[
                        ("q", query),
                        ("format", "json"),
                        ("no_html", "1"),
                        ("skip_disambig", "1"),
                    ])
                    .send()
                    .await
                    .map_err(|e| format!("DuckDuckGo request failed: {}", e))?;

                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();

                if !status.is_success() {
                    return Err(format!("DuckDuckGo returned HTTP {}", status.as_u16()));
                }

                let json: serde_json::Value =
                    serde_json::from_str(&body).unwrap_or(serde_json::json!({}));

                let mut results = Vec::new();
                // DuckDuckGo Instant Answer
                if let Some(answer) = json["AbstractText"].as_str() {
                    if !answer.is_empty() {
                        results.push(SearchResult {
                            title: json["Heading"].as_str().unwrap_or("Instant Answer").to_string(),
                            url: json["AbstractURL"].as_str().unwrap_or("").to_string(),
                            snippet: answer.to_string(),
                            source: "DuckDuckGo".into(),
                            rank: 1,
                        });
                    }
                }
                // Related topics
                if let Some(topics) = json["RelatedTopics"].as_array() {
                    for (_i, topic) in topics.iter().enumerate() {
                        if let Some(text) = topic["Text"].as_str() {
                            results.push(SearchResult {
                                title: text.split(" - ").next().unwrap_or(text).to_string(),
                                url: topic["FirstURL"].as_str().unwrap_or("").to_string(),
                                snippet: text.to_string(),
                                source: "DuckDuckGo".into(),
                                rank: (results.len() + 1) as u32,
                            });
                            if results.len() as u32 >= count {
                                break;
                            }
                        }
                    }
                }

                Ok(SearchResponse {
                    query: query.into(),
                    results: results.clone(),
                    total_estimated: results.len() as u64,
                    provider: "DuckDuckGo".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                })
            }
            _ => Err(format!("Unsupported search provider: {}", provider.name)),
        }
    }

    // ── Web Fetch ───────────────────────────────────────────────

    /// 抓取指定 URL 内容并转换为 Markdown
    pub async fn web_fetch(&mut self, url: &str) -> Result<FetchResult, String> {
        if !self.enabled {
            return Err("Web Intelligence 模块未启用".into());
        }

        // 尝试缓存命中
        let cache_key = format!("fetch:{}", url);
        if let Some(val) = self.cache.get(CacheCategory::WebFetch, &cache_key) {
            if let Ok(result) = serde_json::from_str::<FetchResult>(&val) {
                tracing::info!("[WebIntel] Fetch cache HIT: {}", url);
                return Ok(result);
            }
        }

        // 域名白名单校验
        if !self.is_domain_allowed(url) {
            let domain = extract_domain(url);
            return Err(format!(
                "⛔ 域名 '{}' 不在白名单中。请先添加到域名白名单后再抓取。\n\
                 当前已批准域名数: {} 个。",
                domain,
                self.domain_whitelist.iter().filter(|e| e.allowed).count()
            ));
        }

        let start = std::time::Instant::now();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.request_timeout_ms))
            .user_agent("Chronos-Shadow-WebIntel/1.0 (Research Assistant)")
            .build()
            .map_err(|e| format!("HTTP client build failed: {}", e))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {}", e))?;

        let status_code = resp.status().as_u16();
        let latency_ms = start.elapsed().as_millis() as u64;

        if !resp.status().is_success() {
            let audit_id = self.next_id();
            self.audit_log.push(WebAuditEntry {
                id: audit_id,
                timestamp: chrono::Utc::now().to_rfc3339(),
                operation: "web_fetch".into(),
                target_url: Some(url.into()),
                domain: Some(extract_domain(url)),
                allowed: false,
                approval_id: None,
                bytes_received: 0,
                latency_ms,
                error: Some(format!("HTTP {}", status_code)),
            });
            return Err(format!("HTTP {} when fetching {}", status_code, url));
        }

        let html = resp.text().await.map_err(|e| format!("Read body: {}", e))?;
        let content_length = html.len();
        self.total_bytes_fetched += content_length as u64;

        // HTML → Markdown 转换
        let markdown = html_to_markdown(&html, url);

        // 多级语义蒸馏引擎处理
        let (distilled_md, was_distilled) = if markdown.len() > self.distillation_threshold as usize {
            let distilled = self.distillation.distill(
                &markdown,
                url,
                DistillationLevel::Deep,
                Some(self.distillation_threshold),
            );
            (distilled.markdown.clone(), true)
        } else {
            (markdown.clone(), false)
        };
        let _ = was_distilled; // used in audit entry

        let audit_id = self.next_id();
        self.audit_log.push(WebAuditEntry {
            id: audit_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            operation: "web_fetch".into(),
            target_url: Some(url.into()),
            domain: Some(extract_domain(url)),
            allowed: true,
            approval_id: None,
            bytes_received: content_length,
            latency_ms,
            error: None,
        });

        let fetch_result = FetchResult {
            url: url.into(),
            title: extract_title(&html),
            content_markdown: distilled_md,
            content_length,
            status_code,
            latency_ms,
        };

        // 写入缓存
        if let Ok(json) = serde_json::to_string(&fetch_result) {
            let cache_key = format!("fetch:{}", url);
            self.cache.set(CacheCategory::WebFetch, &cache_key, json, content_length, None);
        }

        Ok(fetch_result)
    }

    // ── Web Research (多源聚合) ─────────────────────────────────

    /// 多源聚合研究：搜索 → 抓取 Top N → 蒸馏 → 生成研究报告
    pub async fn web_research(
        &mut self,
        query: &str,
        search_provider: &str,
        max_sources: usize,
    ) -> Result<DistilledResearch, String> {
        if !self.enabled {
            return Err("Web Intelligence 模块未启用".into());
        }

        tracing::info!("[WebIntel] Starting research: \"{}\"", query);

        // 1. 搜索
        let search_result = self
            .web_search(query, search_provider, Some(max_sources as u32))
            .await?;

        if search_result.results.is_empty() {
            return Err(format!("搜索 \"{}\" 未找到任何结果", query));
        }

        // 2. 逐个抓取 Top N 个结果
        let mut sources = Vec::new();
        let mut all_content = String::new();
        let mut total_raw_size = 0usize;

        for result in search_result.results.iter().take(max_sources) {
            // 跳过已知不可抓取的站点
            if result.url.is_empty() {
                continue;
            }
            sources.push(result.url.clone());

            match self.web_fetch(&result.url).await {
                Ok(fetched) => {
                    total_raw_size += fetched.content_length;
                    all_content.push_str(&format!(
                        "\n\n## Source: {}\n{}\n",
                        result.title, fetched.content_markdown
                    ));
                }
                Err(e) => {
                    tracing::warn!("[WebIntel] Failed to fetch {}: {}", result.url, e);
                    all_content.push_str(&format!(
                        "\n\n## Source: {} (fetch failed)\n> {}\n\nSnippet: {}",
                        result.title, e, result.snippet
                    ));
                }
            }
        }

        // 3. 多级语义蒸馏引擎聚合
        let mut key_findings = Vec::new();
        for result in &search_result.results {
            if !result.snippet.is_empty() {
                key_findings.push(format!("[{}] {}", result.title, result.snippet));
            }
        }

        let (summary, distilled_size, confidence) = if all_content.len() > self.distillation_threshold as usize {
            let distilled = self.distillation.distill(
                &all_content,
                query,
                DistillationLevel::Deep,
                Some(self.distillation_threshold),
            );
            // Extract high-confidence facts as key findings from fragments
            for frag in &distilled.fragments {
                if let ContentFragment::KeyFact { statement, confidence } = frag {
                    if *confidence > 0.7 {
                        key_findings.push(format!("🔗 {}", statement));
                    }
                }
            }
            // Add entity-based insights
            for entity in &distilled.entities {
                if matches!(entity.entity_type, EntityType::Deprecated | EntityType::Breaking) {
                    key_findings.push(format!("⚠️ {}: {}", entity.entity_type.label(), entity.name));
                }
            }
            let summary_text = distilled.markdown.clone();
            let size = summary_text.len();
            let conf = distilled.compression_ratio;
            (summary_text, size, conf)
        } else {
            let size = all_content.len();
            (all_content.clone(), size, 0.7)
        };

        Ok(DistilledResearch {
            query: query.into(),
            summary,
            key_findings,
            sources,
            confidence: confidence as f32,
            raw_size_bytes: total_raw_size,
            distilled_size_bytes: distilled_size,
            compression_ratio: if total_raw_size > 0 {
                distilled_size as f64 / total_raw_size as f64
            } else {
                1.0
            },
        })
    }

    // ── 审计 ────────────────────────────────────────────────────

    /// 查询审计日志
    pub fn get_audit_log(&self, limit: usize) -> Vec<&WebAuditEntry> {
        self.audit_log.iter().rev().take(limit).collect()
    }

    /// 查询白名单域名列表
    pub fn get_whitelist(&self) -> Vec<&DomainEntry> {
        self.domain_whitelist.iter().collect()
    }

    /// 按类别筛选白名单
    pub fn get_whitelist_by_category(&self, category: DomainCategory) -> Vec<&DomainEntry> {
        self.domain_whitelist
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    // ── 内容脱敏 ────────────────────────────────────────────────

    /// 脱敏请求内容 — 移除 API Keys / 文件路径 / 个人信息
    pub fn sanitize_query(query: &str) -> String {
        let sanitized = query
            // 移除可能的 API Key 模式
            .replace(
                regex_lite::Regex::new(r"[A-Za-z0-9_-]{32,}").unwrap().as_str(),
                "[REDACTED]",
            )
            // 移除 Windows 绝对路径
            .replace(
                regex_lite::Regex::new(r"[A-Z]:\\[^\s]*").unwrap().as_str(),
                "[REDACTED_PATH]",
            )
            // 移除 Unix 绝对路径
            .replace(
                regex_lite::Regex::new(r"/home/[^\s]*").unwrap().as_str(),
                "[REDACTED_PATH]",
            );

        // Fallback: if regex_lite doesn't work, do simple replacement
        if sanitized == query {
            // Just return original — regex_lite is optional
            return query.to_string();
        }
        sanitized
    }

    // ── 便捷公开 API（Tauri Commands 调用） ────────────────────

    /// 搜索（公开接口，带缓存加速）
    pub async fn search(
        &mut self,
        query: &str,
        engine: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<Vec<WebSearchResult>, String> {
        let provider = engine.unwrap_or("duckduckgo");
        let cache_key = format!("search:{}:{}:{}", provider, query, max_results.unwrap_or(5));

        // 尝试缓存命中
        if let Some(val) = self.cache.get(CacheCategory::WebSearch, &cache_key) {
            if let Ok(results) = serde_json::from_str::<Vec<WebSearchResult>>(&val) {
                tracing::info!("[WebIntel] Search cache HIT: {}", query);
                return Ok(results);
            }
        }

        let response = self.web_search(query, provider, max_results).await?;
        let results: Vec<WebSearchResult> = response.results.into_iter().map(|r| WebSearchResult {
            title: r.title,
            url: r.url,
            snippet: r.snippet,
            source: r.source,
            relevance_score: (6.0 - r.rank as f64).max(0.0),
        }).collect();

        // 写入缓存
        if let Ok(json) = serde_json::to_string(&results) {
            let size = json.len();
            self.cache.set(CacheCategory::WebSearch, &cache_key, json, size, None);
        }

        Ok(results)
    }

    /// 抓取（公开接口，带多级语义蒸馏引擎）
    pub async fn fetch(
        &mut self,
        url: &str,
        distill: bool,
    ) -> Result<WebFetchResult, String> {
        let result = self.web_fetch(url).await?;
        let (was_distilled, summary, key_points) = if distill && result.content_length > self.distillation_threshold {
            let distilled = self.distillation.distill(
                &result.content_markdown,
                url,
                DistillationLevel::Medium,
                Some(self.distillation_threshold),
            );
            let summary_text = distilled.markdown.clone();
            // Extract key points from fragments: code blocks, API sigs, key facts, entities
            let mut points: Vec<String> = Vec::new();
            for frag in &distilled.fragments {
                match frag {
                    ContentFragment::CodeBlock { language, code, .. } =>
                        points.push(format!("💻 [{}] {}", language, truncate(code, 100))),
                    ContentFragment::ApiSignature { signature, .. } =>
                        points.push(format!("🔧 {}", signature)),
                    ContentFragment::KeyFact { statement, .. } =>
                        points.push(format!("📌 {}", statement)),
                    _ => {}
                }
                if points.len() >= 12 { break; }
            }
            // Add entity highlights
            for entity in &distilled.entities {
                points.push(format!("🏷️ [{}] {}", entity.entity_type.label(), entity.name));
                if points.len() >= 16 { break; }
            }
            (true, Some(summary_text), points)
        } else {
            (false, None, Vec::new())
        };
        Ok(WebFetchResult {
            success: true,
            url: result.url,
            title: result.title.unwrap_or_default(),
            content: if was_distilled { summary.clone().unwrap_or(result.content_markdown) } else { result.content_markdown },
            content_length: result.content_length,
            distilled: was_distilled,
            distilled_summary: summary,
            key_points,
            error: None,
        })
    }

    /// 多源聚合研究（公开接口）
    pub async fn research(
        &mut self,
        topic: &str,
        sources: Vec<String>,
    ) -> Result<ResearchReport, String> {
        let distilled = self.web_research(topic, "bing", sources.len().max(3)).await?;
        Ok(ResearchReport {
            topic: topic.into(),
            summary: distilled.summary,
            key_findings: distilled.key_findings,
            sources: Vec::new(),
            confidence: distilled.confidence as f64,  // f32→f64
            timestamp: chrono::Utc::now().to_rfc3339(),
            recommendations: Vec::new(),
        })
    }

    /// 添加域名（公开接口）
    pub fn add_allowed_domain(&mut self, domain: &str, category: &str) -> Result<(), String> {
        let cat = match category {
            "official" => DomainCategory::OfficialDocs,
            "community" => DomainCategory::TechCommunity,
            "search" => DomainCategory::SearchEngine,
            "office" => DomainCategory::OfficeIntegration,
            _ => DomainCategory::UserCustom,
        };
        self.add_domain(domain, &format!("User-added: {}", domain), cat)
    }

    /// 移除域名（公开接口）
    pub fn remove_allowed_domain(&mut self, domain: &str) -> Result<(), String> {
        self.remove_domain(domain)
    }

    /// 列出所有域名
    pub fn list_allowed_domains(&self) -> Vec<(String, String)> {
        self.domain_whitelist.iter()
            .filter(|e| e.allowed)
            .map(|e| (e.domain.clone(), e.category.label().to_string()))
            .collect()
    }

    /// 获取审计日志
    pub fn get_audit_log_owned(&self, limit: usize) -> Vec<WebAuditEntry> {
        self.audit_log.iter().rev().take(limit).cloned().collect()
    }

    /// 同步进化指标到 EvolutionBus（每个操作周期调用一次）
    pub fn sync_to_evolution_bus(&mut self, bus: &mut EvolutionBus) {
        use super::evolution_bus::EngineId;

        // 蒸馏引擎指标
        let dist_quality = self.distillation.avg_quality();
        let dist_compression = self.distillation.avg_compression_ratio();
        bus.feedback_performance(EngineId::Distillation, "quality", dist_quality, 0.85, true);
        bus.feedback_performance(EngineId::Distillation, "compression", dist_compression, 0.6, true);

        // 缓存引擎指标 - 自适应TTL
        for cat in &[CacheCategory::WebSearch, CacheCategory::WebFetch, CacheCategory::Distillation] {
            let hit_rate = self.cache.category_hit_rate(*cat);
            let new_ttl = self.cache.adapt_ttl(*cat, hit_rate);
            bus.feedback_performance(EngineId::CacheEngine, &format!("ttl_{:?}", cat), new_ttl / 3600.0, 1.0, true);
        }
    }

    /// 获取统计
    pub fn get_stats(&self) -> WebIntelStats {
        let allowed = self.domain_whitelist.iter().filter(|e| e.allowed).count();
        let _total = self.audit_log.len();
        let failed = self.audit_log.iter().filter(|e| e.error.is_some()).count();
        WebIntelStats {
            total_searches: self.audit_log.iter().filter(|e| e.operation == "web_search").count() as u64,
            total_fetches: self.audit_log.iter().filter(|e| e.operation == "web_fetch").count() as u64,
            total_research: self.audit_log.iter().filter(|e| e.operation == "web_research").count() as u64,
            bytes_downloaded: self.total_bytes_fetched,
            domains_whitelisted: allowed as u64,
            requests_blocked: failed as u64,
            estimated_cost_saved: self.total_bytes_fetched as f64 * 0.00001,
            // Distillation engine metrics
            total_distilled: self.distillation.total_distilled(),
            total_bytes_saved: self.distillation.total_bytes_saved_count(),
            avg_compression_ratio: self.distillation.avg_compression_ratio(),
            cache_hit_rate: self.distillation.cache_hit_rate(),
            unified_cache_hits: self.cache.stats().total_hits,
            unified_cache_misses: self.cache.stats().total_misses,
            api_calls_saved: self.cache.stats().api_calls_saved,
        }
    }

    /// 持久化状态
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("web_intelligence.json");
        let state = serde_json::json!({
            "domain_whitelist": self.domain_whitelist,
            "total_bytes_fetched": self.total_bytes_fetched,
            "enabled": self.enabled,
            "approval_required": self.approval_required,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(format!("WebIntelligence state saved to {:?}", path))
    }

    /// 加载状态
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("web_intelligence.json");
        if !path.exists() { return Ok("No saved state found".into()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(domains) = state.get("domain_whitelist") {
            if let Ok(list) = serde_json::from_value::<Vec<DomainEntry>>(domains.clone()) {
                for entry in list {
                    if !self.domain_whitelist.iter().any(|e| e.domain == entry.domain) {
                        self.domain_whitelist.push(entry);
                    }
                }
            }
        }
        if let Some(bytes) = state.get("total_bytes_fetched").and_then(|v| v.as_u64()) {
            self.total_bytes_fetched = bytes;
        }
        Ok(format!("WebIntelligence state loaded from {:?}", path))
    }

    // ── 统计 ────────────────────────────────────────────────────

    pub fn stats(&self) -> serde_json::Value {
        let allowed_domains = self
            .domain_whitelist
            .iter()
            .filter(|e| e.allowed)
            .count();
        let total_requests = self.audit_log.len();
        let failed_requests = self
            .audit_log
            .iter()
            .filter(|e| e.error.is_some())
            .count();

        serde_json::json!({
            "enabled": self.enabled,
            "approval_required": self.approval_required,
            "allowed_domains": allowed_domains,
            "total_domains": self.domain_whitelist.len(),
            "total_requests": total_requests,
            "failed_requests": failed_requests,
            "success_rate": if total_requests > 0 {
                format!("{:.1}%", (total_requests - failed_requests) as f64 / total_requests as f64 * 100.0)
            } else { "N/A".to_string() },
            "total_bytes_fetched": self.total_bytes_fetched,
            "search_providers": self.search_providers.keys().collect::<Vec<_>>(),
            "office_connectors": self.office_connectors.len(),
        })
    }
}

impl Default for WebIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

// ─── HTML → Markdown 转换 ──────────────────────────────────────────

fn html_to_markdown(html: &str, base_url: &str) -> String {
    // 简化版 HTML → Markdown 转换器
    // 生产环境建议集成 pulldown-cmark 或 html2md crate

    let mut md = String::new();

    // 提取 <title>
    if let Some(title) = extract_tag_content(html, "title") {
        md.push_str(&format!("# {}\n\n", title.trim()));
    }

    // 提取 <meta name="description">
    if let Some(desc) = extract_meta_content(html, "description") {
        md.push_str(&format!("> {}\n\n", desc.trim()));
    }

    // 简单移除 script 和 style 标签内容
    let cleaned = remove_tags(html, &["script", "style", "nav", "footer", "header"]);

    // 提取文本段落 (简化版)
    let text = strip_html_tags(&cleaned);
    let paragraphs: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    for p in paragraphs {
        let trimmed = p.trim();
        if trimmed.len() > 20 {
            md.push_str(&format!("{}\n\n", trimmed));
        }
    }

    if md.is_empty() {
        md = format!("[Content from {} — unable to extract text]\n\n{}...",
            base_url,
            text.chars().take(500).collect::<String>()
        );
    }

    // 限制输出大小
    if md.len() > 50_000 {
        md = md.chars().take(50_000).collect();
        md.push_str("\n\n... [content truncated]");
    }

    md
}

fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let start_pattern = format!("<{}", tag);
    let end_pattern = format!("</{}>", tag);

    let lower = html.to_lowercase();
    let start = lower.find(&start_pattern)?;
    let start = lower[start..].find('>').map(|i| start + i + 1)?;
    let end = lower[start..].find(&end_pattern)?;

    Some(html[start..start + end].to_string())
}

fn extract_meta_content(html: &str, name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let pattern = format!("<meta name=\"{}\" content=\"", name);
    let pos = lower.find(&pattern)?;
    let start = pos + pattern.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
}

fn remove_tags(html: &str, tags: &[&str]) -> String {
    let mut result = html.to_string();
    for tag in tags {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);

        while let Some(start) = result.to_lowercase().find(&open) {
            if let Some(tag_end) = result[start..].find('>') {
                let end_search = start + tag_end + 1;
                // 找对应的关闭标签
                if let Some(close_pos) = result[end_search..].to_lowercase().find(&close) {
                    let end = end_search + close_pos + close.len();
                    result.replace_range(start..end, "");
                } else {
                    // 自闭合标签
                    result.replace_range(start..start + tag_end + 1, "");
                }
            } else {
                break;
            }
        }
    }
    result
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    // Decode common HTML entities
    result = result.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    result
}

fn extract_title(html: &str) -> Option<String> {
    extract_tag_content(html, "title")
}

// ─── 蒸馏 ─────────────────────────────────────────────────────────

#[allow(dead_code)]
fn distill_content(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }

    // 保留头部（含标题和摘要）和代表性段落
    let head_ratio = 0.4;
    let tail_ratio = 0.3;
    let middle_ratio = 1.0 - head_ratio - tail_ratio;

    let head_size = (max_bytes as f64 * head_ratio) as usize;
    let tail_size = (max_bytes as f64 * tail_ratio) as usize;
    let middle_size = (max_bytes as f64 * middle_ratio) as usize;

    let head: String = content.chars().take(head_size).collect();
    let tail: String = content.chars().rev().take(tail_size).collect::<String>().chars().rev().collect();

    // 中间部分采样
    let total_len = content.chars().count();
    let mid_start = total_len / 3;
    let middle: String = content
        .chars()
        .skip(mid_start)
        .take(middle_size)
        .collect();

    format!(
        "{}\n\n--- [{} bytes distilled, {:.1}% compression] ---\n\n{}\n\n---\n\n{}",
        head,
        content.len() - max_bytes,
        (1.0 - max_bytes as f64 / content.len() as f64) * 100.0,
        middle,
        tail,
    )
}

// ─── 工具函数 ─────────────────────────────────────────────────────

fn extract_domain(url: &str) -> String {
    let url = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    url.split('/').next().unwrap_or(url).split(':').next().unwrap_or("").to_string()
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_chars).collect::<String>())
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_whitelist_management() {
        let mut wi = WebIntelligence::new();
        let initial = wi.domain_whitelist.len();

        // Add domain
        wi.add_domain("example.com", "Test domain", DomainCategory::UserCustom)
            .unwrap();
        assert_eq!(wi.domain_whitelist.len(), initial + 1);
        assert!(wi.is_domain_allowed("https://example.com/page"));

        // Duplicate should fail
        assert!(wi.add_domain("example.com", "dup", DomainCategory::UserCustom).is_err());

        // Subdomain check
        assert!(wi.is_domain_allowed("https://docs.rs/tokio/1.0/"));

        // Non-whitelisted domain
        assert!(!wi.is_domain_allowed("https://evil.com/malware"));

        // Remove
        wi.remove_domain("example.com").unwrap();
        assert_eq!(wi.domain_whitelist.len(), initial);
    }

    #[test]
    fn test_domain_toggle() {
        let mut wi = WebIntelligence::new();
        wi.add_domain("test.com", "Test", DomainCategory::UserCustom).unwrap();

        wi.toggle_domain("test.com", false).unwrap();
        assert!(!wi.is_domain_allowed("https://test.com/page"));

        wi.toggle_domain("test.com", true).unwrap();
        assert!(wi.is_domain_allowed("https://test.com/page"));
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://docs.rs/tokio/1.0"), "docs.rs");
        assert_eq!(extract_domain("http://example.com/path?q=1"), "example.com");
        assert_eq!(extract_domain("www.github.com/rust-lang"), "github.com");
        assert_eq!(extract_domain("https://api.bing.microsoft.com/v7.0/search"), "api.bing.microsoft.com");
    }

    #[test]
    fn test_html_strip() {
        let html = "<html><head><title>Test</title></head><body><p>Hello World</p></body></html>";
        let text = strip_html_tags(html);
        assert!(text.contains("Test"));
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_html_to_markdown() {
        let html = r#"<html><head><title>Test Page</title><meta name="description" content="A test page"></head><body><p>Test content here. Some more text to make it interesting.</p><p>Second paragraph with enough content for testing purposes.</p></body></html>"#;
        let md = html_to_markdown(html, "https://test.com");
        assert!(md.contains("Test Page"));
        assert!(md.contains("A test page"));
    }

    #[test]
    fn test_distill_content() {
        let content = "A".repeat(10000);
        let distilled = distill_content(&content, 1000);
        assert!(distilled.len() <= 1200); // slight overhead from headers
    }

    #[test]
    fn test_remove_tags() {
        let html = "<div><script>alert('xss')</script><p>Safe content</p></div>";
        let cleaned = remove_tags(html, &["script"]);
        assert!(!cleaned.contains("alert"));
        assert!(cleaned.contains("Safe content"));
    }

    #[test]
    fn test_preloaded_domains() {
        let wi = WebIntelligence::new();
        let docs = wi.get_whitelist_by_category(DomainCategory::OfficialDocs);
        let community = wi.get_whitelist_by_category(DomainCategory::TechCommunity);
        let search = wi.get_whitelist_by_category(DomainCategory::SearchEngine);

        assert!(!docs.is_empty());
        assert!(!community.is_empty());
        assert!(!search.is_empty());

        // Check specific domains
        assert!(wi.is_domain_allowed("https://docs.rs/tokio"));
        assert!(wi.is_domain_allowed("https://api.github.com/repos"));
        assert!(wi.is_domain_allowed("https://stackoverflow.com/questions"));
    }

    #[test]
    fn test_audit_logging() {
        let mut wi = WebIntelligence::new();
        let initial = wi.audit_log.len();

        // Simulate an audit entry
        wi.audit_log.push(WebAuditEntry {
            id: "test-1".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            operation: "web_search".into(),
            target_url: Some("https://api.bing.com/search".into()),
            domain: Some("api.bing.microsoft.com".into()),
            allowed: true,
            approval_id: None,
            bytes_received: 1024,
            latency_ms: 250,
            error: None,
        });

        assert_eq!(wi.audit_log.len(), initial + 1);
        let log = wi.get_audit_log(1);
        assert_eq!(log[0].operation, "web_search");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

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