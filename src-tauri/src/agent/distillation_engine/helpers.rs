// 渲染 / Markdown 解析 / Token 估算辅助函数
use super::types::*;

pub(super) fn render_fragment_light(frag: &ContentFragment) -> String {
    match frag {
        ContentFragment::Heading { level, text } => format!("{} {}\n", "#".repeat(*level as usize), text),
        ContentFragment::CodeBlock { language, code, .. } => {
            format!("```{}\n{}\n```\n", language, code)
        }
        ContentFragment::ApiSignature { signature, context } => {
            format!("`{}` — {}\n", signature, context)
        }
        ContentFragment::Table { headers, rows } => {
            let mut t = String::new();
            t.push_str(&format!("| {} |\n", headers.join(" | ")));
            t.push_str(&format!("| {} |\n", headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
            for row in rows { t.push_str(&format!("| {} |\n", row.join(" | "))); }
            t
        }
        ContentFragment::ListItem { text, depth } => {
            format!("{}- {}\n", "  ".repeat(*depth as usize), text)
        }
        ContentFragment::Link { text, url } => format!("[{}]({})\n", text, url),
        ContentFragment::Definition { term, definition } => format!("**{}**: {}\n", term, definition),
        ContentFragment::Paragraph { text, .. } => format!("{}\n", text),
        ContentFragment::KeyFact { statement, .. } => format!("> {}\n", statement),
        ContentFragment::RawText { text } => format!("{}\n", text),
    }
}

pub(super) fn render_fragment_medium(frag: &ContentFragment) -> String {
    match frag {
        ContentFragment::Heading { level, text } => {
            format!("{} {}\n", "#".repeat((*level + 1).min(6) as usize), text)
        }
        ContentFragment::CodeBlock { language, code, .. } => {
            let truncated = truncate_lines(code, 30);
            format!("```{}\n{}\n```\n", language, truncated)
        }
        ContentFragment::ApiSignature { signature, context } => {
            format!("- `{}` — *{}*\n", signature, truncate_words(context, 15))
        }
        ContentFragment::KeyFact { statement, .. } => format!("> {}\n", statement),
        ContentFragment::Definition { term, definition } => {
            format!("- **{}**: {}\n", term, truncate_words(definition, 50))
        }
        ContentFragment::Paragraph { text, .. } => format!("{}\n", truncate_words(text, 100)),
        ContentFragment::Table { headers, rows } => {
            format!("| {} |\n(Table: {} rows)\n", headers.join(" | "), rows.len())
        }
        _ => String::new(),
    }
}

// ─── 解析辅助函数 ──────────────────────────────────────────────────

pub(super) fn parse_heading(line: &str) -> (u8, String) {
    let trimmed = line.trim();
    let level = trimmed.chars().take_while(|c| *c == '#').count().min(6) as u8;
    let text = trimmed[level as usize..].trim().to_string();
    (level, text)
}

pub(super) fn parse_list_item(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim();
    let depth = (line.len() - trimmed.len()) as u8 / 2;
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        Some((depth, trimmed[2..].to_string()))
    } else if let Some(pos) = trimmed.find(". ") {
        let num_part = &trimmed[..pos];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            Some((depth, trimmed[pos + 2..].to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

pub(super) fn parse_definition(line: &str) -> Option<(String, String)> {
    // Pattern: **Term**: definition
    let line = line.trim();
    if line.starts_with("**") {
        if let Some(end_bold) = line[2..].find("**") {
            let term = line[2..end_bold + 2].to_string();
            let rest = line[end_bold + 4..].trim();
            if rest.starts_with(':') {
                return Some((term, rest[1..].trim().to_string()));
            }
        }
    }
    None
}

pub(super) fn try_parse_table<'a>(lines: &[&'a str], i: &mut usize) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let header_line = lines[*i];
    let headers: Vec<String> = header_line
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() { return None; }

    // 检查下一行是否是分隔线
    if *i + 1 >= lines.len() { return None; }
    let sep_line = lines[*i + 1];
    if !sep_line.contains("---") { return None; }

    *i += 1; // skip separator

    let mut rows = Vec::new();
    while *i + 1 < lines.len() {
        *i += 1;
        let row_line = lines[*i];
        if !row_line.trim().starts_with('|') { break; }
        let cells: Vec<String> = row_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();
        if !cells.is_empty() { rows.push(cells); }
    }

    Some((headers, rows))
}

pub(super) fn extract_inline_links(line: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        // 找 `[text](url)` 模式
        if chars[pos] == '[' {
            if let Some(close_bracket) = chars[pos..].iter().position(|c| *c == ']') {
                let text: String = chars[pos + 1..pos + close_bracket].iter().collect();
                let after = pos + close_bracket + 1;
                if after + 1 < len && chars[after] == '(' {
                    if let Some(close_paren) = chars[after..].iter().position(|c| *c == ')') {
                        let url: String = chars[after + 1..after + close_paren].iter().collect();
                        if url.starts_with("http") && !text.is_empty() {
                            links.push((text, url));
                        }
                        pos = after + close_paren + 1;
                        continue;
                    }
                }
            }
        }
        pos += 1;
    }
    links
}

pub(super) fn is_api_signature(line: &str) -> bool {
    let line = line.trim();
    // fn name(args) -> ReturnType
    if line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("async fn ") {
        return line.contains('(') && line.contains(')');
    }
    // def name(args):
    if line.starts_with("def ") && line.contains('(') && line.contains(')') && line.ends_with(':') {
        return true;
    }
    // function name(args) or const name = (args) =>
    if (line.starts_with("function ") || line.starts_with("const "))
        && line.contains('(') && line.contains(')')
        && (line.contains("=>") || line.contains(':') || line.contains('{')) {
        return true;
    }
    false
}

pub(super) fn is_key_fact(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    let indicators = [
        "important", "note that", "note:", "warning", "caution",
        "关键", "重要", "注意", "必须", "deprecated", "breaking change",
        "since version", "requires", "minimum", "maximum",
    ];
    indicators.iter().any(|i| line.contains(i))
}

pub(super) fn fact_confidence(line: &str) -> f64 {
    let line = line.trim().to_lowercase();
    if line.contains("must") || line.contains("必须") { return 0.95; }
    if line.contains("should") || line.contains("应该") { return 0.8; }
    if line.contains("may") || line.contains("可能") { return 0.5; }
    0.7
}

pub(super) fn paragraph_importance(line: &str) -> f64 {
    let line = line.trim().to_lowercase();
    let mut score = 0.3f64; // base

    if line.chars().count() > 200 { score += 0.2; }

    // 关键词密度加权（重复出现比单次更重要，最多计 3 次避免长文本刷分）
    let keywords: &[(&str, f64)] = &[
        ("example", 0.06), ("示例", 0.06),
        ("api", 0.08), ("function", 0.08), ("method", 0.08),
        ("deprecated", 0.1), ("breaking", 0.1),
        ("version", 0.05), ("usage", 0.05), ("install", 0.05), ("config", 0.05),
        ("`", 0.03),
    ];
    for (kw, w) in keywords {
        score += w * (line.matches(kw).count() as f64).min(3.0);
    }
    if line.starts_with("> ") { score += 0.05; }

    score.min(1.0)
}

pub(super) fn is_noise(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    line.contains("cookie") || line.contains("advertisement") || line.contains("subscribe")
        || line.contains("sign up") || line.contains("log in") || line.contains("©")
        || line.contains("all rights reserved") || line.starts_with("<!--")
        || line.is_empty()
}

// ─── 工具函数 ──────────────────────────────────────────────────────

/// 模型感知 token 估算：ASCII ≈ 4 字符/token，CJK/非ASCII ≈ 1.5 字符/token。
/// 比固定 len/4 更精确——中文是 UTF-8 3 字节/字，len/4 会把中文高估约 25%，
/// 导致过度压缩。兼容 DeepSeek/Kimi/GLM/Ollama 的子词分词器。
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0.0f64;
    for c in text.chars() {
        tokens += if c.is_ascii() { 0.25 } else { 0.6 };
    }
    tokens.ceil() as usize
}

/// 根据模型上下文窗口返回合理的蒸馏 token 预算。
/// Light 保留更多（ctx/4），Deep 压得更狠（ctx/16），clamp 到 [1000, 16000] 防溢出。
pub fn budget_for_model(model: &str, level: DistillationLevel) -> usize {
    let ctx = crate::agent::api_client::get_context_window(model);
    let base = match level {
        DistillationLevel::Light => ctx / 4,
        DistillationLevel::Medium => ctx / 8,
        DistillationLevel::Deep => ctx / 16,
    };
    base.clamp(1000, 16000)
}

pub(super) fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words { return text.to_string(); }
    format!("{}...", words[..max_words].join(" "))
}

pub(super) fn truncate_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines { return text.to_string(); }
    format!("{}\n... ({} more lines)", lines[..max_lines].join("\n"), lines.len() - max_lines)
}

// ─── 实体提取 ──────────────────────────────────────────────────────

