// 三大防幻觉红线硬核拦截器与强类型反序列化 Schema 校验底座
//
// 红线一：Schema 格式物理强校验 — 大模型输出必须 100% 符合 JSON Schema
//          Rust 强类型反序列化 (serde) 毫秒级打回格式不匹配、字段缺失、多余标点
// 红线二：文件与操作非零推定白名单 — 所有写操作锁定在沙盒 Scope 内
//          路径穿越检测、非法 import 扫描、危险操作拦截
// 红线三：死循环自愈熔断机制 — Max_Healing_Loop = 3
//          连续修复失败 → 锁死节点 + 红屏报警，杜绝计费无底洞

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 格式拦截错误枚举 — 对齐白皮书 RedlineError
#[derive(Debug, thiserror::Error)]
pub enum RedlineError {
    #[error("红线一：格式幻觉！必须为纯净JSON数据，检测到自然语言废话或非法包裹: {0}")]
    FormatViolation(String),
    #[error("红线二：空间幻觉！操作路径 [{0}] 逃逸出受保护的物理沙盒边界")]
    SandboxViolation(PathBuf),
    #[error("红线三：死循环熔断！任务 [{task_id}] 连续自愈失败 {count} 次后强制截断")]
    HealingFuse { task_id: String, count: u32 },
}

// ─── 红线一：Schema 定义 ───────────────────────────────────────────

/// 大模型输出的操作指令 — 白名单内的操作类型
/// 对齐白皮书格式：{"action": "file_edit", "params": {"path": "...", "content": "..."}}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", content = "params", rename_all = "snake_case")]
pub enum AgentAction {
    /// 文件编辑操作
    #[serde(rename = "file_edit")]
    FileEdit {
        path: String,
        content: String,
    },
    /// 文件只读操作
    #[serde(rename = "file_read")]
    FileRead {
        path: String,
        range: Option<String>,
    },
    /// 终端命令执行
    #[serde(rename = "terminal")]
    Terminal {
        command: String,
        cwd: Option<String>,
    },
    /// Skill 触发指令
    #[serde(rename = "execute_skill")]
    ExecuteSkill {
        name: String,
        args: serde_json::Value,
    },
    /// MCP 工具调用
    #[serde(rename = "mcp_call")]
    McpCall {
        server_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    /// Web 搜索引擎查询
    #[serde(rename = "web_search")]
    WebSearch {
        query: String,
        /// 可选：指定搜索引擎 (bing/google/duckduckgo)
        engine: Option<String>,
        /// 可选：返回结果数量上限
        max_results: Option<u32>,
    },
    /// Web 网页只读抓取
    #[serde(rename = "web_fetch")]
    WebFetch {
        url: String,
        /// 可选：是否启用端侧蒸馏
        distill: Option<bool>,
    },
    /// 环境检测
    #[serde(rename = "check_environment")]
    CheckEnvironment,
    /// 自动安装依赖
    #[serde(rename = "auto_install_deps")]
    AutoInstallDeps,
    /// PPT 生成
    #[serde(rename = "pptx_generate")]
    PptxGenerate {
        title: String,
        subtitle: Option<String>,
        author: Option<String>,
        template: Option<String>,
        slides: serde_json::Value, // Vec<SlideContent>
    },
}

/// 大模型完整输出 — 入口 Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// 操作序列（不允许空数组）
    pub actions: Vec<AgentAction>,
    /// 操作的简要说明（仅用于日志，不影响执行）
    #[serde(default)]
    pub summary: Option<String>,
}

// ─── 红线校验结果 ──────────────────────────────────────────────────

/// 红线校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "level", rename_all = "snake_case")]
pub enum RedlineResult {
    Pass,
    Warn {
        message: String,
        code: String,
    },
    Fail {
        message: String,
        code: String,
    },
    Fused {
        message: String,
        task_id: String,
        healing_count: u32,
    },
}

impl RedlineResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }
    pub fn is_fused(&self) -> bool {
        matches!(self, Self::Fused { .. })
    }
}

/// 红线校验摘要 — 供前端 RedlineGuardPanel 实时展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedlineStatus {
    /// 红线一：Schema 校验器状态
    pub schema_active: bool,
    pub schema_last_check: Option<String>,
    /// 红线二：沙盒白名单状态
    pub sandbox_active: bool,
    pub sandbox_root: String,
    pub blocked_paths: u32,
    /// 红线三：熔断状态
    pub healing_enabled: bool,
    pub max_loop: u32,
    pub current_loop: u32,
    pub fused: bool,
}

// ─── Schema 校验配置 ───────────────────────────────────────────────

/// 终端命令白名单 — 仅放行开发/构建/版本控制/只读文件系统等安全程序名。
/// 结合 shell 元字符拒绝，从根上阻断命令链式注入与解释器绕过
/// （`cmd`/`powershell`/`bash`/`wscript`/`rundll32` 等解释器刻意不在白名单内）。
const ALLOWED_COMMANDS: &[&str] = &[
    // 构建 / 包管理
    "npm", "npx", "pnpm", "yarn", "bun",
    "cargo", "rustc", "rustup",
    "node", "python", "py", "pip", "pip3",
    "go", "gofmt",
    "dotnet", "msbuild",
    "java", "javac", "mvn", "gradle",
    "tsc", "vite", "vitest", "eslint", "prettier", "oxlint",
    "make", "cmake", "ninja", "mingw32-make", "gcc", "g++", "clang", "cl",
    "docker", "docker-compose", "kubectl", "helm",
    // 版本控制
    "git", "svn", "hg",
    // 只读 / 查询文件系统 (cmd 内建 + 工具)
    "dir", "ls", "type", "cat", "echo", "cd", "chdir", "pwd",
    "where", "which", "tree", "find", "findstr",
    // 网络 / 归档
    "curl", "wget", "ssh", "scp", "tar", "unzip", "zip", "7z",
    // 环境 / 诊断 (只读)
    "set", "ver", "whoami", "hostname", "systeminfo", "tasklist",
];

/// 危险 shell 元字符 — 出现即拒绝，阻断链式 (`&&`)、管道 (`|`)、重定向 (`>` `<`)、
/// 变量替换 (`%` `!` `$`)、转义 (`^`)、子命令 (`` ` `` `(` `)`) 与换行注入。
fn contains_shell_metachar(command: &str) -> bool {
    command.chars().any(|c| {
        matches!(c,
            '&' | '|' | '<' | '>' | '^' | ';' | '%' | '!' | '`' | '$' | '(' | ')'
            | '\n' | '\r' | '\t'
        )
    })
}

/// SSRF 防护：拦截指向内网/环回/链路本地/云元数据地址的 URL。
/// 字面检测 host（IPv4 私有段 + localhost + 内网后缀），阻断 LLM 被诱导抓取内部 HTTPS 服务。
/// 注：DNS 重绑定与 IPv6 内网地址不在本次覆盖范围。
fn is_internal_url(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|h| h.split(':').next())
        .unwrap_or("")
        .to_lowercase();

    if host.is_empty() {
        return false;
    }

    // localhost / 内网域名后缀
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") {
        return true;
    }

    // 字面 IPv4 私有/保留段
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        if let (Ok(a), Ok(b)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if a == 10 || a == 127 || a == 0
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 168)
                || (a == 169 && b == 254)
            {
                return true;
            }
        }
    }

    false
}

/// 危险文件路径模式
const DANGEROUS_PATHS: &[&str] = &[
    "C:\\Windows\\System32",
    "/etc/passwd",
    "/etc/shadow",
    "~/.ssh",
    "C:\\Users\\All Users",
    "/boot",
    "/sys",
    "/proc",
];

/// Schema 校验配置
#[derive(Debug, Clone)]
pub struct SchemaValidator {
    /// 允许的操作类型白名单
    pub allowed_operations: Vec<String>,
    /// 沙盒项目根目录
    pub project_root: PathBuf,
    /// 累计拦截的路径数
    pub blocked_path_count: u32,
}

impl SchemaValidator {
    /// 校验文件路径是否在沙盒范围内
    pub fn is_path_safe(&self, path_str: &str) -> bool {
        let path = Path::new(path_str);

        // 拒绝绝对路径（除非在项目根下）
        if path.is_absolute() {
            if let Ok(canonical) = path.canonicalize() {
                if let Ok(root) = self.project_root.canonicalize() {
                    return canonical.starts_with(&root);
                }
            }
            // 无法规范化 → 拒绝
            return false;
        }

        // 路径穿越检测
        if path_str.contains("..") {
            let normalized = Path::new(path_str);
            if let Ok(canonical) = self.project_root.join(normalized).canonicalize() {
                if let Ok(root) = self.project_root.canonicalize() {
                    return canonical.starts_with(&root);
                }
            }
            return false;
        }

        // 危险系统路径检测
        let lower = path_str.to_lowercase();
        for dangerous in DANGEROUS_PATHS {
            if lower.contains(&dangerous.to_lowercase()) {
                return false;
            }
        }

        // 相对路径 + 无穿越 → 默认安全（后续由 sandbox 模块做最终校验）
        true
    }

    /// 校验终端命令是否在白名单内且不含 shell 元字符（白名单模式，默认拒绝）
    pub fn is_command_allowed(&self, command: &str) -> bool {
        let trimmed = command.trim();
        if trimmed.is_empty() || trimmed.len() > 1024 {
            return false;
        }
        if contains_shell_metachar(trimmed) {
            return false;
        }
        // 取第一个 token 作为程序名，剥离路径前缀，去掉 .exe/.cmd/.bat 后缀防变体绕过
        let program = trimmed.split_whitespace().next().unwrap_or("");
        let basename = program.rsplit(|c| c == '\\' || c == '/').next().unwrap_or(program);
        let lower = basename.to_lowercase();
        let name: &str = lower
            .strip_suffix(".exe")
            .or_else(|| lower.strip_suffix(".cmd"))
            .or_else(|| lower.strip_suffix(".bat"))
            .unwrap_or(&lower);
        ALLOWED_COMMANDS.contains(&name)
    }
}

// ─── 自愈熔断追踪器 ────────────────────────────────────────────────

/// 自愈状态追踪（每个原子任务独立追踪）
#[derive(Debug, Clone)]
pub struct HealingTracker {
    /// 触发熔断的最大尝试次数
    pub max_loop: u32,
    /// 当前已尝试次数
    pub current_loop: u32,
    /// 是否已熔断
    pub fused: bool,
    /// 最近一次自愈的错误信息
    pub last_error: Option<String>,
}

impl HealingTracker {
    pub fn new(max_loop: u32) -> Self {
        Self {
            max_loop,
            current_loop: 0,
            fused: false,
            last_error: None,
        }
    }

    /// 记录一次自愈尝试
    /// 返回：Ok(()) 继续 / Err(RedlineResult) 触发熔断
    pub fn attempt(&mut self, error_msg: &str) -> Result<(), RedlineResult> {
        if self.fused {
            return Err(RedlineResult::Fused {
                message: "Already fused — circuit breaker active".into(),
                task_id: String::new(),
                healing_count: self.current_loop,
            });
        }

        self.current_loop += 1;
        self.last_error = Some(error_msg.into());

        if self.current_loop > self.max_loop {
            self.fused = true;
            Err(RedlineResult::Fused {
                message: format!(
                    "Max healing loop ({}/{}) reached. Circuit breaker triggered: {}",
                    self.current_loop, self.max_loop, error_msg
                ),
                task_id: String::new(),
                healing_count: self.current_loop,
            })
        } else {
            Ok(())
        }
    }

    /// 重置熔断状态（仅人工介入后可调用）
    pub fn reset(&mut self) {
        self.current_loop = 0;
        self.fused = false;
        self.last_error = None;
    }
}

// ─── 红线拦截器（主结构） ──────────────────────────────────────────

/// 红线拦截器 — 大模型输出进入执行流水线前的最后一道防线
pub struct RedlineGuard {
    pub schema_validator: SchemaValidator,
    pub healing_tracker: HealingTracker,
}

impl RedlineGuard {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            schema_validator: SchemaValidator {
                allowed_operations: vec![
                    "file_edit".into(),
                    "file_read".into(),
                    "terminal".into(),
                    "execute_skill".into(),
                    "mcp_call".into(),
                    "web_search".into(),
                    "web_fetch".into(),
                    "pptx_generate".into(),
                    "check_environment".into(),
                    "auto_install_deps".into(),
                ],
                project_root,
                blocked_path_count: 0,
            },
            healing_tracker: HealingTracker::new(3),
        }
    }

    /// ── 红线一 + 二：结构化 Schema 物理强校验 ──
    ///
    /// 毫秒级拦截包含 "Here is the code" 等自然语言废话的大模型响应
    /// 对齐白皮书 validate_and_parse() 接口
    pub fn validate_and_parse(&self, raw_llm_output: &str) -> Result<AgentAction, RedlineError> {
        let trimmed = raw_llm_output.trim();

        // 严格的前置防御：不允许有任何非JSON字符出现在首尾
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return Err(RedlineError::FormatViolation(raw_llm_output.to_string()));
        }

        // 预处理：修复 LLM 在 JSON 字符串内输出的字面换行
        // 将 JSON 值中的真实换行转义为 \n
        let normalized = normalize_json_newlines(trimmed);

        let action: AgentAction = serde_json::from_str(&normalized)
            .map_err(|e| RedlineError::FormatViolation(format!("{}: {}", e, &normalized[..200.min(normalized.len())])))?;

        // 级联触发红线二校验
        match &action {
            AgentAction::FileEdit { path, .. } => {
                self.validate_path_redline(path)?;
            }
            AgentAction::FileRead { path, .. } => {
                self.validate_path_redline(path)?;
            }
            AgentAction::Terminal { command, .. } => {
                if !self.schema_validator.is_command_allowed(command) {
                    return Err(RedlineError::FormatViolation(format!(
                        "Command not in whitelist or contains shell metacharacters: {}",
                        command
                    )));
                }
            }
            AgentAction::WebSearch { query, .. } => {
                if query.trim().is_empty() {
                    return Err(RedlineError::FormatViolation("WebSearch query must not be empty".into()));
                }
                // 检查是否包含恶意命令注入
                if query.contains("DROP") || query.contains("DELETE") || query.contains("--") {
                    return Err(RedlineError::FormatViolation("WebSearch query contains suspicious content".into()));
                }
            }
            AgentAction::WebFetch { url, .. } => {
                if url.trim().is_empty() {
                    return Err(RedlineError::FormatViolation("WebFetch url must not be empty".into()));
                }
                let lower_url = url.to_lowercase();
                if !lower_url.starts_with("https://") {
                    return Err(RedlineError::FormatViolation(
                        "WebFetch URL must use HTTPS protocol".into()
                    ));
                }
                // 基本 URL 格式校验
                if !lower_url.contains(".") || lower_url.len() < 12 {
                    return Err(RedlineError::FormatViolation(
                        format!("WebFetch URL appears invalid: {}", url)
                    ));
                }
                // SSRF 防护：禁止内网/环回/链路本地地址
                if is_internal_url(&lower_url) {
                    return Err(RedlineError::FormatViolation(
                        "WebFetch URL points to an internal/private address — blocked (SSRF)".into()
                    ));
                }
            }
            _ => {}
        }

        Ok(action)
    }

    /// 红线二：路径物理沙盒校验 — 对齐白皮书 SandboxViolation
    fn validate_path_redline(&self, target_path: &str) -> Result<(), RedlineError> {
        // 复用强校验 is_path_safe（拒绝 .. + canonicalize + 危险路径），
        // 避免词法 starts_with 被 ../ 穿越绕过
        if !self.schema_validator.is_path_safe(target_path) {
            return Err(RedlineError::SandboxViolation(PathBuf::from(target_path)));
        }
        Ok(())
    }

    /// ── 批量校验 AgentOutput（兼容旧接口） ──
    pub fn validate_output(&self, raw: &str) -> Result<AgentOutput, RedlineResult> {
        let output: AgentOutput = serde_json::from_str(raw).map_err(|e| {
            RedlineResult::Fail {
                message: format!("Schema validation failed: {}. Output rejected — no API cost incurred.", e),
                code: "SCHEMA_PARSE_ERROR".into(),
            }
        })?;

        // 额外校验：不允许空 actions
        if output.actions.is_empty() {
            return Err(RedlineResult::Fail {
                message: "AgentOutput.actions must not be empty".into(),
                code: "EMPTY_ACTIONS".into(),
            });
        }

        // 对每个 action 做字段级校验
        for (i, action) in output.actions.iter().enumerate() {
            self.validate_action_fields(action, i)?;
        }

        Ok(output)
    }

    /// 对单个 AgentAction 做字段级安全检查
    fn validate_action_fields(
        &self,
        action: &AgentAction,
        index: usize,
    ) -> Result<(), RedlineResult> {
        match action {
            AgentAction::FileEdit { path, content } => {
                if path.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] FileEdit: path must not be empty", index),
                        code: "EMPTY_PATH".into(),
                    });
                }
                if content.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] FileEdit: content must not be empty", index),
                        code: "EMPTY_CONTENT".into(),
                    });
                }
                if !self.schema_validator.is_path_safe(path) {
                    return Err(RedlineResult::Fail {
                        message: format!(
                            "Action[{}] FileEdit: path '{}' is outside sandbox scope",
                            index, path
                        ),
                        code: "PATH_OUT_OF_SCOPE".into(),
                    });
                }
            }
            AgentAction::FileRead { path, range: _ } => {
                if path.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] FileRead: path must not be empty", index),
                        code: "EMPTY_PATH".into(),
                    });
                }
                if !self.schema_validator.is_path_safe(path) {
                    return Err(RedlineResult::Fail {
                        message: format!(
                            "Action[{}] FileRead: path '{}' is outside sandbox scope",
                            index, path
                        ),
                        code: "PATH_OUT_OF_SCOPE".into(),
                    });
                }
            }
            AgentAction::Terminal { command, cwd: _ } => {
                if command.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] Terminal: command must not be empty", index),
                        code: "EMPTY_COMMAND".into(),
                    });
                }
                if !self.schema_validator.is_command_allowed(command) {
                    return Err(RedlineResult::Fail {
                        message: format!(
                            "Action[{}] Terminal: '{}' is not in the command whitelist — blocked",
                            index, command
                        ),
                        code: "COMMAND_NOT_ALLOWED".into(),
                    });
                }
            }
            AgentAction::ExecuteSkill { name, args: _ } => {
                if name.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] ExecuteSkill: name must not be empty", index),
                        code: "EMPTY_SKILL_NAME".into(),
                    });
                }
            }
            AgentAction::McpCall { server_id, tool_name, args: _ } => {
                if server_id.trim().is_empty() || tool_name.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] McpCall: server_id and tool_name must not be empty", index),
                        code: "EMPTY_MCP_PARAMS".into(),
                    });
                }
            }
            AgentAction::WebSearch { query, engine: _, max_results: _ } => {
                if query.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] WebSearch: query must not be empty", index),
                        code: "EMPTY_QUERY".into(),
                    });
                }
            }
            AgentAction::WebFetch { url, distill: _ } => {
                if url.trim().is_empty() {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] WebFetch: url must not be empty", index),
                        code: "EMPTY_URL".into(),
                    });
                }
                if !url.to_lowercase().starts_with("https://") {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] WebFetch: URL must use HTTPS ({})", index, url),
                        code: "INSECURE_PROTOCOL".into(),
                    });
                }
                if is_internal_url(url) {
                    return Err(RedlineResult::Fail {
                        message: format!("Action[{}] WebFetch: internal/private URL blocked (SSRF): {}", index, url),
                        code: "SSRF_BLOCKED".into(),
                    });
                }
            }
            AgentAction::PptxGenerate { .. } => {}
            AgentAction::CheckEnvironment => {}
            AgentAction::AutoInstallDeps => {}
        }
        Ok(())
    }

    /// ── 红线二：文件路径白名单校验（公开接口） ──
    pub fn validate_path(&mut self, path: &str) -> RedlineResult {
        if self.schema_validator.is_path_safe(path) {
            RedlineResult::Pass
        } else {
            self.schema_validator.blocked_path_count += 1;
            RedlineResult::Fail {
                message: format!("Path '{}' blocked by sandbox — outside project root.", path),
                code: "PATH_BLOCKED".into(),
            }
        }
    }

    /// ── 红线三：自愈熔断 ──
    pub fn check_healing(&mut self, task_id: &str, error: &str) -> Result<(), RedlineResult> {
        self.healing_tracker
            .attempt(error)
            .map_err(|mut result| {
                // 注入 task_id
                if let RedlineResult::Fused { task_id: ref mut tid, .. } = &mut result {
                    *tid = task_id.to_string();
                }
                result
            })
    }

    /// 重置熔断（人工介入后调用）
    pub fn reset_fuse(&mut self) {
        self.healing_tracker.reset();
    }

    /// ── 生成前端展示用的状态摘要 ──
    pub fn get_status(&self) -> RedlineStatus {
        RedlineStatus {
            schema_active: true,
            schema_last_check: None,
            sandbox_active: true,
            sandbox_root: self
                .schema_validator
                .project_root
                .to_string_lossy()
                .into(),
            blocked_paths: self.schema_validator.blocked_path_count,
            healing_enabled: true,
            max_loop: self.healing_tracker.max_loop,
            current_loop: self.healing_tracker.current_loop,
            fused: self.healing_tracker.fused,
        }
    }
}

impl Default for RedlineGuard {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

// ─── JSON 换行修复 ──────────────────────────────────────────────

/// 修复 LLM 在 JSON 字符串值中输出的字面换行符
/// 将字符串值内的 \r\n 和 \n 替换为转义形式 \\n
fn normalize_json_newlines(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut in_string = false;
    let mut escape_next = false;
    for ch in json.chars() {
        if escape_next { escape_next = false; result.push(ch); continue; }
        if ch == '\\' && in_string { escape_next = true; result.push(ch); continue; }
        if ch == '"' { in_string = !in_string; result.push(ch); continue; }
        if in_string && ch == '\n' { result.push_str("\\n"); continue; }
        if in_string && ch == '\r' { continue; }
        result.push(ch);
    }
    result
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_guard() -> RedlineGuard {
        RedlineGuard::new(PathBuf::from("D:/Chronos-Shadow/chronos-shadow"))
    }

    #[test]
    fn test_validate_and_parse_valid() {
        let guard = make_guard();
        let raw = r#"{"action": "file_read", "params": {"path": "src/main.tsx"}}"#;
        let result = guard.validate_and_parse(raw);
        assert!(result.is_ok(), "Valid should pass: {:?}", result.err());
    }

    #[test]
    fn test_validate_and_parse_rejects_natural_language() {
        let guard = make_guard();
        let raw = "Here is the code you requested: {\"action\": \"file_read\"}";
        let result = guard.validate_and_parse(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_valid_output() {
        let guard = make_guard();
        let raw = r#"{
            "actions": [
                {"action": "file_read", "params": {"path": "src/main.tsx"}},
                {"action": "terminal", "params": {"command": "npm run build"}}
            ]
        }"#;
        let result = guard.validate_output(raw);
        assert!(result.is_ok(), "Valid output should pass: {:?}", result.err());
    }

    #[test]
    fn test_schema_rejects_invalid_json() {
        let guard = make_guard();
        let raw = "not json at all";
        let result = guard.validate_output(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_rejects_empty_actions() {
        let guard = make_guard();
        let raw = r#"{"actions": [], "summary": "nothing"}"#;
        let result = guard.validate_output(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_rejects_missing_required_field() {
        let guard = make_guard();
        let raw = r#"{
            "actions": [
                {"action": "file_edit", "params": {"path": "src/main.tsx"}}
            ]
        }"#;
        let result = guard.validate_output(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_outside_project_root() {
        let guard = make_guard();
        let raw = r#"{"action": "file_read", "params": {"path": "C:/Windows/System32/config/SAM"}}"#;
        let result = guard.validate_and_parse(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_dangerous_command_blocked() {
        let guard = make_guard();
        let raw = r#"{"action": "terminal", "params": {"command": "rm -rf /"}}"#;
        let result = guard.validate_and_parse(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_whitelist_blocks_interpreter_injection() {
        let guard = make_guard();
        let raw = r#"{"action": "terminal", "params": {"command": "powershell -Command Remove-Item"}}"#;
        assert!(guard.validate_and_parse(raw).is_err());
    }

    #[test]
    fn test_command_whitelist_blocks_chain_injection() {
        let guard = make_guard();
        let raw = r#"{"action": "terminal", "params": {"command": "npm run build && del /s /q"}}"#;
        assert!(guard.validate_and_parse(raw).is_err());
    }

    #[test]
    fn test_command_whitelist_allows_dev_command() {
        let guard = make_guard();
        let raw = r#"{"action": "terminal", "params": {"command": "cargo test --lib"}}"#;
        assert!(guard.validate_and_parse(raw).is_ok());
    }

    #[test]
    fn test_webfetch_blocks_internal_url() {
        let guard = make_guard();
        let raw = r#"{"action": "web_fetch", "params": {"url": "https://127.0.0.1:8443/admin"}}"#;
        assert!(guard.validate_and_parse(raw).is_err());
    }

    #[test]
    fn test_webfetch_allows_public_url() {
        let guard = make_guard();
        let raw = r#"{"action": "web_fetch", "params": {"url": "https://example.com/docs"}}"#;
        assert!(guard.validate_and_parse(raw).is_ok());
    }

    #[test]
    fn test_healing_tracker_fuse() {
        let mut guard = make_guard();
        // 3 attempts should be OK
        assert!(guard.check_healing("task-1", "error 1").is_ok());
        assert!(guard.check_healing("task-1", "error 2").is_ok());
        assert!(guard.check_healing("task-1", "error 3").is_ok());
        // 4th attempt should fuse
        let result = guard.check_healing("task-1", "error 4");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RedlineResult::Fused { .. }));
        // After reset, should work again
        guard.reset_fuse();
        assert!(guard.check_healing("task-2", "new error").is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let guard = make_guard();
        let raw = r#"{"action": "file_read", "params": {"path": "../../../etc/passwd"}}"#;
        let result = guard.validate_and_parse(raw);
        assert!(result.is_err());
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn get_redline_status(state: tauri::State<crate::state::AppState>) -> RedlineStatus {
    state.redline.lock().unwrap().get_status()
}

#[tauri::command]
pub fn validate_model_output(state: tauri::State<crate::state::AppState>, raw: String) -> Result<String, String> {
    match state.redline.lock().unwrap().validate_output(&raw) {
        Ok(output) => Ok(serde_json::to_string(&output).unwrap_or_default()),
        Err(e) => Err(format!("{:?}", e)),
    }
}

#[tauri::command]
pub fn reset_fuse(state: tauri::State<crate::state::AppState>) -> String {
    state.redline.lock().unwrap().reset_fuse();
    "Fuse reset successfully".into()
}
