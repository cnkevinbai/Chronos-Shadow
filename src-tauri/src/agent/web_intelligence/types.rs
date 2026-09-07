use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::agent::distillation_engine::DistillationEngine;
use crate::agent::cache_engine::UnifiedCache;

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
    pub(super) request_counter: u64,
    /// 累计抓取字节数
    pub total_bytes_fetched: u64,
    /// 多级语义蒸馏引擎
    pub distillation: DistillationEngine,
    /// 统一缓存引擎（搜索/抓取/蒸馏跨模块缓存）
    pub cache: UnifiedCache,
}
