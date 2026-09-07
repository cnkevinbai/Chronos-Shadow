// 实体识别与内容类型检测
use std::collections::HashMap;
use super::types::*;

pub(super) fn extract_entities_weighted(content: &str, references: &[(String, String)], aggressiveness: f64) -> Vec<ExtractedEntity> {
    let mut entities = extract_entities(content, references);

    // 激进实体提取 → 保留更多低置信度实体
    if aggressiveness < 0.4 {
        entities.retain(|e| e.occurrences > 1); // 保守：只保留多次出现的
    }

    // 去重并排序
    entities.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    entities.dedup_by(|a, b| a.name == b.name && a.entity_type == b.entity_type);

    let max = (entities.len() as f64 * aggressiveness).ceil() as usize;
    entities.truncate(max.max(3));

    entities
}

/// 检测内容类型
pub(super) fn detect_content_type(content: &str) -> String {
    let lower = content.to_lowercase();
    let code_indicators = ["```", "fn ", "def ", "class ", "import ", "const ", "let ", "var ", "function("];
    let doc_indicators = ["## ", "api reference", "parameters", "returns", "example", "usage", "install"];
    let blog_indicators = ["published", "author", "comment", "share", "subscribe", "read more"];

    let code_score = code_indicators.iter().filter(|i| lower.contains(*i)).count();
    let doc_score = doc_indicators.iter().filter(|i| lower.contains(*i)).count();
    let blog_score = blog_indicators.iter().filter(|i| lower.contains(*i)).count();

    if code_score > doc_score && code_score > blog_score { return "code".into(); }
    if doc_score > code_score && doc_score > blog_score { return "documentation".into(); }
    if blog_score > code_score && blog_score > doc_score { return "blog".into(); }
    if code_score > 0 && doc_score > 0 { return "mixed".into(); }
    "documentation".into()
}

/// 原始版本（向后兼容）
pub(super) fn extract_entities(content: &str, references: &[(String, String)]) -> Vec<ExtractedEntity> {
    let mut entities = Vec::new();
    let mut seen = HashMap::new();

    // 版本号提取: X.Y.Z 或 vX.Y.Z
    let version_re = regex_lite::Regex::new(r"\b(v?\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?)\b").unwrap();
    for cap in version_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let ver = m.as_str().to_string();
            let count = seen.entry(("version", ver.clone())).or_insert(0u32);
            if *count < 3 {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Version,
                    name: ver,
                    context: format!("Found in content near byte {}", m.start()),
                    occurrences: 1,
                });
            }
            *count += 1;
        }
    }

    // 日期提取: YYYY-MM-DD 或 Month DD, YYYY
    let date_re = regex_lite::Regex::new(r"\b(\d{4}-\d{2}-\d{2})\b").unwrap();
    for cap in date_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let date = m.as_str().to_string();
            let count = seen.entry(("date", date.clone())).or_insert(0u32);
            if *count < 2 {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Date,
                    name: date,
                    context: "Release/blog date".into(),
                    occurrences: 1,
                });
            }
            *count += 1;
        }
    }

    // Rust crate 提取: `crate_name` or "crate-name" in backticks
    let crate_re = regex_lite::Regex::new(r"`([a-z][a-z0-9_-]+)`").unwrap();
    let known_crates = ["tokio", "serde", "reqwest", "axum", "actix", "diesel", "sqlx",
        "clap", "tracing", "chrono", "async-trait", "thiserror", "anyhow", "tauri",
        "rocket", "warp", "hyper", "tonic", "prost"];
    for cap in crate_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let name = m.as_str().to_string();
            if known_crates.contains(&name.as_str()) {
                let count = seen.entry(("crate", name.clone())).or_insert(0u32);
                if *count < 2 {
                    entities.push(ExtractedEntity {
                        entity_type: EntityType::Crate,
                        name: name.clone(),
                        context: format!("Crate reference"),
                        occurrences: 1,
                    });
                }
                *count += 1;
            }
        }
    }

    // npm/pip package 提取
    let pkg_re = regex_lite::Regex::new(r"\b(npm install|pip install|cargo add|cargo install)\s+([^\s]+)").unwrap();
    for cap in pkg_re.captures_iter(content) {
        if let Some(m) = cap.get(2) {
            let name = m.as_str().to_string();
            entities.push(ExtractedEntity {
                entity_type: EntityType::Package,
                name,
                context: "Package install command".into(),
                occurrences: 1,
            });
        }
    }

    // GitHub 仓库提取: user/repo
    let repo_re = regex_lite::Regex::new(r"github\.com/([a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+)").unwrap();
    for cap in repo_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Repository,
                name: m.as_str().to_string(),
                context: "GitHub repository".into(),
                occurrences: 1,
            });
        }
    }

    // 废弃标记
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.contains("deprecated") || lower.contains("deprecation") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Deprecated,
                name: line.trim().chars().take(80).collect(),
                context: "Deprecation notice".into(),
                occurrences: 1,
            });
        }
        if lower.contains("breaking change") || lower.contains("breaking-change") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Breaking,
                name: line.trim().chars().take(80).collect(),
                context: "Breaking change notice".into(),
                occurrences: 1,
            });
        }
    }

    // 许可证提取
    for (text, _url) in references {
        let lower = text.to_lowercase();
        if lower.contains("license") || lower.contains("mit") || lower.contains("apache") || lower.contains("gpl") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::License,
                name: text.clone(),
                context: "License reference".into(),
                occurrences: 1,
            });
        }
    }

    // 去重
    entities.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    entities.dedup_by(|a, b| a.entity_type == b.entity_type && a.name == b.name);

    entities
}

// ─── 单元测试 ──────────────────────────────────────────────────────

