// 统一行动调度引擎 (Action Dispatch)
// 从 LLM 响应中提取 JSON 动作块并执行：Web 搜索/抓取、文件读写、终端、
// MCP 调用、环境检测、PPT 生成等。红线一 + 安全边界双校验。

use crate::state::AppState;
use crate::agent::redline::AgentAction;
use crate::agent::web_intelligence::{WebSearchResult, WebFetchResult};

/// 从 LLM 响应文本中提取所有 JSON 动作块
fn extract_action_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text.char_indices() {
        if escape_next { escape_next = false; continue; }
        if ch == '\\' && in_string { escape_next = true; continue; }
        if ch == '"' { in_string = !in_string; continue; }
        if in_string { continue; } // 跳过字符串内容

        match ch {
            '{' => {
                if depth == 0 { start = Some(i); }
                depth += 1;
            }
            '}' => {
                if depth > 0 { depth -= 1; }
                if depth == 0 {
                    if let Some(s) = start {
                        let block = text[s..=i].to_string();
                        if block.contains("\"action\"") || block.contains("\"actions\"") {
                            // 验证是否为合法 JSON
                            if serde_json::from_str::<serde_json::Value>(&block).is_ok() {
                                blocks.push(block);
                            }
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    blocks
}

/// 格式化 Web 搜索结果供 LLM 上下文注入
fn format_search_for_llm(results: &[WebSearchResult]) -> String {
    if results.is_empty() {
        return "No search results found.".into();
    }
    let mut out = String::from("## Web Search Results\n\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "{}. **{}**\n   URL: {}\n   {}\n   Source: {} | Score: {:.1}\n\n",
            i + 1, r.title, r.url, r.snippet, r.source, r.relevance_score
        ));
    }
    out
}

/// 格式化 Web 抓取结果供 LLM 上下文注入
fn format_fetch_for_llm(result: &WebFetchResult) -> String {
    if !result.success {
        return format!("Web fetch failed: {}", result.error.as_deref().unwrap_or("unknown error"));
    }
    let mut out = format!("## Web Fetch: {}\n\n", result.title);
    if result.distilled {
        out.push_str(&format!(
            "> Content distilled from {} bytes. Key points:\n\n",
            result.content_length
        ));
        for point in &result.key_points {
            out.push_str(&format!("- {}\n", point));
        }
        if let Some(ref summary) = result.distilled_summary {
            out.push_str(&format!("\n### Summary\n\n{}\n", summary));
        }
    } else {
        out.push_str(&result.content);
    }
    out
}

/// 核心行动调度器 — 验证 + 执行 + 返回结果文本
async fn dispatch_action(
    state: &AppState,
    action: &AgentAction,
) -> Result<String, String> {
    match action {
        AgentAction::WebSearch { query, engine, max_results } => {
            tracing::info!("[DISPATCH] WebSearch: {}", query);
            let mut wi = state.web_intelligence.lock().await;
            let results = wi.search(query, engine.as_deref(), *max_results).await?;
            Ok(format_search_for_llm(&results))
        }
        AgentAction::WebFetch { url, distill } => {
            tracing::info!("[DISPATCH] WebFetch: {}", url);
            // 用户显式要求抓取 → 用永不言弃抓取器 (无白名单限制, 多策略降级)
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build().map_err(|e| e.to_string())?;
            let mut domain_states = std::collections::HashMap::new();
            let result = crate::agent::indomitable_fetcher::indomitable_fetch(
                url, &client, &mut domain_states, 0,
            ).await;
            if result.success {
                let _ = distill;
                Ok(format!(
                    "✅ 抓取成功: {}\n标题: {}\n\n{}",
                    result.url,
                    result.title.as_deref().unwrap_or("(无标题)"),
                    &result.main_content.chars().take(4000).collect::<String>()
                ))
            } else {
                // 回退到白名单 web_intelligence 抓取
                let mut wi = state.web_intelligence.lock().await;
                let r = wi.fetch(url, distill.unwrap_or(true)).await?;
                Ok(format_fetch_for_llm(&r))
            }
        }
        AgentAction::FileRead { path, range: _ } => {
            let guard = state.cvfs.lock().await;
            let projects = guard.get_projects().await;
            let project_root = projects.first().map(|(_, r)| r.clone())
                .unwrap_or_else(|| state.sandbox.lock().unwrap().project_root.clone());
            drop(guard);
            let full_path = project_root.join(path);
            std::fs::read_to_string(&full_path)
                .map_err(|e| format!("File read failed: {} — {}", path, e))
        }
        AgentAction::Terminal { command, cwd: _ } => {
            tracing::info!("[DISPATCH] Terminal: {}", command);
            let output = std::process::Command::new("cmd")
                .args(["/C", command])
                .output()
                .map_err(|e| format!("Command execution failed: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                Ok(format!("Command succeeded:\n```\n{}\n```", stdout))
            } else {
                Ok(format!("Command failed (exit {}):\n```\n{}\n```\nStderr:\n```\n{}\n```",
                    output.status.code().unwrap_or(-1), stdout, stderr))
            }
        }
        AgentAction::ExecuteSkill { name, args } => {
            // Skill execution via sync blocking to avoid Send issues
            let name_c = name.clone();
            let _args_c = args.clone();
            let result = tokio::task::spawn_blocking(move || {
                // Use a fresh SkillEngine for this blocking call
                let _engine = crate::agent::skill_engine::SkillEngine::new();
                // Note: in production this should use the real engine from state
                // For now, return a placeholder since skills require filesystem access
                Err::<String, String>(format!("Skill '{}' requires local filesystem access", name_c))
            }).await.map_err(|e| format!("Skill spawn failed: {}", e))?;
            Err(result.err().unwrap_or_else(|| format!("Skill '{}' execution failed", name)))
        }
        AgentAction::McpCall { server_id, tool_name, args } => {
            let mcp = state.mcp_client.lock().await;
            let result = mcp.call_tool(server_id, tool_name, args).await;
            if result.success {
                if let Some(ref distilled) = result.distilled {
                    Ok(format!("MCP Result (distilled {}→{}):\n{}",
                        distilled.original_size, distilled.distilled_size, distilled.summary))
                } else {
                    Ok(serde_json::to_string(&result.data).unwrap_or_default())
                }
            } else {
                Err(result.error.unwrap_or_else(|| format!("MCP call '{}' on '{}' failed", tool_name, server_id)))
            }
        }
        AgentAction::CheckEnvironment => {
            let profile = crate::agent::env_checker::get_environment_profile();
            let tool_status: Vec<String> = profile.tools.iter().map(|t| {
                format!("{} {}: {}", if t.installed { "✅" } else { "❌" }, t.name,
                    t.version.as_deref().unwrap_or(if t.installed { "已安装" } else { "未安装" }))
            }).collect();
            Ok(format!(
                "🖥️ 环境剖面:\n\
                OS: {} ({})\n\
                主机: {} @ {}\n\
                主目录: {}\n\
                CPU核心: {} | 磁盘剩余: {:.1}GB\n\n\
                🔧 工具:\n{}\n\n\
                💡 缺失 {} 项工具, 需要安装请告诉我。",
                profile.os, profile.arch, profile.user, profile.hostname,
                profile.home_dir, profile.cpu_cores, profile.disk_free_gb,
                tool_status.join("\n"),
                profile.tools.iter().filter(|t| !t.installed).count()
            ))
        }
        AgentAction::AutoInstallDeps => {
            let report = crate::agent::env_checker::check_environment();
            let results = crate::agent::env_checker::auto_install_missing(&report);
            Ok(format!("🔧 自动安装:\n{}", results.join("\n")))
        }
        AgentAction::PptxGenerate { title, subtitle, author, template, slides } => {
            tracing::info!("[DISPATCH] PptxGenerate: {} ({} slides)", title, slides.as_array().map(|a| a.len()).unwrap_or(0));
            let req = crate::agent::pptx_engine::PptGenerationRequest {
                title: title.clone(),
                subtitle: subtitle.clone(),
                author: author.clone(),
                template: template.as_deref().map(|t| match t {
                    "Corporate"|"企业商务" => crate::agent::pptx_engine::PptTemplate::Corporate,
                    "TechMinimal"|"科技极简" => crate::agent::pptx_engine::PptTemplate::TechMinimal,
                    "Creative"|"创意设计" => crate::agent::pptx_engine::PptTemplate::Creative,
                    "Academic"|"学术答辩" => crate::agent::pptx_engine::PptTemplate::Academic,
                    "MinimalWhite"|"极简白" => crate::agent::pptx_engine::PptTemplate::MinimalWhite,
                    "DarkMode"|"暗夜模式" => crate::agent::pptx_engine::PptTemplate::DarkMode,
                    "vercel_monochrome"|"Vercel"|"VercelMonochrome" => crate::agent::pptx_engine::PptTemplate::VercelMonochrome,
                    "linear_dark_neon"|"Linear"|"LinearDarkNeon" => crate::agent::pptx_engine::PptTemplate::LinearDarkNeon,
                    "apple_minimalist"|"Apple"|"AppleMinimalist" => crate::agent::pptx_engine::PptTemplate::AppleMinimalist,
                    _ => crate::agent::pptx_engine::PptTemplate::Corporate,
                }),
                slides: serde_json::from_value(slides.clone()).unwrap_or_default(),
                reference_url: None,
                output_path: {
                    let cvfs = state.cvfs.lock().await;
                    cvfs.get_projects().await.first().map(|(_, r)| {
                        let safe_name: String = title.chars()
                            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                            .collect();
                        r.join(format!("{}.pptx", safe_name.trim())).to_string_lossy().to_string()
                    })
                },
            };
            let engine = crate::agent::pptx_engine::PptxEngine::new();
            let result = engine.generate(&req);
            if result.success {
                Ok(format!("✅ PPT 已生成!\n📄 文件: {}\n📊 模板: {}\n📝 幻灯片: {} 页\n💡 安装 python-pptx 后自动生成 .pptx 文件: pip install python-pptx",
                    result.file_path.as_deref().unwrap_or("output.pptx"),
                    result.template_used, result.slide_count))
            } else {
                Err(result.error.unwrap_or_else(|| "PPT 生成失败".into()))
            }
        }
        AgentAction::FileEdit { path, content } => {
            tracing::info!("[DISPATCH] FileEdit: {} ({} bytes)", path, content.len());
            // Get project root from C-VFS, fallback to sandbox
            let project_root = {
                let guard = state.cvfs.lock().await;
                let projects = guard.get_projects().await;
                projects.first().map(|(_, r)| r.clone())
                    .unwrap_or_else(|| state.sandbox.lock().unwrap().project_root.clone())
            };
            let full_path = project_root.join(path);
            if content.len() > 10 * 1024 * 1024 {
                return Err(format!("File too large: {} bytes", content.len()));
            }
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
            }
            std::fs::write(&full_path, content)
                .map_err(|e| format!("Write failed: {}", e))?;
            tracing::info!("[DISPATCH] File written to {:?}", full_path);
            Ok(format!("✅ 文件已写入: {} ({} bytes)", path, content.len()))
        }
    }
}

/// Tauri Command: 解析并执行单个 LLM 动作
#[tauri::command]
pub async fn execute_agent_action(
    state: tauri::State<'_, AppState>,
    action_json: String,
) -> Result<String, String> {
    // 红线一校验 (drop before await)
    let action = {
        let redline = state.redline.lock().unwrap();
        redline.validate_and_parse(&action_json)
            .map_err(|e| format!("Redline validation failed: {}", e))?
    };

    // 安全边界校验 (drop before await)
    {
        let mut boundary = state.security_boundary.lock().unwrap();
        let scan_text = format!("{:?}", action);
        let violations = boundary.scan_llm_output(&scan_text);
        if !violations.is_empty() {
            return Err(format!(
                "Security boundary blocked: {}",
                violations.iter().map(|d| d.reason.clone()).collect::<Vec<_>>().join("; ")
            ));
        }
    }

    dispatch_action(&state, &action).await
}

/// 从 LLM 响应中提取代码块并自动保存为文件
async fn extract_and_save_code_blocks(
    _state: &AppState,
    text: &str,
) -> Vec<String> {
    let mut files = Vec::new();
    let re = regex::Regex::new(r"```(\w+)?(?:\s+(.+))?\n([\s\S]*?)```").unwrap();

    for cap in re.captures_iter(text) {
        let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let hint = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        let code = cap[3].trim();

        if code.len() < 20 { continue; } // 跳过太短的片段

        // 推断文件名
        let filename = infer_filename(lang, hint, code);
        let path = std::path::Path::new(&filename);
        // 获取 C-VFS 项目根目录 (首个项目路径)
        let project_root = {
            let cvfs = _state.cvfs.lock().await;
            cvfs.get_projects().await.first()
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        };
        let full_path = project_root.join(path);

        // 红线二：自动保存同样受路径沙盒约束，拒绝绝对路径 / 驱动器前缀 / .. 穿越 / 符号链接逃逸
        if !is_path_within_root(&project_root, path, &full_path) {
            tracing::warn!("[AutoSave] Blocked path escape attempt: {}", filename);
            continue;
        }

        if let Some(parent) = full_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&full_path, code).is_ok() {
            // 返回相对路径 (相对于项目根), 便于后续 cvfs_read_file 回读
            let rel = filename.clone();
            tracing::info!("[AutoSave] Code block → {} (full: {})", rel, full_path.to_string_lossy());
            files.push(rel);
        }
    }
    files
}

/// 校验自动保存的目标路径不逃逸出项目根目录。
/// 拒绝绝对路径、驱动器前缀、根目录、`..` 穿越；对已存在路径（或其父目录）
/// 做 canonicalize 校验，拦截符号链接/目录联接逃逸。
fn is_path_within_root(
    root: &std::path::Path,
    rel: &std::path::Path,
    full: &std::path::Path,
) -> bool {
    use std::path::Component;
    if rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return false;
    }
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    match full.canonicalize() {
        Ok(c) => c.starts_with(&root_canon),
        // 目标文件尚不存在：校验其最近父目录
        Err(_) => full
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .map(|pc| pc.starts_with(&root_canon))
            .unwrap_or(false),
    }
}

/// 根据语言和内容推断文件名 — 支持任意文件类型，无格式限制
fn infer_filename(lang: &str, hint: &str, code: &str) -> String {
    // 1. 用户明确指定文件名 (含扩展名) → 直接使用
    if !hint.is_empty() && hint.contains('.') {
        return hint.to_string();
    }
    // 2. 用户指定文件名但无扩展名 → 用语言作为扩展名
    if !hint.is_empty() && !hint.contains('.') {
        let ext = if lang.is_empty() { "txt" } else { lang };
        return format!("{}.{}", hint, ext);
    }

    // 3. 常见语言映射 (仅作友好别名)
    let ext = match lang {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yml",
        "markdown" | "md" => "md",
        "sql" => "sql",
        "sh" | "bash" => "sh",
        "powershell" | "ps1" => "ps1",
        "java" => "java",
        "go" => "go",
        "cpp" | "c++" => "cpp",
        "c" => "c",
        "svg" => "svg",
        "xml" => "xml",
        "ini" | "conf" => "ini",
        "csv" => "csv",
        "log" => "log",
        "txt" | "text" | "" => "txt",
        // 未知语言 → 直接用语言名作为扩展名，不强制限制
        _ => lang,
    };

    // 4. 尝试从代码首行注释推断具体文件名
    let first_line = code.lines().next().unwrap_or("");
    if first_line.starts_with("//") || first_line.starts_with("#") {
        let comment = first_line.trim_start_matches("//").trim_start_matches("#").trim();
        if comment.len() > 3 && comment.len() < 60 && !comment.contains(' ') {
            return format!("{}.{}", comment, ext);
        }
    }

    format!("generated_{}.{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"), ext)
}

/// Tauri Command: 从 LLM 响应中提取并执行所有动作，返回纯文本结果
/// 如果响应中不含动作块，返回原始文本
#[tauri::command]
pub async fn extract_and_execute_actions(
    state: tauri::State<'_, AppState>,
    llm_response: String,
) -> Result<serde_json::Value, String> {
    let blocks = extract_action_blocks(&llm_response);

    // 🔬 代码块自动保存 — 无论是否有 JSON 动作都执行
    let auto_files = extract_and_save_code_blocks(&state, &llm_response).await;

    if blocks.is_empty() {
        // 无 JSON 动作, 但可能已保存代码块
        if auto_files.is_empty() {
            return Ok(serde_json::json!({
                "has_actions": false,
                "text_response": llm_response,
                "action_results": []
            }));
        }
        let summary = auto_files.iter()
            .map(|f| format!("  ✅ {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(serde_json::json!({
            "has_actions": true,
            "text_response": llm_response,
            "action_results": [],
            "combined_context": format!("✅ 自动保存文件:\n{}", summary),
            "files_created": auto_files,
            "files_summary": format!("📁 自动保存 {} 个代码文件:\n{}", auto_files.len(), summary),
        }));
    }

    let mut action_results = Vec::new();
    let mut combined_results = String::new();
    let mut files_created: Vec<String> = Vec::new();
    let mut files_read: Vec<String> = Vec::new();

    for block in &blocks {
        let action_json = block.clone();
        match execute_agent_action_inner(&state, &action_json).await {
            Ok(result_text) => {
                combined_results.push_str(&result_text);
                combined_results.push_str("\n");
                // Track file operations
                if let Ok(action) = serde_json::from_str::<serde_json::Value>(&action_json) {
                    if action["action"] == "file_edit" {
                        if let Some(path) = action["params"]["path"].as_str() {
                            files_created.push(path.to_string());
                        }
                    } else if action["action"] == "pptx_generate" {
                        let name = action["params"]["title"].as_str().unwrap_or("presentation");
                        let safe: String = name.chars()
                            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                            .collect();
                        files_created.push(format!("{}.pptx", safe.trim()));
                    } else if action["action"] == "check_environment" || action["action"] == "auto_install_deps" {
                        // 环境检测/安装不产生文件
                    } else if action["action"] == "file_read" {
                        if let Some(path) = action["params"]["path"].as_str() {
                            files_read.push(path.to_string());
                        }
                    }
                }
                action_results.push(serde_json::json!({
                    "action": block, "success": true, "result": result_text
                }));
            }
            Err(e) => {
                tracing::warn!("[DISPATCH] Action failed: {}", e);
                action_results.push(serde_json::json!({
                    "action": block, "success": false, "error": e
                }));
            }
        }
    }

    // 代码块已在函数开头自动保存, 这里合并到 files_created
    for f in &auto_files { files_created.push(f.clone()); }

    // Build file operations summary
    let mut summary = String::new();
    if !files_created.is_empty() {
        summary.push_str(&format!("📁 Created {} files:\n", files_created.len()));
        for f in &files_created { summary.push_str(&format!("  ✅ {}\n", f)); }
    }
    if !files_read.is_empty() {
        summary.push_str(&format!("📖 Read {} files:\n", files_read.len()));
        for f in &files_read { summary.push_str(&format!("  📄 {}\n", f)); }
    }

    Ok(serde_json::json!({
        "has_actions": true,
        "text_response": llm_response,
        "action_results": action_results,
        "combined_context": combined_results,
        "files_created": files_created,
        "files_read": files_read,
        "files_summary": summary,
    }))
}

/// 内部函数：无需 State 参数的执行路径
async fn execute_agent_action_inner(
    state: &AppState,
    action_json: &str,
) -> Result<String, String> {
    let action = {
        let redline = state.redline.lock().unwrap();
        redline.validate_and_parse(action_json)
            .map_err(|e| format!("Redline validation failed: {}", e))?
    };

    {
        let mut boundary = state.security_boundary.lock().unwrap();
        let scan_text = format!("{:?}", action);
        let violations = boundary.scan_llm_output(&scan_text);
        if !violations.is_empty() {
            return Err(format!(
                "Security boundary blocked: {}",
                violations.iter().map(|d| d.reason.clone()).collect::<Vec<_>>().join("; ")
            ));
        }
    }

    dispatch_action(state, &action).await
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_within_root_rejects_traversal() {
        let root = std::path::PathBuf::from("D:/project");
        let rel = std::path::Path::new("../../etc/passwd");
        assert!(!is_path_within_root(&root, rel, &root.join(rel)));
    }

    #[test]
    fn test_is_path_within_root_rejects_absolute() {
        let root = std::path::PathBuf::from("D:/project");
        let rel = std::path::Path::new("C:/Windows/System32/x.dll");
        assert!(!is_path_within_root(&root, rel, &root.join(rel)));
    }

    #[test]
    fn test_is_path_within_root_rejects_drive_relative() {
        let root = std::path::PathBuf::from("D:/project");
        let rel = std::path::Path::new("C:evil.txt");
        assert!(!is_path_within_root(&root, rel, &root.join(rel)));
    }
}
