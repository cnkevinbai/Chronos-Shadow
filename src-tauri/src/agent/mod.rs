// 多智能体角色定义与事件驱动状态机中枢

pub mod orchestrator;
pub mod router;
pub mod sandbox;
pub mod skill_engine;
pub mod mcp_client;
pub mod api_client;
pub mod auditor;
pub mod evolving;
pub mod lan_discovery;
pub mod redline;
pub mod shadow;
pub mod subagents;
pub mod win_hooks;
pub mod worktree;
pub mod agent_evolution;

// ─── WorkBuddy 泛办公协同底座 ──────────────────────────────────────
pub mod buddy_scan;
pub mod context_glue;
pub mod workbuddy_engine;

// ─── 远程服务器研发协同 ──────────────────────────────────────────
pub mod remote_proxy;
pub mod remote_cluster;

// ─── 财务审计引擎 ────────────────────────────────────────────────
pub mod billing;
pub mod billing_engine;

// ─── 零 Token 技能检测 + 集群分配引擎 ───────────────────────
pub mod detector;

// ─── Agent 调度 + 技能匹配引擎 ──────────────────────────────
pub mod scheduling_engine;

// ─── 防幻觉深度检测引擎 ────────────────────────────────────
pub mod hallucination_guard;

// ─── 金融级安全保险箱 ──────────────────────────────────────
pub mod security_vault;

// ─── 第四红线：人类审批门禁 ──────────────────────────────────
pub mod approval_gate;

// ─── 输入验证与完整性保护 ──────────────────────────────────
pub mod input_guard;

// ─── 端侧科学化分析引擎 ────────────────────────────────────
pub mod local_analytics;

// ─── 统一持久化状态管理器 ────────────────────────────────────
pub mod state_manager;

// ─── 会话持久化引擎 ──────────────────────────────────────────
pub mod session_db;
