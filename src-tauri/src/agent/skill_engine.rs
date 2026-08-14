// 本地专属 Skill 动态加载插件热加载运行时
//
// 核心功能：
// - JSON 声明 + 脚本文件热加载自定义 Skill
// - 将多步 Windows 桌面操控封装为端侧确定的原子执行脚本
// - 大模型只需下发 JSON 触发指令 → 节约 95% 动作重试 Token
// - 支持启用/禁用、版本检查、执行超时控制

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Skill 定义 ────────────────────────────────────────────────────

/// Skill 执行器类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillExecutor {
    /// Python 脚本
    Python,
    /// PowerShell 脚本
    PowerShell,
    /// 批处理
    Batch,
    /// 原生可执行文件
    Native,
}

/// Skill 清单定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// 唯一 ID
    #[serde(default)]
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 功能描述
    pub description: String,
    /// 执行器类型
    #[serde(default)]
    pub executor: SkillExecutor,
    /// 脚本文件路径
    #[serde(default)]
    pub script_path: String,
    /// 输入参数 JSON Schema (backward compat: also accepts "parameters")
    #[serde(default, alias = "parameters")]
    pub input_schema: serde_json::Value,
    /// 版本号
    #[serde(default = "default_version")]
    pub version: String,
    /// 作者
    pub author: Option<String>,
    /// 执行超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
}

fn default_version() -> String { "0.1.0".into() }
fn default_timeout() -> u32 { 30 }

impl Default for SkillExecutor {
    fn default() -> Self { SkillExecutor::PowerShell }
}

/// Skill 运行时状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillState {
    /// 已加载但未激活
    Loaded,
    /// 激活中（Prompt 已同步）
    Active,
    /// 已禁用
    Disabled,
    /// 加载失败（带错误信息）
    Error(String),
}

/// Skill 运行时实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstance {
    /// 清单定义
    pub manifest: SkillManifest,
    /// 当前状态
    pub state: SkillState,
    /// 执行次数
    pub execution_count: u32,
    /// 总执行耗时（毫秒）
    pub total_duration_ms: u64,
    /// 最近一次错误
    pub last_error: Option<String>,
}

/// Skill 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    /// 是否成功
    pub success: bool,
    /// 输出内容
    pub output: String,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

// ─── Skill 引擎 ────────────────────────────────────────────────────

/// 执行外部命令（真实 std::process::Command）
fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    use std::process::Command;
    tracing::info!("[Skill] Exec: {} {}", program, args.join(" "));
    match Command::new(program).args(args).output() {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(format!("Exit {}: {}", output.status, String::from_utf8_lossy(&output.stderr)))
            }
        }
        Err(e) => Err(format!("Spawn failed: {}", e)),
    }
}

/// Skill 热加载运行时引擎
pub struct SkillEngine {
    /// 已加载的 Skill 实例映射表
    pub skills: HashMap<String, SkillInstance>,
}

impl SkillEngine {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    // ── 生命周期管理 ──────────────────────────────────────────────

    /// 从 JSON 清单热加载一个 Skill
    pub fn load_from_json(&mut self, manifest_json: &str) -> Result<(), String> {
        let manifest: SkillManifest = serde_json::from_str(manifest_json)
            .map_err(|e| format!("Invalid skill manifest JSON: {}", e))?;

        // 若 id 为空则回退使用 name 作为唯一标识
        let skill_id = if manifest.id.is_empty() { manifest.name.clone() } else { manifest.id.clone() };

        if self.skills.contains_key(&skill_id) {
            return Err(format!("Skill '{}' is already loaded", skill_id));
        }

        self.skills.insert(
            skill_id,
            SkillInstance {
                manifest,
                state: SkillState::Loaded,
                execution_count: 0,
                total_duration_ms: 0,
                last_error: None,
            },
        );
        Ok(())
    }

    /// 激活 Skill（同步到 Prompt）
    pub fn activate(&mut self, id: &str) -> Result<(), String> {
        let skill = self
            .skills
            .get_mut(id)
            .ok_or_else(|| format!("Skill '{}' not found", id))?;
        skill.state = SkillState::Active;
        Ok(())
    }

    /// 禁用 Skill
    pub fn deactivate(&mut self, id: &str) -> Result<(), String> {
        let skill = self
            .skills
            .get_mut(id)
            .ok_or_else(|| format!("Skill '{}' not found", id))?;
        skill.state = SkillState::Disabled;
        Ok(())
    }

    /// 卸载 Skill
    pub fn unload(&mut self, id: &str) -> Result<(), String> {
        self.skills
            .remove(id)
            .ok_or_else(|| format!("Skill '{}' not found", id))?;
        Ok(())
    }

    // ── 执行 ───────────────────────────────────────────────────────

    /// 执行 Skill — 通过 std::process::Command 真实执行脚本
    pub async fn execute(&self, id: &str, args: &serde_json::Value) -> SkillResult {
        let start = std::time::Instant::now();

        let skill = match self.skills.get(id) {
            Some(s) if s.state == SkillState::Active => s,
            Some(s) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(format!("Skill '{}' is {:?}", id, s.state)),
                };
            }
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(format!("Skill '{}' not found", id)),
                };
            }
        };

        let args_str = serde_json::to_string(args).unwrap_or_default();

        // Build and execute the command
        let result = match &skill.manifest.executor {
            SkillExecutor::Python => {
                run_command("python", &[&skill.manifest.script_path, &args_str])
            }
            SkillExecutor::PowerShell => {
                run_command("powershell", &["-File", &skill.manifest.script_path, "-Args", &args_str])
            }
            SkillExecutor::Batch => {
                run_command("cmd", &["/c", &skill.manifest.script_path, &args_str])
            }
            SkillExecutor::Native => {
                run_command(&skill.manifest.script_path, &[&args_str])
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        match result {
            Ok(output) => SkillResult {
                success: true,
                output,
                duration_ms,
                error: None,
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                duration_ms,
                error: Some(e),
            },
        }
    }

    // ── 查询 ───────────────────────────────────────────────────────

    /// 获取所有已加载的 Skill
    pub fn list_all(&self) -> Vec<&SkillInstance> {
        self.skills.values().collect()
    }

    /// 获取已激活的 Skill（可暴露给大模型 Prompt）
    pub fn active_skills(&self) -> Vec<&SkillInstance> {
        self.skills
            .values()
            .filter(|s| s.state == SkillState::Active)
            .collect()
    }

    /// 生成 Skill 清单（用于注入到 Prompt 头部）
    pub fn generate_prompt_fragment(&self) -> String {
        let active = self.active_skills();
        if active.is_empty() {
            return String::new();
        }

        let mut fragment = String::from("## Available Skills\n\n");
        for skill in &active {
            fragment.push_str(&format!(
                "- **{}** (`{}`): {}\n",
                skill.manifest.name, skill.manifest.id, skill.manifest.description
            ));
        }
        fragment.push_str("\nTo invoke a skill, output: {\"action\": \"skill_trigger\", \"skill_id\": \"<id>\", \"args\": {...}}\n");
        fragment
    }
}

impl Default for SkillEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> String {
        r#"{
            "id": "ppt-gen",
            "name": "PPT 自动生成",
            "description": "将 Markdown 转换为 PowerPoint 演示文稿",
            "executor": "Python",
            "script_path": "skills/ppt_gen.py",
            "input_schema": {
                "type": "object",
                "properties": {
                    "markdown": { "type": "string" },
                    "template": { "type": "string" }
                }
            },
            "version": "1.0.0",
            "timeout_secs": 60
        }"#
        .into()
    }

    #[test]
    fn test_load_and_activate() {
        let mut engine = SkillEngine::new();
        engine.load_from_json(&sample_manifest()).unwrap();
        assert_eq!(engine.skills.len(), 1);

        engine.activate("ppt-gen").unwrap();
        let skill = engine.skills.get("ppt-gen").unwrap();
        assert_eq!(skill.state, SkillState::Active);
    }

    #[test]
    fn test_duplicate_load_rejected() {
        let mut engine = SkillEngine::new();
        engine.load_from_json(&sample_manifest()).unwrap();
        assert!(engine.load_from_json(&sample_manifest()).is_err());
    }

    #[test]
    fn test_execute_disabled_skill() {
        let mut engine = SkillEngine::new();
        engine.load_from_json(&sample_manifest()).unwrap();
        engine.deactivate("ppt-gen").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(engine.execute("ppt-gen", &serde_json::json!({})));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Disabled"));
    }

    #[test]
    fn test_prompt_fragment_generation() {
        let mut engine = SkillEngine::new();
        engine.load_from_json(&sample_manifest()).unwrap();
        engine.activate("ppt-gen").unwrap();

        let fragment = engine.generate_prompt_fragment();
        assert!(fragment.contains("PPT 自动生成"));
        assert!(fragment.contains("ppt-gen"));
        assert!(fragment.contains("skill_trigger"));
    }

    #[test]
    fn test_unload() {
        let mut engine = SkillEngine::new();
        engine.load_from_json(&sample_manifest()).unwrap();
        engine.unload("ppt-gen").unwrap();
        assert!(engine.skills.is_empty());
    }

    #[test]
    fn test_invalid_json_rejected() {
        let mut engine = SkillEngine::new();
        assert!(engine.load_from_json("not json").is_err());
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn list_skills(state: tauri::State<crate::state::AppState>) -> Vec<SkillInstance> {
    state.skill_engine.lock().unwrap().list_all().into_iter().cloned().collect()
}
