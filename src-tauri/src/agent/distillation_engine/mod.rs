// 蒸馏引擎 (Distillation Engine)
//
// 从原始网页/文档内容中提取结构化知识，多级蒸馏压缩，
// 确保喂给大模型的上下文既精炼又信息密度最大化。
//
// 设计理念：
//   1. 结构感知 — 识别标题层级、代码块、API签名、表格、列表等
//   2. 分级蒸馏 — Light (保留结构) / Medium (语义提取) / Deep (知识压缩)
//   3. Token预算精确 — 按目标 token 数裁剪，而非简单字节截断
//   4. 来源锚定 — 每条提取信息标注原文位置，防幻觉追溯
//   5. 缓存加速 — 相同 URL 不重复蒸馏

mod types;
mod engine;
mod helpers;
mod entities;

pub use types::*;
pub use helpers::{estimate_tokens, budget_for_model};

#[cfg(test)]
mod tests {
    use super::*;
    use super::helpers::*;

    fn sample_markdown() -> String {
        r#"# Rust Async Traits

> Important: async fn in traits was stabilized in Rust 1.75.0

## Overview

Rust 1.75.0 stabilized `async fn` in trait definitions. This is a **breaking change**
for libraries using the `#[async_trait]` macro from the async-trait crate.

## API Reference

```rust
pub trait Service {
    async fn handle(&self, req: Request) -> Response;
    async fn health_check(&self) -> bool;
}
```

The `handle` method must return a `Response`. The `health_check` is optional.

### Requirements
- **Minimum Rust version**: 1.75.0
- Edition 2021 or later

### Migration Guide

If you were using `#[async_trait]`, remove the macro and add `async` directly:

```rust
// Before
#[async_trait]
pub trait OldWay {
    async fn process(&self) -> Result<()>;
}

// After (Rust 1.75+)
pub trait NewWay {
    async fn process(&self) -> Result<()>;
}
```

## Links
- [Rust Blog: async fn in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit.html)
- [Stabilization PR](https://github.com/rust-lang/rust/pull/12345)

Note that `Send` bounds are not automatically inferred. You may need explicit `Send` bounds.

© 2024 Rust Team. All rights reserved.
"#.to_string()
    }

    #[test]
    fn test_estimate_tokens_ascii_vs_cjk() {
        // 纯 ASCII：100 字符 ≈ 25 token（4 字符/token）
        assert_eq!(estimate_tokens(&"a".repeat(100)), 25);
        // 纯中文：100 字 ≈ 60 token（旧 len/4 是 300 字节/4 = 75，高估 25%）
        assert_eq!(estimate_tokens(&"中".repeat(100)), 60);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_budget_for_model_levels() {
        let light = budget_for_model("deepseek-v4-pro", DistillationLevel::Light);
        let medium = budget_for_model("deepseek-v4-pro", DistillationLevel::Medium);
        let deep = budget_for_model("deepseek-v4-pro", DistillationLevel::Deep);
        assert_eq!(light, 16000);   // 120000/4 = 30000 → clamp 16000
        assert_eq!(medium, 15000);  // 120000/8
        assert_eq!(deep, 7500);     // 120000/16
        // 小上下文模型（ollama 8K）→ clamp 下限 1000
        assert_eq!(budget_for_model("ollama-local", DistillationLevel::Deep), 1000);
    }

    #[test]
    fn test_light_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Light, Some(2000));

        assert!(result.distilled_size < result.original_size);
        assert!(result.markdown.contains("Rust Async Traits"));
        assert!(result.markdown.contains("```"));
        assert!(!result.markdown.contains("©")); // noise filtered
    }

    #[test]
    fn test_medium_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Medium, Some(1000));

        assert!(result.markdown.contains("API"));
        assert!(!result.key_insights.is_empty());
        assert!(result.compression_ratio > 0.5);
    }

    #[test]
    fn test_deep_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Deep, Some(500));

        assert!(result.markdown.contains("TL;DR"));
        assert!(result.markdown.contains("Core APIs"));
        assert!(result.compression_ratio > 0.8);
        assert!(result.estimated_tokens < 500);
    }

    #[test]
    fn test_cache_hit() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let url = "https://docs.rs/cached-test";

        let r1 = engine.distill(&content, url, DistillationLevel::Deep, Some(500));
        let r2 = engine.distill(&content, url, DistillationLevel::Deep, Some(500));

        // Should be identical (cache hit)
        assert_eq!(r1.markdown, r2.markdown);
        assert_eq!(engine.total_distillations, 2);
    }

    #[test]
    fn test_api_signature_detection() {
        assert!(is_api_signature("pub async fn handle(&self, req: Request) -> Response"));
        assert!(is_api_signature("fn process(data: &[u8]) -> Result<()>"));
        assert!(is_api_signature("def process(self, data: bytes) -> None:"));
        assert!(!is_api_signature("This is just a sentence about functions."));
    }

    #[test]
    fn test_heading_parsing() {
        let (level, text) = parse_heading("## API Reference");
        assert_eq!(level, 2);
        assert_eq!(text, "API Reference");

        let (level, text) = parse_heading("# Top Level");
        assert_eq!(level, 1);
        assert_eq!(text, "Top Level");
    }

    #[test]
    fn test_link_extraction() {
        let links = extract_inline_links("See the [Rust Blog](https://blog.rust-lang.org) and [docs](https://docs.rs)");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1, "https://blog.rust-lang.org");
    }

    #[test]
    fn test_noise_filtering() {
        assert!(is_noise("© 2024 All rights reserved"));
        assert!(is_noise("Sign up for our newsletter"));
        assert!(!is_noise("Rust 1.75 stabilized async fn in traits"));
    }

    #[test]
    fn test_fragment_extraction() {
        let engine = DistillationEngine::new();
        let content = sample_markdown();
        let fragments = engine.extract_fragments(&content, DistillationLevel::Medium);

        let headings: Vec<_> = fragments.iter().filter(|f| matches!(f, ContentFragment::Heading { .. })).collect();
        let code_blocks: Vec<_> = fragments.iter().filter(|f| matches!(f, ContentFragment::CodeBlock { .. })).collect();

        assert!(headings.len() >= 2);
        assert!(code_blocks.len() >= 1);
    }
}
