// 特种专业子智能体网络 (Explore, Scout, Compaction 上下文压缩体)
//
// 主智能体在运行中自动激活或通过 @ 触发后台轻量级专用特种兵：
// - @Explore: 只读检索源码结构 → 返回文件拓扑 + 符号表（真实遍历文件系统 + 正则符号提取）
// - @Scout: 抓取远程技术文档 → 返回结构化摘要（真实 HTTP GET + HTML→文本）
// - @Compaction: 上下文压缩体 → 蒸馏工具调用日志为结构化摘要

use serde::{Deserialize, Serialize};
use std::path::Path;

// ─── 类型定义 ──────────────────────────────────────────────────────

/// 子智能体类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentType {
    /// 只读源码检索
    Explore,
    /// 远程文档抓取
    Scout,
    /// 上下文压缩蒸馏
    Compaction,
}

impl SubagentType {
    pub fn trigger(&self) -> &str {
        match self {
            SubagentType::Explore => "@Explore",
            SubagentType::Scout => "@Scout",
            SubagentType::Compaction => "@Compaction",
        }
    }
}

/// 子智能体执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    /// 子智能体类型
    pub agent_type: SubagentType,
    /// 是否成功
    pub success: bool,
    /// 结构化摘要
    pub summary: String,
    /// 附加数据
    pub data: Option<serde_json::Value>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

/// 文件节点（用于 Explore 的拓扑输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub children: Option<Vec<FileNode>>,
    pub symbols: Vec<String>, // 函数/类/接口名
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    Directory,
    File,
    Config,
    Test,
}

/// 上下文压缩统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionStats {
    /// 原始 Token 数
    pub original_tokens: usize,
    /// 压缩后 Token 数
    pub compressed_tokens: usize,
    /// 压缩比
    pub ratio: f64,
    /// 压缩耗时（毫秒）
    pub duration_ms: u64,
}

// ─── 子智能体管理器 ────────────────────────────────────────────────

/// 子智能体池 — 主智能体的"特种兵"调度中心
pub struct SubagentPool {
    /// Explore 执行次数
    pub explore_count: u32,
    /// Scout 执行次数
    pub scout_count: u32,
    /// Compaction 执行次数
    pub compaction_count: u32,
    /// 累计节省 Token 数
    pub tokens_saved: usize,
}

impl SubagentPool {
    pub fn new() -> Self {
        Self {
            explore_count: 0,
            scout_count: 0,
            compaction_count: 0,
            tokens_saved: 0,
        }
    }

    /// 激活 @Explore — 只读遍历项目源码，生成文件拓扑 + 符号表
    ///
    /// `root` 为项目根目录；`query` 用于过滤文件名/路径/符号（空则返回全部）。
    /// 跳过 node_modules / target / .git 等非源码目录，限制深度与条目数避免爆炸。
    pub async fn explore(&mut self, root: &Path, query: &str) -> SubagentResult {
        let start = std::time::Instant::now();
        self.explore_count += 1;

        tracing::info!("@Explore #{} activated for: {} (root {:?})", self.explore_count, query, root);

        let tree = build_tree(root, 0, query);
        let symbol_count = count_symbols(&tree);

        SubagentResult {
            agent_type: SubagentType::Explore,
            success: true,
            summary: format!(
                "Explored {:?} — {} entries, {} symbols matching '{}'",
                root, tree.len(), symbol_count, query
            ),
            data: Some(serde_json::to_value(&tree).unwrap_or_default()),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 激活 @Scout — 抓取远程技术文档，转换为 Markdown 结构化摘要
    pub async fn scout(&mut self, url: &str) -> SubagentResult {
        let start = std::time::Instant::now();
        self.scout_count += 1;

        tracing::info!("@Scout #{} activated for: {}", self.scout_count, url);

        let (success, summary) = match fetch_and_extract(url).await {
            Ok(s) => (true, s),
            Err(e) => (false, format!("Scout failed for {}: {}", url, e)),
        };

        SubagentResult {
            agent_type: SubagentType::Scout,
            success,
            summary,
            data: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 激活 @Compaction — 上下文压缩蒸馏
    pub async fn compact(&mut self, context: &str) -> SubagentResult {
        let _start = std::time::Instant::now();
        self.compaction_count += 1;

        tracing::info!(
            "@Compaction #{} activated: {} chars input",
            self.compaction_count,
            context.len()
        );

        let stats = self.compact_context(context);
        self.tokens_saved += stats.original_tokens.saturating_sub(stats.compressed_tokens);

        SubagentResult {
            agent_type: SubagentType::Compaction,
            success: true,
            summary: format!(
                "[Compacted] {}→{} tokens ({}% reduced, {}ms)",
                stats.original_tokens,
                stats.compressed_tokens,
                ((1.0 - stats.ratio) * 100.0) as u32,
                stats.duration_ms
            ),
            data: Some(serde_json::to_value(&stats).unwrap_or_default()),
            duration_ms: stats.duration_ms,
        }
    }

    /// 压缩上下文（工具调用日志 → 结构化摘要）
    fn compact_context(&self, context: &str) -> CompactionStats {
        let original_tokens = estimate_tokens(context);
        let lines: Vec<&str> = context.lines().collect();
        let mut compressed = String::new();
        let mut skipped = 0;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.contains("Calling")
                || trimmed.contains("error")
                || trimmed.contains("Error")
                || trimmed.contains("warning")
                || trimmed.contains("Warning")
                || trimmed.contains("failed")
                || trimmed.contains("Fused")
                || trimmed.contains("熔断")
            {
                compressed.push_str(line);
                compressed.push('\n');
            } else {
                skipped += 1;
            }
        }

        if skipped > 0 {
            compressed.push_str(&format!(
                "\n[{} redundant lines omitted by @Compaction]\n",
                skipped
            ));
        }

        let compressed_tokens = estimate_tokens(&compressed);
        CompactionStats {
            original_tokens,
            compressed_tokens,
            ratio: if original_tokens > 0 {
                compressed_tokens as f64 / original_tokens as f64
            } else {
                1.0
            },
            duration_ms: 0,
        }
    }

    /// 统计信息
    pub fn stats(&self) -> SubagentStats {
        SubagentStats {
            explore_count: self.explore_count,
            scout_count: self.scout_count,
            compaction_count: self.compaction_count,
            tokens_saved: self.tokens_saved,
        }
    }
}

/// 子智能体统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStats {
    pub explore_count: u32,
    pub scout_count: u32,
    pub compaction_count: u32,
    pub tokens_saved: usize,
}

impl Default for SubagentPool {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 工具函数 ──────────────────────────────────────────────────────

/// 粗略估算 Token 数（英文 ~4 字符/Token，中文 ~1.5 字符/Token）
fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            tokens += 1;
        } else {
            tokens += 2;
        }
    }
    tokens / 4
}

/// 递归构建文件拓扑树（跳过非源码目录，限制深度与条目数）
fn build_tree(root: &Path, depth: usize, query: &str) -> Vec<FileNode> {
    const SKIP_DIRS: &[&str] = &[
        "node_modules", "target", ".git", "dist", "dist-ssr", "build", "out",
        ".venv", "venv", "__pycache__", ".next", ".cache", "coverage", ".turbo",
    ];
    const MAX_DEPTH: usize = 5;
    const MAX_ENTRIES: usize = 300;

    let mut nodes = Vec::new();
    if depth >= MAX_DEPTH {
        return nodes;
    }

    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return nodes,
    };

    let q = query.to_lowercase();
    for entry in entries.flatten().take(MAX_ENTRIES) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            let children = build_tree(&path, depth + 1, query);
            let name_matches = q.is_empty() || name.to_lowercase().contains(&q);
            if !children.is_empty() || name_matches {
                nodes.push(FileNode {
                    name,
                    path: path.to_string_lossy().to_string(),
                    kind: FileKind::Directory,
                    children: Some(children),
                    symbols: vec![],
                });
            }
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let kind = if matches!(ext, "json" | "toml" | "yaml" | "yml" | "lock") {
                FileKind::Config
            } else if name.ends_with(".test.ts") || name.ends_with(".test.tsx") || name.ends_with(".spec.ts") {
                FileKind::Test
            } else {
                FileKind::File
            };

            let symbols = if is_source(ext) {
                std::fs::read_to_string(&path)
                    .map(|c| extract_symbols(ext, &c))
                    .unwrap_or_default()
            } else {
                vec![]
            };

            let matches = q.is_empty()
                || name.to_lowercase().contains(&q)
                || path.to_string_lossy().to_lowercase().contains(&q)
                || symbols.iter().any(|s| s.to_lowercase().contains(&q));

            if matches {
                nodes.push(FileNode {
                    name,
                    path: path.to_string_lossy().to_string(),
                    kind,
                    children: None,
                    symbols,
                });
            }
        }
    }

    nodes
}

/// 是否为可提取符号的源码文件
fn is_source(ext: &str) -> bool {
    matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "py" | "java" | "c" | "cpp" | "h" | "cs" | "rb" | "php")
}

/// 用正则提取源码中的函数/类/接口/类型名（tree-sitter 的轻量替代）
fn extract_symbols(ext: &str, content: &str) -> Vec<String> {
    let patterns: &[&str] = match ext {
        "rs" => &[
            r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)",
            r"(?m)^\s*(?:pub\s+)?(?:struct|enum|trait|impl)\s+(\w+)",
        ],
        "ts" | "tsx" | "js" | "jsx" => &[
            r"(?m)^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)",
            r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+(\w+)",
            r"(?m)^\s*(?:export\s+)?(?:class|interface|type)\s+(\w+)",
        ],
        "go" => &[
            r"(?m)^\s*func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)",
            r"(?m)^\s*type\s+(\w+)\s+struct",
        ],
        "py" => &[
            r"(?m)^\s*(?:async\s+)?def\s+(\w+)",
            r"(?m)^\s*class\s+(\w+)",
        ],
        "java" => &[
            r"(?m)^\s*(?:public|private|protected)?\s*(?:static\s+)?[\w<>\[\]]+\s+(\w+)\s*\(",
            r"(?m)^\s*(?:public\s+)?(?:class|interface|enum)\s+(\w+)",
        ],
        _ => &[],
    };

    let mut symbols: Vec<String> = Vec::new();
    for pat in patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            for cap in re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let s = m.as_str().to_string();
                    if !symbols.contains(&s) {
                        symbols.push(s);
                    }
                }
            }
        }
    }
    symbols
}

/// 递归统计树中符号总数
fn count_symbols(nodes: &[FileNode]) -> usize {
    nodes
        .iter()
        .map(|n| n.symbols.len() + n.children.as_ref().map(|c| count_symbols(c)).unwrap_or(0))
        .sum()
}

/// HTTP GET → HTML → Markdown 结构化摘要
async fn fetch_and_extract(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Chronos-Shadow-Scout/0.3.0")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(html_to_summary(&html, url))
}

/// HTML → 结构化 Markdown 摘要（标题 + 目录 + 正文摘要）
fn html_to_summary(html: &str, url: &str) -> String {
    let title = extract_tag(html, "title").unwrap_or_else(|| url.to_string());

    let mut headings: Vec<String> = Vec::new();
    for level in 1..=3 {
        let re = regex::Regex::new(&format!(r"(?is)<h{level}[^>]*>(.*?)</h{level}>")).unwrap();
        for cap in re.captures_iter(html) {
            let text = strip_tags(cap.get(1).map(|m| m.as_str()).unwrap_or("")).trim().to_string();
            if !text.is_empty() && text.len() < 120 {
                headings.push(format!("{} {}", "#".repeat(level), text));
            }
        }
    }

    let body = strip_tags(html);
    let excerpt: String = body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(800).collect();

    let mut summary = format!("# {}\n\n> {}\n\n", title, url);
    if !headings.is_empty() {
        summary.push_str("## 目录\n");
        for h in headings.iter().take(20) {
            summary.push_str(h);
            summary.push('\n');
        }
        summary.push('\n');
    }
    summary.push_str("## 摘要\n");
    summary.push_str(&excerpt);
    summary
}

/// 剥离 HTML 标签 + 解码常见实体
fn strip_tags(html: &str) -> String {
    let no_block = regex::Regex::new(r"(?is)<(script|style|noscript)[^>]*>.*?</\1>")
        .unwrap()
        .replace_all(html, " ");
    let no_tags = regex::Regex::new(r"<[^>]+>").unwrap().replace_all(&no_block, " ");
    no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// 提取单个 HTML 标签的文本内容
fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>")).unwrap();
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| strip_tags(m.as_str()).trim().to_string())
        .filter(|s| !s.is_empty())
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_root() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("chronos_subagent_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn hello() {}\npub struct Foo {}\nimpl Foo { pub fn bar(&self) {} }\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/main.ts"), "export function run() {}\nexport class App {}\n").unwrap();
        std::fs::write(dir.join("package.json"), "{}\n").unwrap();
        dir
    }

    #[test]
    fn test_explore_real_traversal() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let root = sample_root();
        let result = rt.block_on(pool.explore(&root, ""));
        assert!(result.success);
        assert_eq!(pool.explore_count, 1);
        // 真实遍历应找到 src 目录 + 文件
        let data = result.data.unwrap();
        let tree: Vec<FileNode> = serde_json::from_value(data).unwrap();
        assert!(!tree.is_empty(), "应遍历出文件树");
        assert!(count_symbols(&tree) > 0, "应从源码提取出符号");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_explore_query_filter() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let root = sample_root();
        // 按符号名查询
        let result = rt.block_on(pool.explore(&root, "hello"));
        let data = result.data.unwrap();
        let tree: Vec<FileNode> = serde_json::from_value(data).unwrap();
        assert!(count_symbols(&tree) > 0, "应匹配到 hello 符号");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_scout() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(pool.scout("https://docs.rs/tauri"));
        assert_eq!(pool.scout_count, 1);
        // 网络可能不可用：只要执行了（成功或失败都记录）就验证摘要非空
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_compaction() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let context = r#"
Calling tool: FileRead
  Result: OK (file contents)
Calling tool: FileEdit
  Result: OK (file modified)
Calling tool: Terminal
  Result: OK (command executed)
Some verbose output here that is not important
More verbose output
    error: compilation failed
Calling tool: FileEdit
  Result: OK
"#;

        let result = rt.block_on(pool.compact(context));
        assert!(result.success);
        assert_eq!(pool.compaction_count, 1);
        assert!(result.summary.contains("Compacted"));
        assert!(pool.tokens_saved > 0);
    }

    #[test]
    fn test_stats() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(pool.explore(Path::new("."), "test"));
        rt.block_on(pool.scout("test"));

        let stats = pool.stats();
        assert_eq!(stats.explore_count, 1);
        assert_eq!(stats.scout_count, 1);
        assert_eq!(stats.compaction_count, 0);
    }
}
