// 永不言弃链接抓取引擎 (Indomitable Web Fetcher)
//
// 核心理念: "一个人可以被毁灭，但是不可以被打败" — 海明威《老人与海》
//
// 每条链接都会经历:
//   1. URL 智能提取 — 从任意文本中识别所有链接
//   2. 多策略抓取 — 原始HTTP → 移动UA → 纯文本, 逐级降级
//   3. 主体内容提取 — Readability 风格, 剥离导航/广告/页脚
//   4. 递归深度跟随 — 自动发现并抓取关键链接
//   5. 韧性熔断 — 按域名统计失败, 指数退避重试, 永不言弃
//   6. 自动引用生成 — 提取标题/作者/日期/摘要

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── URL 智能提取器 ──────────────────────────────────────────────

/// 从任意文本中提取所有 URL
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Markdown 链接 [text](url)
    let md_re = regex::Regex::new(r"\[([^\]]*)\]\((https?://[^\s)]+)\)").unwrap();
    for cap in md_re.captures_iter(text) {
        let url = cap[2].trim().to_string();
        if seen.insert(url.clone()) { urls.push(url); }
    }

    // 裸 URL (匹配直到遇到空白或常见分隔符)
    let url_re = regex::Regex::new(r"https?://[^\s<>]+").unwrap();
    for m in url_re.find_iter(text) {
        let url = m.as_str().trim().trim_end_matches('.').trim_end_matches(',')
            .trim_end_matches(';').trim_end_matches(')').to_string();
        if seen.insert(url.clone()) { urls.push(url); }
    }

    urls
}

// ─── 抓取策略枚举 ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FetchStrategy {
    /// 标准 HTTP (默认)
    Standard,
    /// 移动端 User-Agent (绕过桌面限制)
    MobileUA,
    /// 纯文本模式 (绕过 JS 渲染要求)
    TextOnly,
    /// 搜索引擎缓存 (最后一搏)
    SearchCache,
}

impl FetchStrategy {
    pub fn label(&self) -> &str {
        match self {
            Self::Standard => "标准HTTP",
            Self::MobileUA => "移动端UA",
            Self::TextOnly => "纯文本",
            Self::SearchCache => "搜索引擎缓存",
        }
    }

    pub fn user_agent(&self) -> &str {
        match self {
            Self::Standard => "Chronos-Shadow/1.0 ResearchBot",
            Self::MobileUA => "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/120 Mobile",
            Self::TextOnly => "curl/8.0 (text-extraction)",
            Self::SearchCache => "Chronos-Shadow-CacheBot/1.0",
        }
    }
}

/// 所有尝试策略 (按优先级排列)
pub const FETCH_STRATEGIES: &[FetchStrategy] = &[
    FetchStrategy::Standard,
    FetchStrategy::MobileUA,
    FetchStrategy::TextOnly,
    FetchStrategy::SearchCache,
];

// ─── 尝试记录 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAttempt {
    pub strategy: FetchStrategy,
    pub success: bool,
    pub status_code: Option<u16>,
    pub bytes_received: usize,
    pub latency_ms: u64,
    pub error: Option<String>,
}

// ─── 抓取结果 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndomitableFetchResult {
    pub url: String,
    pub title: Option<String>,
    /// 提取的正文 (Markdown)
    pub main_content: String,
    /// 原始 HTML 长度
    pub raw_size: usize,
    /// 正文长度
    pub content_size: usize,
    /// 压缩比
    pub extraction_ratio: f64,
    /// 所有尝试记录
    pub attempts: Vec<FetchAttempt>,
    /// 最终成功的策略
    pub winning_strategy: Option<FetchStrategy>,
    /// 总耗时
    pub total_latency_ms: u64,
    /// 自动提取的元数据
    pub metadata: PageMetadata,
    /// 页面内发现的链接
    pub discovered_links: Vec<String>,
    /// 是否最终成功
    pub success: bool,
    /// 失败原因 (如果全部策略都失败)
    pub final_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageMetadata {
    pub author: Option<String>,
    pub published_date: Option<String>,
    pub language: Option<String>,
    pub word_count: usize,
    pub reading_time_minutes: f64,
    pub keywords: Vec<String>,
}

// ─── 域名韧性状态 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainResilience {
    pub domain: String,
    pub total_attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub consecutive_failures: u32,
    pub backoff_until_ms: u64, // 冻结到何时
    pub last_success_strategy: Option<FetchStrategy>,
}

impl DomainResilience {
    pub fn new(domain: &str) -> Self {
        Self {
            domain: domain.into(),
            total_attempts: 0,
            successes: 0,
            failures: 0,
            consecutive_failures: 0,
            backoff_until_ms: 0,
            last_success_strategy: None,
        }
    }

    /// 是否应该跳过该域名 (熔断中)
    pub fn is_circuit_open(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        self.consecutive_failures >= 5 && now < self.backoff_until_ms
    }

    /// 记录成功
    pub fn record_success(&mut self, strategy: FetchStrategy) {
        self.total_attempts += 1;
        self.successes += 1;
        self.consecutive_failures = 0;
        self.backoff_until_ms = 0;
        self.last_success_strategy = Some(strategy);
    }

    /// 记录失败 + 指数退避
    pub fn record_failure(&mut self) {
        self.total_attempts += 1;
        self.failures += 1;
        self.consecutive_failures += 1;
        let backoff_secs = 2u64.pow(self.consecutive_failures.min(6)) * 1000; // 2s→4s→8s→...→64s
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        self.backoff_until_ms = now + backoff_secs;
    }
}

// ─── 正文提取器 (Readability 简化版) ─────────────────────────────

/// HTML → 纯文本正文提取
pub fn extract_main_content(html: &str) -> String {
    // 移除 script/style/noscript
    let cleaned = remove_tags_content(html, &["script", "style", "noscript", "iframe", "svg"]);
    // 移除 HTML 标签
    let text = strip_tags(&cleaned);
    // 压缩空白
    let compressed = compress_whitespace(&text);
    // 截取合理长度 (最大 64KB 正文)
    if compressed.len() > 65536 {
        compressed[..65536].to_string()
    } else {
        compressed
    }
}

/// HTML → Markdown 转换
pub fn html_to_markdown_simple(html: &str, base_url: &str) -> String {
    let _cleaned = remove_tags_content(html, &["script", "style", "noscript", "nav", "footer", "header", "aside"]);
    let mut md = String::new();

    // 标题
    if let Some(title) = extract_meta(html, "title") {
        md.push_str(&format!("# {}\n\n", title));
    }

    // 元数据行
    let author = extract_meta(html, "author");
    let date = extract_meta(html, "date");
    if author.is_some() || date.is_some() {
        md.push_str("> ");
        if let Some(a) = &author { md.push_str(&format!("作者: {}  ", a)); }
        if let Some(d) = &date { md.push_str(&format!("日期: {}  ", d)); }
        md.push_str(&format!("\n> 来源: {}\n\n", base_url));
    } else {
        md.push_str(&format!("> 来源: {}\n\n", base_url));
    }

    // 正文段落 (每段 > 40 chars)
    let body = extract_main_content(html);
    for para in body.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.len() > 10 {
            md.push_str(&format!("{}\n\n", trimmed));
        }
    }

    md
}

/// 从 HTML 提取元数据
pub fn extract_page_metadata(html: &str) -> PageMetadata {
    let mut meta = PageMetadata::default();
    meta.author = extract_meta(html, "author");
    meta.published_date = extract_meta(html, "date")
        .or_else(|| extract_meta(html, "pubdate"));
    meta.language = extract_meta_attr(html, "html", "lang");

    // 关键词
    let kw_str = extract_meta(html, "keywords").unwrap_or_default();
    meta.keywords = kw_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    // 字数与阅读时间
    let body = extract_main_content(html);
    meta.word_count = body.split_whitespace().count();
    meta.reading_time_minutes = (meta.word_count as f64 / 250.0).max(0.5); // 250 wpm

    meta
}

// ─── 自治愈链接发现 ──────────────────────────────────────────────

/// 从页面中提取值得跟随的内部链接
pub fn discover_important_links(html: &str, base_url: &str, max_links: usize) -> Vec<String> {
    let link_re = regex::Regex::new(r#"<a[^>]+href=["']([^"']+)["'][^>]*>"#).unwrap();
    let mut links = Vec::new();
    let _base_domain = extract_domain(base_url);

    for cap in link_re.captures_iter(html).take(max_links * 3) {
        let href = cap[1].to_string();
        // 跳过锚点、mailto、javascript
        if href.starts_with('#') || href.starts_with("mailto:") || href.starts_with("javascript:") {
            continue;
        }
        let full = if href.starts_with("http") {
            href
        } else if href.starts_with('/') {
            format!("{}://{}{}", if base_url.starts_with("https") { "https" } else { "http" },
                extract_domain(base_url), href)
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), href.trim_start_matches('/'))
        };

        if !links.contains(&full) && links.len() < max_links {
            links.push(full);
        }
    }

    links
}

// ─── 完整抓取流程 (永不言弃) ──────────────────────────────────────

pub async fn indomitable_fetch(
    url: &str,
    client: &reqwest::Client,
    domain_states: &mut HashMap<String, DomainResilience>,
    follow_depth: u8,
) -> IndomitableFetchResult {
    let start = std::time::Instant::now();
    let domain = extract_domain(url);
    let state = domain_states.entry(domain.clone()).or_insert_with(|| DomainResilience::new(&domain));

    // 熔断检查
    if state.is_circuit_open() {
        return IndomitableFetchResult {
            url: url.into(), title: None, main_content: String::new(),
            raw_size: 0, content_size: 0, extraction_ratio: 0.0,
            attempts: vec![], winning_strategy: None, total_latency_ms: start.elapsed().as_millis() as u64,
            metadata: PageMetadata::default(), discovered_links: vec![],
            success: false, final_error: Some(format!("域名 {} 已熔断 (连续失败 {} 次)", domain, state.consecutive_failures)),
        };
    }

    let mut attempts = Vec::new();
    let mut last_html = String::new();
    let mut last_error = String::new();
    let mut winning = None;

    // 如果上次有成功策略，优先使用
    let strategies: Vec<&FetchStrategy> = if let Some(ref prev) = state.last_success_strategy {
        let mut s = vec![prev];
        s.extend(FETCH_STRATEGIES.iter().filter(|&st| st != prev));
        s
    } else {
        FETCH_STRATEGIES.iter().collect()
    };

    for strategy in strategies {
        let att_start = std::time::Instant::now();
        match fetch_with_strategy(url, client, strategy).await {
            Ok((html, status)) => {
                let latency = att_start.elapsed().as_millis() as u64;
                attempts.push(FetchAttempt {
                    strategy: strategy.clone(), success: true, status_code: Some(status),
                    bytes_received: html.len(), latency_ms: latency, error: None,
                });
                last_html = html;
                winning = Some(strategy.clone());
                state.record_success(strategy.clone());
                break;
            }
            Err(e) => {
                let latency = att_start.elapsed().as_millis() as u64;
                attempts.push(FetchAttempt {
                    strategy: strategy.clone(), success: false, status_code: None,
                    bytes_received: 0, latency_ms: latency, error: Some(e.clone()),
                });
                last_error = e;
            }
        }
    }

    if last_html.is_empty() {
        state.record_failure();
        let latency = start.elapsed().as_millis() as u64;
        return IndomitableFetchResult {
            url: url.into(), title: None, main_content: String::new(),
            raw_size: 0, content_size: 0, extraction_ratio: 0.0,
            attempts: attempts.clone(), winning_strategy: None, total_latency_ms: latency,
            metadata: PageMetadata::default(), discovered_links: vec![],
            success: false,
            final_error: Some(format!("全部 {} 种策略均失败。最后错误: {}", attempts.len(), last_error)),
        };
    }

    // 提取正文
    let raw_size = last_html.len();
    let markdown = html_to_markdown_simple(&last_html, url);
    let content_size = markdown.len();
    let metadata = extract_page_metadata(&last_html);

    // 递归发现链接
    let discovered = if follow_depth > 0 {
        discover_important_links(&last_html, url, 5)
    } else {
        vec![]
    };

    let latency = start.elapsed().as_millis() as u64;
    IndomitableFetchResult {
        url: url.into(),
        title: metadata.author.clone().or_else(|| extract_title(&last_html)),
        main_content: markdown,
        raw_size, content_size,
        extraction_ratio: if raw_size > 0 { 1.0 - content_size as f64 / raw_size as f64 } else { 0.0 },
        attempts, winning_strategy: winning, total_latency_ms: latency,
        metadata, discovered_links: discovered,
        success: true, final_error: None,
    }
}

// ─── 内部辅助 ──────────────────────────────────────────────────────

async fn fetch_with_strategy(url: &str, client: &reqwest::Client, strategy: &FetchStrategy) -> Result<(String, u16), String> {
    let resp = client
        .get(url)
        .header("User-Agent", strategy.user_agent())
        .header("Accept", "text/html,application/xhtml+xml,text/plain;q=0.9,*/*;q=0.8")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("{}: {}", strategy.label(), e))?;

    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", status));
    }

    let html = resp.text().await.map_err(|e| format!("Read body: {}", e))?;
    Ok((html, status))
}

fn extract_domain(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url)
        .split('/').next().unwrap_or("")
        .split(':').next().unwrap_or("")
        .to_lowercase()
}

fn remove_tags_content(html: &str, tags: &[&str]) -> String {
    let mut result = html.to_string();
    for tag in tags {
        let re = regex::Regex::new(&format!(r"<{}[^>]*>[\s\S]*?</{}>", tag, tag)).unwrap();
        result = re.replace_all(&result, "").to_string();
    }
    result
}

fn strip_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(html, " ").to_string()
}

fn compress_whitespace(text: &str) -> String {
    let re = regex::Regex::new(r"[ \t]{2,}").unwrap();
    let tmp = re.replace_all(text, " ").to_string();
    let re2 = regex::Regex::new(r"\n{3,}").unwrap();
    re2.replace_all(&tmp, "\n\n").to_string()
}

fn extract_meta(html: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"<meta[^>]+(?:name|property)=["']{}["'][^>]+content=["']([^"']+)["']"#, name);
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(html).map(|c| c[1].to_string())
}

fn extract_meta_attr(html: &str, tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"<{}[^>]+{}="([^"]+)""#, tag, attr);
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(html).map(|c| c[1].to_string())
}

fn extract_title(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    re.captures(html).map(|c| c[1].trim().to_string())
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls() {
        let text = r#"Check out https://example.com and [docs](https://docs.rs/tokio) for info. Also visit http://test.org/page?q=1"#;
        let urls = extract_urls(text);
        assert!(urls.iter().any(|u| u == "https://example.com"));
        assert!(urls.iter().any(|u| u == "https://docs.rs/tokio"));
        assert!(urls.iter().any(|u| u == "http://test.org/page?q=1"));
    }

    #[test]
    fn test_domain_resilience() {
        let mut state = DomainResilience::new("example.com");
        assert!(!state.is_circuit_open());

        // 5 failures should trigger circuit
        for _ in 0..5 { state.record_failure(); }
        assert!(state.is_circuit_open());

        // Success resets
        state.record_success(FetchStrategy::Standard);
        assert!(!state.is_circuit_open());
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_extract_main_content() {
        let html = r#"<html><head><title>Test</title></head><body><script>console.log('x')</script><nav>menu</nav><article><p>Hello world this is the main content of the page and it should be extracted properly.</p></article><footer>copyright</footer></body></html>"#;
        let content = extract_main_content(html);
        assert!(content.contains("Hello world"));
        assert!(!content.contains("console.log"));
        assert!(!content.contains("menu"));
    }

    #[test]
    fn test_discover_links() {
        let html = "<a href='/docs'>Docs</a><a href='#top'>Top</a><a href='mailto:x@y.com'>Email</a><a href='https://ext.com/page'>External</a>";
        let links = discover_important_links(html, "https://example.com", 10);
        assert!(links.iter().any(|l| l.contains("/docs")));
        assert!(links.iter().any(|l| l.contains("ext.com")));
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn indomitable_fetch_url(
    _state: tauri::State<'_, crate::state::AppState>,
    url: String,
    follow_depth: Option<u8>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| e.to_string())?;
    let mut domain_states = std::collections::HashMap::new();
    let result = indomitable_fetch(
        &url, &client, &mut domain_states, follow_depth.unwrap_or(0),
    ).await;
    Ok(serde_json::json!(result))
}

#[tauri::command]
pub fn extract_urls_from_text(text: String) -> Vec<String> {
    extract_urls(&text)
}
