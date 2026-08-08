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

// WorkBuddy
pub mod buddy_scan;
pub mod context_glue;

// Remote
pub mod remote_proxy;
pub mod remote_cluster;

// Billing
pub mod billing;
pub mod billing_engine;

// Detector
pub mod detector;

// Agent Scheduling
pub mod scheduling_engine;

// Vault
pub mod security_vault;

// Session
pub mod session_db;
