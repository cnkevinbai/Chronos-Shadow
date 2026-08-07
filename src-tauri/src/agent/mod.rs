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

// ─── WorkBuddy 泛办公协同底座 ──────────────────────────────────────
pub mod buddy_scan;
pub mod context_glue;

// ─── 远程服务器研发协同 ──────────────────────────────────────────
pub mod remote_proxy;
pub mod remote_cluster;

// ─── 财务审计引擎 ────────────────────────────────────────────────
pub mod billing;
pub mod billing_engine;

// ─── 零 Token 技能检测 + 集群分配引擎 ───────────────────────
pub mod detector;

// ─── 金融级安全保险箱 ──────────────────────────────────────
pub mod security_vault;

// ─── 会话持久化引擎 ──────────────────────────────────────────
pub mod session_db;
