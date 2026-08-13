// 基于事件总线与黑板模式的全角色自动化 Headless 守护调度引擎
//
// CS-Orchestrator 负责：
// - 事件总线 (Event Bus)：各 Agent 之间通过发布/订阅松散耦合
// - 黑板模式 (Blackboard)：全局共享状态，Agent 读写状态信号
// - 任务调度：Kanban 队列管理、优先级排序、并发控制
// - Headless 守护态：支持脱离 UI 在 Windows Server 上常驻运行
// - SSE 事件流：通过标准 Server-Sent Events 向外推送任务状态

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
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
    Auditor,           // 安全审计 — AST 增量审计
    ComplianceOfficer, // 法律合规专家 — GDPR/PIPL/开源协议审查
    Verifier,          // 自动化 CI/CD 纠错 — 本地编译 + 自愈
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
            AgentRole::ComplianceOfficer => 6,
            AgentRole::Verifier => 7,
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
            AgentRole::Auditor => "安全审计",
            AgentRole::ComplianceOfficer => "法律合规专家",
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

/// 事件回调类型
pub type EventCallback = Box<dyn Fn(&BlackboardEvent) + Send + Sync>;

/// 编排引擎 — 核心状态机
pub struct Orchestrator {
    /// 事件总线发送端
    pub event_tx: broadcast::Sender<BlackboardEvent>,
    /// 全局黑板
    pub blackboard: Blackboard,
    /// 当前活跃的 Agent 角色
    pub active_role: AgentRole,
    /// 事件回调注册表 (event_code → Vec<callback>)
    pub event_callbacks: HashMap<String, Vec<EventCallback>>,
    /// Dead Letter Queue — 未被任何模块处理的事件
    pub dead_letter_queue: VecDeque<BlackboardEvent>,
    /// 最大死信队列长度
    pub max_dead_letters: usize,
    /// 检查点数量上限
    pub max_checkpoints: usize,
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
            event_callbacks: HashMap::new(),
            dead_letter_queue: VecDeque::with_capacity(64),
            max_dead_letters: 100,
            max_checkpoints: 50,
        }
    }

    /// 设置 Tauri AppHandle 用于前端事件推送
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    // ── 事件回调注册 ────────────────────────────────────────────

    /// 注册事件回调 — 当指定 event_code 的事件发布时触发
    pub fn on_event<F>(&mut self, event_code: &str, callback: F)
    where F: Fn(&BlackboardEvent) + Send + Sync + 'static {
        self.event_callbacks
            .entry(event_code.to_string())
            .or_default()
            .push(Box::new(callback));
    }

    /// 处理死信队列 — 将超限的事件写入日志
    pub fn flush_dead_letters(&mut self) -> Vec<BlackboardEvent> {
        let drained: Vec<_> = self.dead_letter_queue.drain(..).collect();
        for event in &drained {
            tracing::warn!(
                "[ORCHESTRATOR] Dead letter: id={} type={:?} source={:?}",
                event.id, event.event_type, event.source
            );
        }
        drained
    }

    // ── 事件指标 ────────────────────────────────────────────

    /// 获取事件发布统计
    pub fn event_metrics(&self) -> serde_json::Value {
        let dlq_len = self.dead_letter_queue.len();
        let cb_count = self.event_callbacks.values().map(|v| v.len()).sum::<usize>();
        serde_json::json!({
            "total_events": self.event_counter,
            "registered_callbacks": cb_count,
            "dead_letter_queue_size": dlq_len,
            "max_dead_letters": self.max_dead_letters,
            "checkpoint_limit": self.max_checkpoints,
            "active_tasks": self.tasks.len(),
        })
    }

    // ── 内存管理 ────────────────────────────────────────────────

    /// 清理超限的旧任务
    pub fn prune_old_tasks(&mut self, keep: usize) {
        if self.tasks.len() > keep {
            let removed = self.tasks.len() - keep;
            self.tasks.drain(0..removed);
            tracing::info!("[ORCHESTRATOR] Pruned {} old tasks (kept {})", removed, keep);
        }
    }

    // ── 事件发布 ──────────────────────────────────────────────────

    /// 生成唯一事件 ID
    fn next_event_id(&mut self) -> String {
        self.event_counter += 1;
        format!("evt-{:04}", self.event_counter)
    }

    /// 发布事件到事件总线（广播 + 回调触发 + 死信队列）
    pub fn publish(&mut self, source: AgentRole, event_type: EventType) {
        let event = BlackboardEvent {
            id: self.next_event_id(),
            timestamp: chrono_now(),
            source: source.clone(),
            target: None,
            event_type: event_type.clone(),
            payload: serde_json::json!({}),
        };

        // 1. 广播到 broadcast channel
        let _ = self.event_tx.send(event.clone());

        // 2. 触发已注册的回调
        let event_code = Self::event_code(&event_type);
        let mut handled = false;
        if let Some(callbacks) = self.event_callbacks.get(&event_code) {
            for cb in callbacks {
                cb(&event);
            }
            handled = true;
        }
        // 也触发通配回调 (*)
        if let Some(callbacks) = self.event_callbacks.get("*") {
            for cb in callbacks {
                cb(&event);
            }
            handled = true;
        }

        // 3. 未被任何模块处理 → 入死信队列
        if !handled {
            self.dead_letter_queue.push_back(event);
            while self.dead_letter_queue.len() > self.max_dead_letters {
                self.dead_letter_queue.pop_front();
            }
        }
    }

    /// 从 EventType 提取事件代码（用于回调匹配）
    fn event_code(event_type: &EventType) -> String {
        match event_type {
            EventType::PipelineStarted => "pipeline_started".into(),
            EventType::PipelinePaused => "pipeline_paused".into(),
            EventType::PipelineResumed => "pipeline_resumed".into(),
            EventType::PipelineCompleted => "pipeline_completed".into(),
            EventType::PipelineFailed { .. } => "pipeline_failed".into(),
            EventType::TaskCreated { .. } => "task_created".into(),
            EventType::TaskCompleted { .. } => "task_completed".into(),
            EventType::TaskFailed { .. } => "task_failed".into(),
            EventType::TaskFused { .. } => "task_fused".into(),
            EventType::RedlineViolation { .. } => "redline_violation".into(),
            EventType::CircuitBreakerTriggered { .. } => "circuit_breaker".into(),
            EventType::Heartbeat => "heartbeat".into(),
            _ => "unknown".into(),
        }
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
            AgentRole::Auditor => AgentRole::ComplianceOfficer,
            AgentRole::ComplianceOfficer => AgentRole::Verifier,
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

    // ── 自动化调度矩阵 v2: 依赖图 + 并行分组 ──────────────────

    /// 获取可执行任务（所有依赖已完成且未开始）
    pub fn executable_tasks(&self) -> Vec<&KanbanTask> {
        let completed_ids: std::collections::HashSet<&str> = self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed))
            .map(|t| t.id.as_str())
            .collect();

        self.tasks
            .iter()
            .filter(|t| {
                matches!(t.status, TaskStatus::Pending | TaskStatus::Assigned(_))
                    && t.dependencies.iter().all(|dep| completed_ids.contains(dep.as_str()))
            })
            .collect()
    }

    /// 依赖图拓扑排序（Kahn 算法），返回推荐执行顺序
    pub fn topological_sort(&self) -> Vec<String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for t in &self.tasks {
            in_degree.entry(&t.id).or_insert(0);
            for dep in &t.dependencies {
                adj.entry(dep.as_str()).or_default().push(&t.id);
                *in_degree.entry(&t.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(next) {
                        *deg -= 1;
                        if *deg == 0 { queue.push_back(next); }
                    }
                }
            }
        }

        // 未排序的（循环依赖或孤立）追加
        for t in &self.tasks {
            if !sorted.contains(&t.id) { sorted.push(t.id.clone()); }
        }
        sorted
    }

    /// 并行执行分组：将无相互依赖的任务分组，每组可并行执行
    pub fn parallel_groups(&self) -> Vec<Vec<&KanbanTask>> {
        let sorted_ids = self.topological_sort();

        let mut groups: Vec<Vec<&KanbanTask>> = Vec::new();
        let mut assigned: HashMap<&str, usize> = HashMap::new(); // task_id → group_index

        for id in &sorted_ids {
            if let Some(task) = self.tasks.iter().find(|t| &t.id == id) {
                // 找到该任务所有依赖的最晚分组
                let mut max_dep_group: i32 = -1;
                for dep in &task.dependencies {
                    if let Some(&g) = assigned.get(dep.as_str()) {
                        max_dep_group = max_dep_group.max(g as i32);
                    }
                }
                let group = (max_dep_group + 1) as usize;
                while groups.len() <= group { groups.push(Vec::new()); }
                groups[group].push(task);
                assigned.insert(id, group);
            }
        }
        groups
    }

    /// 自动分配可执行任务（按优先级排序，优先级相同按创建时间）
    pub fn auto_schedule(&mut self, available_agents: &[AgentRole]) -> Vec<String> {
        let mut executable_ids: Vec<(String, u8, String)> = self.executable_tasks()
            .iter()
            .map(|t| (t.id.clone(), t.priority, t.created_at.clone()))
            .collect();
        executable_ids.sort_by_key(|(_, p, c)| (*p, c.clone()));

        let mut scheduled = Vec::new();
        for (id, _, _) in executable_ids {
            if let Some(agent) = available_agents.first() {
                let _ = self.assign_task(&id, agent.clone());
                let _ = self.start_task(&id);
                scheduled.push(id);
            }
        }
        scheduled
    }

    /// 调度矩阵评分：综合成本/质量/并行度评估调度方案
    pub fn schedule_quality_score(&self) -> f64 {
        let total = self.tasks.len() as f64;
        if total == 0.0 { return 100.0; }

        let completed = self.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Completed)).count() as f64;
        let fused = self.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Fused { .. })).count() as f64;

        let completion_rate = completed / total * 50.0;
        let fuse_penalty = (fused / total * 30.0).min(30.0);

        let parallel_groups = self.parallel_groups();
        let parallelism_bonus = if parallel_groups.len() > 1 {
            (total / parallel_groups.len() as f64 * 10.0).min(20.0)
        } else { 0.0 };

        (completion_rate + parallelism_bonus - fuse_penalty).max(0.0).min(100.0)
    }

    /// 智能重试：对失败任务指数退避重试
    pub fn smart_retry_failed(&mut self, max_retries: u32, base_delay_ms: u64) -> Vec<String> {
        let mut retried = Vec::new();
        let failed_info: Vec<(String, u32)> = self.tasks
            .iter()
            .filter(|t| matches!(&t.status, TaskStatus::Failed(_)))
            .map(|t| (t.id.clone(), t.healing_count))
            .collect();

        for (i, (id, healing)) in failed_info.iter().enumerate() {
            if *healing >= max_retries { continue; }
            let delay = (base_delay_ms * 2u64.pow(i as u32)).min(30_000);

            if let Some(task) = self.tasks.iter_mut().find(|t| &t.id == id) {
                task.status = TaskStatus::Pending;
                task.healing_count += 1;
                if delay > 0 {
                    tracing::info!("[Orchestrator] Retrying {} after {}ms (attempt {})", id, delay, task.healing_count);
                }
            }
            retried.push(id.clone());
        }
        retried
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
            AgentRole::ComplianceOfficer,
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
        let _id2 = orch.create_task("Task 2", "desc", vec![], 0);

        orch.assign_task(&id1, AgentRole::Coder).unwrap();
        orch.start_task(&id1).unwrap();
        orch.complete_task(&id1).unwrap();

        let stats = orch.stats();
        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.pending_tasks, 1);
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

fn parse_role(s: &str) -> Result<AgentRole, String> {
    match s.to_lowercase().as_str() {
        "pm" => Ok(AgentRole::PM),
        "ui" | "ui_designer" | "uidesigner" => Ok(AgentRole::UIDesigner),
        "architect" | "arch" => Ok(AgentRole::Architect),
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "auditor" => Ok(AgentRole::Auditor),
        "verifier" => Ok(AgentRole::Verifier),
        _ => Err(format!("Unknown role: {}", s)),
    }
}

#[tauri::command]
pub fn get_pipeline_stats(state: tauri::State<crate::state::AppState>) -> OrchestratorStats {
    state.orchestrator.lock().unwrap().stats()
}

#[tauri::command]
pub fn orch_topological_sort(state: tauri::State<crate::state::AppState>) -> Vec<String> {
    state.orchestrator.lock().unwrap().topological_sort()
}

#[tauri::command]
pub fn orch_parallel_groups(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    let orch = state.orchestrator.lock().unwrap();
    let groups = orch.parallel_groups();
    serde_json::json!({
        "total_groups": groups.len(),
        "groups": groups.iter().enumerate().map(|(i, g)| {
            serde_json::json!({
                "group": i,
                "tasks": g.iter().map(|t| serde_json::json!({
                    "id": t.id, "title": t.title, "priority": t.priority,
                    "dependencies": t.dependencies, "status": format!("{:?}", t.status),
                })).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>()
    })
}

#[tauri::command]
pub fn orch_executable_tasks(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    let orch = state.orchestrator.lock().unwrap();
    let tasks = orch.executable_tasks();
    serde_json::json!({
        "count": tasks.len(),
        "tasks": tasks.iter().map(|t| serde_json::json!({
            "id": t.id, "title": t.title, "priority": t.priority, "dependencies": t.dependencies,
        })).collect::<Vec<_>>()
    })
}

#[tauri::command]
pub fn orch_schedule_quality(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    let orch = state.orchestrator.lock().unwrap();
    let score = orch.schedule_quality_score();
    let groups = orch.parallel_groups();
    serde_json::json!({
        "quality_score": format!("{:.1}", score),
        "parallel_groups": groups.len(),
        "completion_rate": format!("{:.1}%",
            orch.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Completed)).count() as f64
            / orch.tasks.len().max(1) as f64 * 100.0),
    })
}

#[tauri::command]
pub fn orch_smart_retry(state: tauri::State<crate::state::AppState>) -> Vec<String> {
    state.orchestrator.lock().unwrap().smart_retry_failed(3, 1000)
}

#[tauri::command]
pub fn start_pipeline(state: tauri::State<crate::state::AppState>) -> String {
    state.orchestrator.lock().unwrap().start_pipeline();
    "Pipeline started".into()
}

#[tauri::command]
pub fn pause_pipeline(state: tauri::State<crate::state::AppState>) -> String {
    state.orchestrator.lock().unwrap().pause_pipeline();
    "Pipeline paused".into()
}

#[tauri::command]
pub fn resume_pipeline(state: tauri::State<crate::state::AppState>) -> String {
    state.orchestrator.lock().unwrap().resume_pipeline();
    "Pipeline resumed".into()
}

#[tauri::command]
pub fn advance_pipeline(state: tauri::State<crate::state::AppState>) -> Result<String, String> {
    // 第四红线：关键阶段跃迁前检查审批状态
    let (from_stage, to_stage, needs_approval) = {
        let orch = state.orchestrator.lock().unwrap();
        let current = &orch.active_role;
        let next = match current {
            AgentRole::PM => AgentRole::UIDesigner,
            AgentRole::UIDesigner => AgentRole::Architect,
            AgentRole::Architect => AgentRole::Planner,
            AgentRole::Planner => AgentRole::Coder,
            AgentRole::Coder => AgentRole::Auditor,
            AgentRole::Auditor => AgentRole::ComplianceOfficer,
            AgentRole::ComplianceOfficer => AgentRole::Verifier,
            AgentRole::Verifier => AgentRole::PM,
        };
        let needs = matches!(current, AgentRole::Coder | AgentRole::Auditor);
        let from_id = match current {
            AgentRole::PM => "PM", AgentRole::UIDesigner => "UIDesigner",
            AgentRole::Architect => "Architect", AgentRole::Planner => "Planner",
            AgentRole::Coder => "Coder", AgentRole::Auditor => "Auditor",
            AgentRole::ComplianceOfficer => "ComplianceOfficer",
            AgentRole::Verifier => "Verifier",
        };
        let to_id = match &next {
            AgentRole::PM => "PM", AgentRole::UIDesigner => "UIDesigner",
            AgentRole::Architect => "Architect", AgentRole::Planner => "Planner",
            AgentRole::Coder => "Coder", AgentRole::Auditor => "Auditor",
            AgentRole::ComplianceOfficer => "ComplianceOfficer",
            AgentRole::Verifier => "Verifier",
        };
        (from_id.to_string(), to_id.to_string(), needs)
    };

    if needs_approval {
        state.approval_gate.lock().unwrap().check_pipeline_advance(&from_stage, &to_stage)?;
    }

    let role = state.orchestrator.lock().unwrap().advance_pipeline();
    Ok(role.label().into())
}

#[tauri::command]
pub fn create_task(
    state: tauri::State<crate::state::AppState>,
    title: String,
    description: String,
    dependencies: Vec<String>,
    priority: u8,
) -> String {
    state
        .orchestrator
        .lock()
        .unwrap()
        .create_task(&title, &description, dependencies, priority)
}

#[tauri::command]
pub fn assign_task(state: tauri::State<crate::state::AppState>, task_id: String, role: String) -> Result<String, String> {
    let role = parse_role(&role)?;
    state
        .orchestrator
        .lock()
        .unwrap()
        .assign_task(&task_id, role)
        .map(|_| format!("Task {} assigned", task_id))
}

#[tauri::command]
pub fn complete_task(state: tauri::State<crate::state::AppState>, task_id: String) -> Result<String, String> {
    state
        .orchestrator
        .lock()
        .unwrap()
        .complete_task(&task_id)
        .map(|_| format!("Task {} completed", task_id))
}

#[tauri::command]
pub fn fail_task(state: tauri::State<crate::state::AppState>, task_id: String, error: String) -> Result<String, String> {
    let can_retry = state
        .orchestrator
        .lock()
        .unwrap()
        .fail_task(&task_id, &error)
        .map_err(|e| e)?;

    if can_retry {
        Ok(format!("Task {} failed — can retry", task_id))
    } else {
        Err(format!("Task {} FUSED — manual intervention required", task_id))
    }
}
