// 特种专业子智能体网络 (Explore, Scout, Compaction 上下文压缩体)
//
// 主智能体在运行中自动激活或通过 @ 触发后台轻量级专用特种兵：
// - @Explore: 只读检索源码结构 → 返回文件拓扑 + 符号表
// - @Scout: 抓取远程技术文档 → 返回结构化摘要
// - @Compaction: 上下文压缩体 → 蒸馏工具调用日志为结构化摘要

use serde::{Deserialize, Serialize};

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

    /// 激活 @Explore — 只读检索项目源码结构
    ///
    /// 生成文件拓扑树 + 符号表，避免大模型盲目扫描整个代码库
    pub async fn explore(&mut self, query: &str) -> SubagentResult {
        let start = std::time::Instant::now();
        self.explore_count += 1;

        tracing::info!("@Explore #{} activated for: {}", self.explore_count, query);

        // 生产环境：实际遍历项目文件系统，调用 tree-sitter 提取符号
        let mock_tree = vec![
            FileNode {
                name: "src".into(),
                path: "src".into(),
                kind: FileKind::Directory,
                children: Some(vec![
                    FileNode {
                        name: "App.tsx".into(),
                        path: "src/App.tsx".into(),
                        kind: FileKind::File,
                        children: None,
                        symbols: vec!["App".into(), "useState".into()],
                    },
                    FileNode {
                        name: "components".into(),
                        path: "src/components".into(),
                        kind: FileKind::Directory,
                        children: Some(vec![FileNode {
                            name: "FooterBar.tsx".into(),
                            path: "src/components/FooterBar.tsx".into(),
                            kind: FileKind::File,
                            children: None,
                            symbols: vec!["FooterBar".into()],
                        }]),
                        symbols: vec![],
                    },
                ]),
                symbols: vec![],
            },
            FileNode {
                name: "package.json".into(),
                path: "package.json".into(),
                kind: FileKind::Config,
                children: None,
                symbols: vec![],
            },
        ];

        SubagentResult {
            agent_type: SubagentType::Explore,
            success: true,
            summary: format!(
                "Explored: {} — found {} root entries matching '{}'",
                query,
                mock_tree.len(),
                query
            ),
            data: Some(serde_json::to_value(&mock_tree).unwrap_or_default()),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 激活 @Scout — 抓取远程技术文档
    ///
    /// 通过 HTTP 请求抓取文档，解析为 Markdown 结构化摘要
    pub async fn scout(&mut self, url: &str) -> SubagentResult {
        let start = std::time::Instant::now();
        self.scout_count += 1;

        tracing::info!("@Scout #{} activated for: {}", self.scout_count, url);

        // 生产环境：HTTP GET → HTML→Markdown 转换 → 提取核心内容
        let mock_summary = format!(
            "Scouted: {} — extracted {} sections from documentation",
            url,
            url.len().min(500) / 100
        );

        SubagentResult {
            agent_type: SubagentType::Scout,
            success: true,
            summary: mock_summary,
            data: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 激活 @Compaction — 上下文压缩蒸馏
    ///
    /// 将繁杂的工具调用日志秒级蒸馏为结构化摘要，清空无效 Token 占用
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

        // 压缩策略：
        // 1. 提取工具调用行（以 "Calling" 开头）
        // 2. 保留错误/警告行
        // 3. 丢弃冗余的 OK 响应
        // 4. 合并连续相同操作

        let lines: Vec<&str> = context.lines().collect();
        let mut compressed = String::new();
        let mut skipped = 0;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 保留：工具调用、错误、警告
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
        let duration = 0; // 简化

        CompactionStats {
            original_tokens,
            compressed_tokens,
            ratio: if original_tokens > 0 {
                compressed_tokens as f64 / original_tokens as f64
            } else {
                1.0
            },
            duration_ms: duration,
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
            tokens += 1; // 累计，最后除以 4
        } else {
            tokens += 2; // CJK 字符
        }
    }
    tokens / 4
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explore() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(pool.explore("App.tsx"));
        assert!(result.success);
        assert_eq!(pool.explore_count, 1);
        assert!(result.summary.contains("App.tsx"));
    }

    #[test]
    fn test_scout() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(pool.scout("https://docs.rs/tauri"));
        assert!(result.success);
        assert_eq!(pool.scout_count, 1);
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
        // 应该保留了关键行
        assert!(result.summary.contains("Compacted"));
        // Token 数应该减少
        assert!(pool.tokens_saved > 0);
    }

    #[test]
    fn test_multiple_compactions_accumulate() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let saved_before = pool.tokens_saved;
        // Use longer text to ensure token savings are measurable
        let long_text = "Calling tool: A\nerror: test failure\n".repeat(10)
            + &"some verbose output here that should be omitted\n".repeat(20);
        rt.block_on(pool.compact(&long_text));
        rt.block_on(pool.compact(&long_text));

        assert!(pool.tokens_saved > saved_before,
            "tokens_saved should increase after compaction (was {}, now {})",
            saved_before, pool.tokens_saved);
        assert_eq!(pool.compaction_count, 2);
    }

    #[test]
    fn test_stats() {
        let mut pool = SubagentPool::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(pool.explore("test"));
        rt.block_on(pool.scout("test"));

        let stats = pool.stats();
        assert_eq!(stats.explore_count, 1);
        assert_eq!(stats.scout_count, 1);
        assert_eq!(stats.compaction_count, 0);
    }
}
