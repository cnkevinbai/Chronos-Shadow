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

/// 危险命令模式 — 终端命令黑名单
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "del /F /S C:\\",
    "format ",
    "shutdown",
    "dd if=",
    "mkfs.",
    "> /dev/sda",
    ":(){ :|:& };:", // fork bomb
];

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

    /// 校验终端命令是否安全
    pub fn is_command_safe(&self, command: &str) -> bool {
        let lower = command.to_lowercase();
        for dangerous in DANGEROUS_COMMANDS {
            if lower.contains(&dangerous.to_lowercase()) {
                return false;
            }
        }
        true
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

        let action: AgentAction = serde_json::from_str(trimmed)
            .map_err(|e| RedlineError::FormatViolation(format!("{}: {}", e, &trimmed[..200.min(trimmed.len())])))?;

        // 级联触发红线二校验
        match &action {
            AgentAction::FileEdit { path, .. } => {
                self.validate_path_redline(path)?;
            }
            AgentAction::FileRead { path, .. } => {
                self.validate_path_redline(path)?;
            }
            AgentAction::Terminal { command, .. } => {
                if !self.schema_validator.is_command_safe(command) {
                    return Err(RedlineError::FormatViolation(format!("Dangerous command blocked: {}", command)));
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
                // URL 安全检查：协议必须是 https
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
            }
            _ => {}
        }

        Ok(action)
    }

    /// 红线二：路径物理沙盒校验 — 对齐白皮书 SandboxViolation
    fn validate_path_redline(&self, target_path: &str) -> Result<(), RedlineError> {
        let path = Path::new(target_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.schema_validator.project_root.join(path)
        };

        if !absolute_path.starts_with(&self.schema_validator.project_root) {
            return Err(RedlineError::SandboxViolation(absolute_path));
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
                if !self.schema_validator.is_command_safe(command) {
                    return Err(RedlineResult::Fail {
                        message: format!(
                            "Action[{}] Terminal: '{}' is a dangerous command — blocked",
                            index, command
                        ),
                        code: "DANGEROUS_COMMAND".into(),
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
            }
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
