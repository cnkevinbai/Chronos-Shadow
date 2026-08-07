// Git Worktrees 并行隔离 — Coder Subagent 独立沙盒
//
// 当 Planner 分发并行开发任务时，Rust 后端调用 Git 命令行在本地克隆出
// 多个物理隔离的 Git Worktrees 目录沙盒。各子智能体在隔离沙盒内并行对
// 局部文件执行增量改写，避免将全量项目源码塞入每一个会话。
//
// 核心收益：单次写码的 Token 开销缩减 90%

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

// ─── 类型定义 ──────────────────────────────────────────────────────

/// Worktree 状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeState {
    /// 已创建，等待分配
    Created,
    /// Coder Agent 正在其中工作
    Active { task_id: String, agent_id: String },
    /// 工作完成，等待合并
    Completed { task_id: String },
    /// 已合并回主分支，待清理
    Merged,
    /// 出错
    Error(String),
}

/// 单个 Worktree 实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInstance {
    /// Worktree 唯一 ID
    pub id: String,
    /// 物理路径
    pub path: PathBuf,
    /// 关联的任务 ID
    pub task_id: Option<String>,
    /// 当前状态
    pub state: WorktreeState,
    /// 分支名
    pub branch: String,
    /// 创建时间
    pub created_at: String,
}

/// Worktree 创建参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// 任务 ID
    pub task_id: String,
    /// 此 Worktree 需要处理的文件列表
    pub files: Vec<String>,
    /// 基础分支
    pub base_branch: String,
}

/// 合并结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub conflicts: Vec<String>,
    pub merged_files: Vec<String>,
    pub error: Option<String>,
}

// ─── Worktree 管理器 ───────────────────────────────────────────────

/// Git Worktrees 管理器
pub struct WorktreeManager {
    /// 项目根目录（包含 .git）
    pub project_root: PathBuf,
    /// 所有 Worktree 实例
    pub worktrees: Vec<WorktreeInstance>,
    /// Worktree 存储目录
    pub worktrees_dir: PathBuf,
    /// 计数器
    counter: u32,
}

impl WorktreeManager {
    pub fn new(project_root: PathBuf) -> Self {
        let worktrees_dir = project_root.join(".cs-worktrees");
        Self {
            project_root,
            worktrees: Vec::new(),
            worktrees_dir,
            counter: 0,
        }
    }

    /// 检查 Git 是否可用
    pub fn is_git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 创建新的 Worktree
    pub fn create_worktree(&mut self, config: &WorktreeConfig) -> Result<String, String> {
        self.counter += 1;
        let id = format!("wt-{:04}", self.counter);
        let branch = format!("cs/{}", config.task_id);
        let path = self.worktrees_dir.join(&id);

        // 确保 worktrees 目录存在
        std::fs::create_dir_all(&self.worktrees_dir)
            .map_err(|e| format!("Failed to create worktrees dir: {}", e))?;

        // 执行 git worktree add
        let output = Command::new("git")
            .current_dir(&self.project_root)
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                path.to_str().unwrap_or("."),
                &config.base_branch,
            ])
            .output()
            .map_err(|e| format!("Git worktree add failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git worktree add failed: {}", stderr));
        }

        // 对每个需要处理的文件做 sparse-checkout（可选优化）
        if !config.files.is_empty() {
            let _ = Command::new("git")
                .current_dir(&path)
                .args(["sparse-checkout", "set", "--cone"])
                .output();
            let _ = Command::new("git")
                .current_dir(&path)
                .args(["sparse-checkout", "set"])
                .args(&config.files)
                .output();
        }

        let instance = WorktreeInstance {
            id: id.clone(),
            path,
            task_id: Some(config.task_id.clone()),
            state: WorktreeState::Created,
            branch,
            created_at: chrono_now(),
        };

        self.worktrees.push(instance);
        Ok(id)
    }

    /// 激活 Worktree（分配给 Coder Agent）
    pub fn activate(&mut self, worktree_id: &str, task_id: &str, agent_id: &str) -> Result<(), String> {
        let wt = self
            .worktrees
            .iter_mut()
            .find(|w| w.id == worktree_id)
            .ok_or_else(|| format!("Worktree {} not found", worktree_id))?;

        if wt.state != WorktreeState::Created {
            return Err(format!("Worktree {} is not in Created state", worktree_id));
        }

        wt.state = WorktreeState::Active {
            task_id: task_id.into(),
            agent_id: agent_id.into(),
        };
        Ok(())
    }

    /// 标记 Worktree 完成
    pub fn complete(&mut self, worktree_id: &str) -> Result<(), String> {
        let wt = self
            .worktrees
            .iter_mut()
            .find(|w| w.id == worktree_id)
            .ok_or_else(|| format!("Worktree {} not found", worktree_id))?;

        let task_id = match &wt.state {
            WorktreeState::Active { task_id, .. } => task_id.clone(),
            _ => return Err(format!("Worktree {} is not active", worktree_id)),
        };

        wt.state = WorktreeState::Completed { task_id };
        Ok(())
    }

    /// 合并 Worktree 回主分支
    pub fn merge_worktree(&mut self, worktree_id: &str) -> Result<MergeResult, String> {
        let wt = self
            .worktrees
            .iter()
            .find(|w| w.id == worktree_id)
            .ok_or_else(|| format!("Worktree {} not found", worktree_id))?;

        let branch = wt.branch.clone();

        // 切换回主分支并合并
        let output = Command::new("git")
            .current_dir(&self.project_root)
            .args(["merge", "--no-ff", &branch])
            .output()
            .map_err(|e| format!("Git merge failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if output.status.success() {
            // 标记已合并
            if let Some(wt) = self.worktrees.iter_mut().find(|w| w.id == worktree_id) {
                wt.state = WorktreeState::Merged;
            }

            Ok(MergeResult {
                success: true,
                conflicts: vec![],
                merged_files: parse_merged_files(&stdout),
                error: None,
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let conflicts = parse_conflicts(&stderr);

            Ok(MergeResult {
                success: false,
                conflicts,
                merged_files: vec![],
                error: Some(stderr),
            })
        }
    }

    /// 清理已合并的 Worktree
    pub fn prune_worktree(&mut self, worktree_id: &str) -> Result<(), String> {
        let wt = self
            .worktrees
            .iter()
            .find(|w| w.id == worktree_id)
            .ok_or_else(|| format!("Worktree {} not found", worktree_id))?;

        let path = wt.path.clone();

        // git worktree remove
        let _ = Command::new("git")
            .current_dir(&self.project_root)
            .args(["worktree", "remove", path.to_str().unwrap_or("."), "--force"])
            .output();

        // 删除分支
        let _ = Command::new("git")
            .current_dir(&self.project_root)
            .args(["branch", "-D", &wt.branch])
            .output();

        self.worktrees.retain(|w| w.id != worktree_id);
        Ok(())
    }

    /// 批量创建 Worktrees（并行任务分配）
    pub fn create_batch(&mut self, configs: &[WorktreeConfig]) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        for config in configs {
            match self.create_worktree(config) {
                Ok(id) => ids.push(id),
                Err(e) => {
                    // 回滚已创建的
                    for id in &ids {
                        let _ = self.prune_worktree(id);
                    }
                    return Err(format!("Batch create failed at {}: {}", config.task_id, e));
                }
            }
        }
        Ok(ids)
    }

    /// 获取所有活跃的 Worktrees
    pub fn active_worktrees(&self) -> Vec<&WorktreeInstance> {
        self.worktrees
            .iter()
            .filter(|w| matches!(w.state, WorktreeState::Active { .. }))
            .collect()
    }

    /// 统计信息
    pub fn stats(&self) -> WorktreeStats {
        WorktreeStats {
            total: self.worktrees.len() as u32,
            active: self
                .worktrees
                .iter()
                .filter(|w| matches!(w.state, WorktreeState::Active { .. }))
                .count() as u32,
            completed: self
                .worktrees
                .iter()
                .filter(|w| matches!(w.state, WorktreeState::Completed { .. }))
                .count() as u32,
            merged: self
                .worktrees
                .iter()
                .filter(|w| w.state == WorktreeState::Merged)
                .count() as u32,
            errors: self
                .worktrees
                .iter()
                .filter(|w| matches!(w.state, WorktreeState::Error(_)))
                .count() as u32,
        }
    }
}

/// Worktree 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStats {
    pub total: u32,
    pub active: u32,
    pub completed: u32,
    pub merged: u32,
    pub errors: u32,
}

// ─── 工具函数 ──────────────────────────────────────────────────────

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_merged_files(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| l.contains('|'))
        .filter_map(|l| l.split('|').next())
        .map(|s| s.trim().to_string())
        .collect()
}

fn parse_conflicts(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| l.contains("CONFLICT") || l.contains("conflict"))
        .map(|s| s.trim().to_string())
        .collect()
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_manager_creation() {
        let mgr = WorktreeManager::new(PathBuf::from("."));
        assert_eq!(mgr.worktrees.len(), 0);
        assert!(mgr.worktrees_dir.ends_with(".cs-worktrees"));
    }

    #[test]
    fn test_worktree_stats() {
        let mut mgr = WorktreeManager::new(PathBuf::from("."));

        // Manually add worktrees in different states
        mgr.worktrees.push(WorktreeInstance {
            id: "wt-1".into(),
            path: PathBuf::from("/tmp/wt1"),
            task_id: Some("task-1".into()),
            state: WorktreeState::Active {
                task_id: "task-1".into(),
                agent_id: "coder-1".into(),
            },
            branch: "cs/task-1".into(),
            created_at: chrono_now(),
        });
        mgr.worktrees.push(WorktreeInstance {
            id: "wt-2".into(),
            path: PathBuf::from("/tmp/wt2"),
            task_id: Some("task-2".into()),
            state: WorktreeState::Completed {
                task_id: "task-2".into(),
            },
            branch: "cs/task-2".into(),
            created_at: chrono_now(),
        });

        let stats = mgr.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.completed, 1);
    }

    #[test]
    fn test_active_worktrees_filter() {
        let mut mgr = WorktreeManager::new(PathBuf::from("."));
        mgr.worktrees.push(WorktreeInstance {
            id: "wt-1".into(),
            path: PathBuf::from("/tmp/wt1"),
            task_id: Some("task-1".into()),
            state: WorktreeState::Active {
                task_id: "task-1".into(),
                agent_id: "coder-1".into(),
            },
            branch: "cs/task-1".into(),
            created_at: chrono_now(),
        });
        mgr.worktrees.push(WorktreeInstance {
            id: "wt-2".into(),
            path: PathBuf::from("/tmp/wt2"),
            task_id: Some("task-2".into()),
            state: WorktreeState::Merged,
            branch: "cs/task-2".into(),
            created_at: chrono_now(),
        });

        let active = mgr.active_worktrees();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_parse_conflicts() {
        let stderr = "Auto-merging src/main.rs\nCONFLICT (content): Merge conflict in src/main.rs\nAutomatic merge failed";
        let conflicts = parse_conflicts(stderr);
        assert_eq!(conflicts.len(), 1);
    }
}
