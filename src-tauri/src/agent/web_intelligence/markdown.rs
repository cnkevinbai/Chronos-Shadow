// HTML→Markdown 转换与文本处理辅助
// ─── HTML → Markdown 转换 ──────────────────────────────────────────

pub(super) fn html_to_markdown(html: &str, base_url: &str) -> String {
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

pub(super) fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let start_pattern = format!("<{}", tag);
    let end_pattern = format!("</{}>", tag);

    let lower = html.to_lowercase();
    let start = lower.find(&start_pattern)?;
    let start = lower[start..].find('>').map(|i| start + i + 1)?;
    let end = lower[start..].find(&end_pattern)?;

    Some(html[start..start + end].to_string())
}

pub(super) fn extract_meta_content(html: &str, name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let pattern = format!("<meta name=\"{}\" content=\"", name);
    let pos = lower.find(&pattern)?;
    let start = pos + pattern.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
}

pub(super) fn remove_tags(html: &str, tags: &[&str]) -> String {
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

pub(super) fn strip_html_tags(html: &str) -> String {
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

pub(super) fn extract_title(html: &str) -> Option<String> {
    extract_tag_content(html, "title")
}

// ─── 蒸馏 ─────────────────────────────────────────────────────────

#[allow(dead_code)]
pub(super) fn distill_content(content: &str, max_bytes: usize) -> String {
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

pub(super) fn extract_domain(url: &str) -> String {
    let url = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    url.split('/').next().unwrap_or(url).split(':').next().unwrap_or("").to_string()
}

pub(super) fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_chars).collect::<String>())
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────
