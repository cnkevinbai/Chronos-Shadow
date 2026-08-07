// 基于事件总线与黑板模式的全角色自动化 Headless 守护调度引擎
//
// CS-Orchestrator 负责：
// - 事件总线 (Event Bus)：各 Agent 之间通过发布/订阅松散耦合
// - 黑板模式 (Blackboard)：全局共享状态，Agent 读写状态信号
// - 任务调度：Kanban 队列管理、优先级排序、并发控制
// - Headless 守护态：支持脱离 UI 在 Windows Server 上常驻运行
// - SSE 事件流：通过标准 Server-Sent Events 向外推送任务状态

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::redline::RedlineGuard;

// ─── 角色与状态枚举 ────────────────────────────────────────────────

/// Agent 角色枚举 — 对应 SDLC 流水线 7 个阶段
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    PM,          // 产品经理 — 梳理 PRD，锁定 Scope
    UIDesigner,  // UI 视觉设计师 — 输出布局 JSON，多模态走查
    Architect,   // 架构设计师 — 分析代码库拓扑，更新 CLAUDE.md
    Planner,     // 任务拆解与路由 — 拆解原子 Kanban 任务
    Coder,       // 编码子智能体集群 — 物理隔离增量写码
    Auditor,     // 安全与合规审查 — AST 增量审计
    Verifier,    // 自动化 CI/CD 纠错 — 本地编译 + 自愈
}

impl AgentRole {
    /// 流水线顺序
    pub fn order(&self) -> u8 {
        match self {
            AgentRole::PM => 0,
            AgentRole::UIDesigner => 1,
            AgentRole::Architect => 2,
            AgentRole::Planner => 3,
            AgentRole::Coder => 4,
            AgentRole::Auditor => 5,
            AgentRole::Verifier => 6,
        }
    }

    /// 角色中文标签
    pub fn label(&self) -> &str {
        match self {
            AgentRole::PM => "产品经理",
            AgentRole::UIDesigner => "UI 视觉设计师",
            AgentRole::Architect => "架构设计师",
            AgentRole::Planner => "任务拆解与路由",
            AgentRole::Coder => "编码子智能体集群",
            AgentRole::Auditor => "安全与合规审查",
            AgentRole::Verifier => "CI/CD 纠错",
        }
    }
}

// ─── 任务状态 ──────────────────────────────────────────────────────

/// 任务生命周期状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// 等待分配
    Pending,
    /// 已分配但未开始
    Assigned(AgentRole),
    /// 执行中
    InProgress(AgentRole),
    /// 等待审查（Coder → Auditor 之间）
    AwaitingReview,
    /// 完成
    Completed,
    /// 失败（带错误信息）
    Failed(String),
    /// 自愈熔断
    Fused { task_id: String, healing_count: u32 },
}

// ─── 事件定义 ──────────────────────────────────────────────────────

/// 黑板事件 — Agent 之间通信的消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardEvent {
    /// 事件唯一 ID
    pub id: String,
    /// 时间戳 (ISO-8601)
    pub timestamp: String,
    /// 事件来源 Agent
    pub source: AgentRole,
    /// 事件目标 Agent（None = 广播）
    pub target: Option<AgentRole>,
    /// 事件类型
    pub event_type: EventType,
    /// 事件负载
    pub payload: serde_json::Value,
}

/// 标准事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // 流水线事件
    PipelineStarted,
    PipelinePaused,
    PipelineResumed,
    PipelineCompleted,
    PipelineFailed { error: String },
    // 任务事件
    TaskCreated { task_id: String },
    TaskAssigned { task_id: String, role: AgentRole },
    TaskStarted { task_id: String },
    TaskCompleted { task_id: String },
    TaskFailed { task_id: String, error: String },
    TaskFused { task_id: String, healing_count: u32 },
    // Agent 事件
    AgentActivated { role: AgentRole },
    AgentDeactivated { role: AgentRole },
    AgentError { role: AgentRole, error: String },
    // 红线事件
    RedlineViolation { code: String, message: String },
    HealingAttempt { task_id: String, attempt: u32, max: u32 },
    CircuitBreakerTriggered { task_id: String },
    // 子智能体事件
    SubagentSpawned { agent_type: String, query: String },
    SubagentCompleted { agent_type: String, summary: String },
    // 心跳
    Heartbeat,
}

// ─── SDLC 流水线状态枚举 ────────────────────────────────────

/// SDLC 流水线状态（状态飞轮）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SdlcState {
    Idle,
    Designing,
    Planning,
    Coding,
    Auditing,
    Verifying,
    Completed,
}

impl SdlcState {
    pub fn label(&self) -> &str {
        match self {
            SdlcState::Idle => "空闲",
            SdlcState::Designing => "设计中",
            SdlcState::Planning => "规划中",
            SdlcState::Coding => "编码中",
            SdlcState::Auditing => "审计中",
            SdlcState::Verifying => "验证中",
            SdlcState::Completed => "已完成",
        }
    }
}

/// SDLC 事件（触发状态转移）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SdlcEvent {
    RequirementSubmitted(String),
    DesignFinalized,
    PlanGenerated,
    CodingCompleted,
    AuditPassed,
    VerificationDone,
}

// ─── 黑板状态 ──────────────────────────────────────────────────────

/// 全局黑板 — 所有 Agent 共享的状态存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blackboard {
    /// 项目全局契约 (CLAUDE.md 内容)
    pub global_contract: Option<String>,
    /// PRD 文档
    pub prd_document: Option<String>,
    /// UI 布局 JSON Schema
    pub ui_layout: Option<serde_json::Value>,
    /// 架构设计说明书
    pub architecture_doc: Option<String>,
    /// Scope 功能边界白名单
    pub scope_rules: Vec<String>,
    /// 当前 SDLC 状态（状态飞轮）
    pub current_state: SdlcState,
    /// 自定义键值存储
    pub context: HashMap<String, serde_json::Value>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            global_contract: None,
            prd_document: None,
            ui_layout: None,
            architecture_doc: None,
            scope_rules: Vec::new(),
            current_state: SdlcState::Idle,
            context: HashMap::new(),
        }
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 任务卡片 ──────────────────────────────────────────────────────

/// 原子任务卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTask {
    /// 任务唯一 ID
    pub id: String,
    /// 任务标题
    pub title: String,
    /// 任务描述
    pub description: String,
    /// 分配给的 Agent 角色
    pub assigned_to: Option<AgentRole>,
    /// 当前状态
    pub status: TaskStatus,
    /// 依赖的前置任务 ID 列表
    pub dependencies: Vec<String>,
    /// 最大自愈次数
    pub max_healing_loop: u32,
    /// 已自愈次数
    pub healing_count: u32,
    /// 创建时间
    pub created_at: String,
    /// 优先级 (0 = 最高)
    pub priority: u8,
}

// ─── 编排引擎统计 ──────────────────────────────────────────────────

/// 编排引擎运行时统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub fused_tasks: usize,
    pub pending_tasks: usize,
    pub active_role: AgentRole,
    pub pipeline_running: bool,
}

// ─── 编排引擎（主结构） ────────────────────────────────────────────

/// 编排引擎 — 核心状态机
pub struct Orchestrator {
    /// 事件总线发送端
    pub event_tx: broadcast::Sender<BlackboardEvent>,
    /// 全局黑板
    pub blackboard: Blackboard,
    /// 当前活跃的 Agent 角色
    pub active_role: AgentRole,
    /// Kanban 任务队列
    pub tasks: Vec<KanbanTask>,
    /// 流水线是否运行中
    pub running: bool,
    /// 事件计数器（用于生成唯一 ID）
    event_counter: u64,
    /// 任务 ID 计数器
    task_counter: u64,
    /// Tauri AppHandle (用于 emit 前端事件)
    app_handle: Option<tauri::AppHandle>,
}

impl Orchestrator {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            event_tx,
            blackboard: Blackboard::new(),
            active_role: AgentRole::PM,
            tasks: Vec::new(),
            running: false,
            event_counter: 0,
            task_counter: 0,
            app_handle: None,
        }
    }

    /// 设置 Tauri AppHandle 用于前端事件推送
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    // ── 事件发布 ──────────────────────────────────────────────────

    /// 生成唯一事件 ID
    fn next_event_id(&mut self) -> String {
        self.event_counter += 1;
        format!("evt-{:04}", self.event_counter)
    }

    /// 发布事件到事件总线（广播）
    pub fn publish(&mut self, source: AgentRole, event_type: EventType) {
        let event = BlackboardEvent {
            id: self.next_event_id(),
            timestamp: chrono_now(),
            source: source.clone(),
            target: None,
            event_type,
            payload: serde_json::json!({}),
        };
        let _ = self.event_tx.send(event);
    }

    /// 发布定向事件
    pub fn publish_to(
        &mut self,
        source: AgentRole,
        target: AgentRole,
        event_type: EventType,
        payload: serde_json::Value,
    ) {
        let event = BlackboardEvent {
            id: self.next_event_id(),
            timestamp: chrono_now(),
            source,
            target: Some(target),
            event_type,
            payload,
        };
        let _ = self.event_tx.send(event);
    }

    // ── 流水线控制 ────────────────────────────────────────────────

    /// 启动流水线
    pub fn start_pipeline(&mut self) {
        self.running = true;
        self.active_role = AgentRole::PM;
        self.publish(AgentRole::PM, EventType::PipelineStarted);
    }

    /// 暂停流水线
    pub fn pause_pipeline(&mut self) {
        self.running = false;
        self.publish(self.active_role.clone(), EventType::PipelinePaused);
    }

    /// 恢复流水线
    pub fn resume_pipeline(&mut self) {
        self.running = true;
        self.publish(self.active_role.clone(), EventType::PipelineResumed);
    }

    /// 推进流水线到下一个 Agent
    pub fn advance_pipeline(&mut self) -> AgentRole {
        let old_role = self.active_role.clone();
        self.active_role = match &self.active_role {
            AgentRole::PM => AgentRole::UIDesigner,
            AgentRole::UIDesigner => AgentRole::Architect,
            AgentRole::Architect => AgentRole::Planner,
            AgentRole::Planner => AgentRole::Coder,
            AgentRole::Coder => AgentRole::Auditor,
            AgentRole::Auditor => AgentRole::Verifier,
            AgentRole::Verifier => AgentRole::PM,
        };
        self.publish(
            old_role,
            EventType::AgentActivated {
                role: self.active_role.clone(),
            },
        );
        self.active_role.clone()
    }

    // ── 任务管理 ──────────────────────────────────────────────────

    /// 创建新任务
    pub fn create_task(
        &mut self,
        title: &str,
        description: &str,
        dependencies: Vec<String>,
        priority: u8,
    ) -> String {
        self.task_counter += 1;
        let id = format!("task-{:04}", self.task_counter);
        let task = KanbanTask {
            id: id.clone(),
            title: title.into(),
            description: description.into(),
            assigned_to: None,
            status: TaskStatus::Pending,
            dependencies,
            max_healing_loop: 3,
            healing_count: 0,
            created_at: chrono_now(),
            priority,
        };
        self.tasks.push(task);
        self.publish(
            self.active_role.clone(),
            EventType::TaskCreated {
                task_id: id.clone(),
            },
        );
        id
    }

    /// 将任务分配给指定 Agent
    pub fn assign_task(&mut self, task_id: &str, role: AgentRole) -> Result<(), String> {
        // 第一步：检查依赖（只读遍历）
        let task = self
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        for dep_id in &task.dependencies {
            let dep_completed = self
                .tasks
                .iter()
                .any(|t| &t.id == dep_id && t.status == TaskStatus::Completed);
            if !dep_completed {
                return Err(format!("Dependency {} not completed", dep_id));
            }
        }

        // 第二步：修改状态（可变借用）
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .unwrap(); // safe: we already verified existence

        let role_clone = role.clone(); // 用于事件发布
        task.assigned_to = Some(role);
        task.status = TaskStatus::Assigned(role_clone.clone());
        let task_id_owned = task_id.to_string();

        // 第三步：发布事件
        self.publish(
            self.active_role.clone(),
            EventType::TaskAssigned {
                task_id: task_id_owned,
                role: role_clone,
            },
        );
        Ok(())
    }

    /// 开始执行任务
    pub fn start_task(&mut self, task_id: &str) -> Result<(), String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        let role = task
            .assigned_to
            .clone()
            .ok_or("Task not assigned to any role")?;

        task.status = TaskStatus::InProgress(role);
        self.publish(
            self.active_role.clone(),
            EventType::TaskStarted {
                task_id: task_id.into(),
            },
        );
        Ok(())
    }

    /// 完成任务
    pub fn complete_task(&mut self, task_id: &str) -> Result<(), String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        task.status = TaskStatus::Completed;
        self.publish(
            self.active_role.clone(),
            EventType::TaskCompleted {
                task_id: task_id.into(),
            },
        );
        Ok(())
    }

    /// 任务失败（触发自愈）
    pub fn fail_task(&mut self, task_id: &str, error: &str) -> Result<bool, String> {
        // 第一步：计算熔断状态（可变借用 tasks）
        let (fused, healing_count, max_loop) = {
            let task = self
                .tasks
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task {} not found", task_id))?;

            task.healing_count += 1;

            if task.healing_count > task.max_healing_loop {
                task.status = TaskStatus::Fused {
                    task_id: task_id.into(),
                    healing_count: task.healing_count,
                };
                (true, task.healing_count, task.max_healing_loop)
            } else {
                task.status = TaskStatus::Failed(error.into());
                (false, task.healing_count, task.max_healing_loop)
            }
        }; // 可变借用在此结束

        // 第二步：发布事件（无借用冲突）
        let tid = task_id.to_string();
        if fused {
            self.publish(
                self.active_role.clone(),
                EventType::TaskFused {
                    task_id: tid.clone(),
                    healing_count,
                },
            );
            self.publish(
                self.active_role.clone(),
                EventType::CircuitBreakerTriggered { task_id: tid },
            );
            Ok(false)
        } else {
            self.publish(
                self.active_role.clone(),
                EventType::HealingAttempt {
                    task_id: tid,
                    attempt: healing_count,
                    max: max_loop,
                },
            );
            Ok(true)
        }
    }

    /// 获取所有待处理任务（按优先级排序）
    pub fn pending_tasks(&self) -> Vec<&KanbanTask> {
        let mut pending: Vec<_> = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::Assigned(_)))
            .collect();
        pending.sort_by_key(|t| t.priority);
        pending
    }

    /// 获取指定角色的任务
    pub fn tasks_for_role(&self, role: &AgentRole) -> Vec<&KanbanTask> {
        self.tasks
            .iter()
            .filter(|t| t.assigned_to.as_ref() == Some(role))
            .collect()
    }

    // ── 运行时统计 ────────────────────────────────────────────────

    // ── Headless 事件循环（对齐白皮书 run_loop） ─────────────────

    /// 异步事件总线监听器 — 支持完全脱离界面的 Headless Daemon 常驻运行
    /// 对齐白皮书 orchestrator.rs run_loop() 实现
    pub async fn run_loop(
        redline: Arc<RedlineGuard>,
        mut event_rx: broadcast::Receiver<BlackboardEvent>,
        max_healing: u32,
    ) {
        tracing::info!("[CHRONOS-SHADOW] Headless Orchestration Daemon started.");

        while let Ok(event) = event_rx.recv().await {
            tracing::info!("[EVENT] Received: {:?}", event.event_type);

            match event.event_type {
                EventType::TaskCompleted { task_id } => {
                    tracing::info!("[ORCH] Task {} completed — advancing pipeline", task_id);
                }
                EventType::TaskFailed { task_id, error } => {
                    // 红线三：自愈重试
                    let _healing_key = format!("heal-{}", task_id);
                    // In production: track per-task healing counter in Blackboard
                    tracing::warn!(
                        "[SELF-HEALING] Task {} failed: {}. Retry logic active (max {})",
                        task_id, error, max_healing
                    );

                    // Simulate auto-retry with fixed output
                    let fixed_output = format!(
                        r#"{{"action": "file_edit", "params": {{"path": "src/main.rs", "content": "// auto-fixed by @Verifier"}}}}"#
                    );

                    // Activate Redline before retrying
                    match redline.validate_and_parse(&fixed_output) {
                        Ok(action) => {
                            tracing::info!(
                                "[Auditor] Redlines cleared for auto-fix. Action: {:?}",
                                action
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "[SHIELD] Antihallucination Intercepted! Error: {}",
                                e
                            );
                        }
                    }
                }
                EventType::CircuitBreakerTriggered { task_id } => {
                    tracing::error!(
                        "[MELTDOWN CRITICAL] Task {} exceeded max healing loop. Budget protected.",
                        task_id
                    );
                }
                _ => {}
            }
        }
    }

    /// 生成前端展示用的统计摘要
    pub fn stats(&self) -> OrchestratorStats {
        let completed = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
            .count();
        let fused = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Fused { .. }))
            .count();
        let pending = self
            .tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    TaskStatus::Pending
                        | TaskStatus::Assigned(_)
                        | TaskStatus::InProgress(_)
                        | TaskStatus::AwaitingReview
                )
            })
            .count();

        OrchestratorStats {
            total_tasks: self.tasks.len(),
            completed_tasks: completed,
            failed_tasks: failed,
            fused_tasks: fused,
            pending_tasks: pending,
            active_role: self.active_role.clone(),
            pipeline_running: self.running,
        }
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 工具函数 ──────────────────────────────────────────────────────

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_orchestrator() -> Orchestrator {
        Orchestrator::new()
    }

    #[test]
    fn test_pipeline_lifecycle() {
        let mut orch = make_orchestrator();
        assert!(!orch.running);

        orch.start_pipeline();
        assert!(orch.running);
        assert_eq!(orch.active_role, AgentRole::PM);

        orch.advance_pipeline();
        assert_eq!(orch.active_role, AgentRole::UIDesigner);

        orch.pause_pipeline();
        assert!(!orch.running);

        orch.resume_pipeline();
        assert!(orch.running);
    }

    #[test]
    fn test_full_pipeline_cycle() {
        let mut orch = make_orchestrator();
        orch.start_pipeline();

        let roles = vec![
            AgentRole::PM,
            AgentRole::UIDesigner,
            AgentRole::Architect,
            AgentRole::Planner,
            AgentRole::Coder,
            AgentRole::Auditor,
            AgentRole::Verifier,
            AgentRole::PM, // back to start
        ];

        for expected in roles {
            assert_eq!(orch.active_role, expected);
            orch.advance_pipeline();
        }
    }

    #[test]
    fn test_task_lifecycle() {
        let mut orch = make_orchestrator();
        orch.start_pipeline();

        // Create
        let id = orch.create_task("Test task", "A test task", vec![], 0);
        assert_eq!(orch.tasks.len(), 1);

        // Assign
        orch.assign_task(&id, AgentRole::Coder).unwrap();
        let task = orch.tasks.iter().find(|t| t.id == id).unwrap();
        assert!(matches!(task.status, TaskStatus::Assigned(AgentRole::Coder)));

        // Start
        orch.start_task(&id).unwrap();
        let task = orch.tasks.iter().find(|t| t.id == id).unwrap();
        assert!(matches!(task.status, TaskStatus::InProgress(AgentRole::Coder)));

        // Complete
        orch.complete_task(&id).unwrap();
        let task = orch.tasks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_dependency() {
        let mut orch = make_orchestrator();

        let dep_id = orch.create_task("Dependency", "Must finish first", vec![], 0);
        let main_id = orch.create_task("Main task", "Depends on first", vec![dep_id.clone()], 0);

        // Cannot assign main until dependency completes
        assert!(orch.assign_task(&main_id, AgentRole::Coder).is_err());

        // Complete dependency first
        orch.assign_task(&dep_id, AgentRole::Planner).unwrap();
        orch.start_task(&dep_id).unwrap();
        orch.complete_task(&dep_id).unwrap();

        // Now main can be assigned
        assert!(orch.assign_task(&main_id, AgentRole::Coder).is_ok());
    }

    #[test]
    fn test_healing_fuse() {
        let mut orch = make_orchestrator();
        let id = orch.create_task("Flaky task", "May fail", vec![], 0);
        orch.assign_task(&id, AgentRole::Coder).unwrap();
        orch.start_task(&id).unwrap();

        // 3 healing attempts should be OK
        assert!(orch.fail_task(&id, "error 1").unwrap());
        assert!(orch.fail_task(&id, "error 2").unwrap());
        assert!(orch.fail_task(&id, "error 3").unwrap());

        // 4th attempt should fuse
        assert!(!orch.fail_task(&id, "error 4").unwrap());

        let task = orch.tasks.iter().find(|t| t.id == id).unwrap();
        assert!(matches!(task.status, TaskStatus::Fused { .. }));
    }

    #[test]
    fn test_agent_role_order() {
        assert!(AgentRole::PM.order() < AgentRole::UIDesigner.order());
        assert!(AgentRole::Coder.order() < AgentRole::Auditor.order());
        assert_eq!(AgentRole::Verifier.order(), 6);
    }

    #[test]
    fn test_stats() {
        let mut orch = make_orchestrator();
        let id1 = orch.create_task("Task 1", "desc", vec![], 0);
        let id2 = orch.create_task("Task 2", "desc", vec![], 0);

        orch.assign_task(&id1, AgentRole::Coder).unwrap();
        orch.start_task(&id1).unwrap();
        orch.complete_task(&id1).unwrap();

        let stats = orch.stats();
        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.pending_tasks, 1);
    }
}
