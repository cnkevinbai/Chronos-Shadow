// Chronos-Shadow IPC 类型定义
// 与 Rust 端 src-tauri/src/agent/*.rs 中的类型严格对应

// ─── Redline ───────────────────────────────────────────────────────

export interface RedlineStatus {
  schema_active: boolean;
  schema_last_check: string | null;
  sandbox_active: boolean;
  sandbox_root: string;
  blocked_paths: number;
  healing_enabled: boolean;
  max_loop: number;
  current_loop: number;
  fused: boolean;
}

// ─── Orchestrator ──────────────────────────────────────────────────

export type AgentRole =
  | "PM"
  | "UIDesigner"
  | "Architect"
  | "Planner"
  | "Coder"
  | "Auditor"
  | "Verifier";

export interface OrchestratorStats {
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  fused_tasks: number;
  pending_tasks: number;
  active_role: AgentRole;
  pipeline_running: boolean;
}

// ─── Vision (估算) ────────────────────────────────────────────────

export interface VisionSavings {
  blocked_requests: number;
  tokens_saved: number;
  estimated_cost_saved: number;
}

// ─── Sandbox ───────────────────────────────────────────────────────

export interface MountPoint {
  name: string;
  source: string;
  target: string;
  read_only: boolean;
  active: boolean;
}

export interface SystemSnapshot {
  id: string;
  timestamp: string;
  label: string;
  files_changed: number;
  snapshot_type: "Auto" | "Manual";
}

// ─── WorkBuddy: Buddy Scan ────────────────────────────────────────

export interface ComponentLocation {
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  confidence: number;
  component_type: string;
}

export interface DpiCorrection {
  original: [number, number];
  corrected: [number, number];
  scale_factor: number;
  dpi_mode: string;
  offset: [number, number];
}

export interface TextVerification {
  expected_text: string;
  detected_text: string;
  similarity: number;
  passed: boolean;
}

export interface BuddyScanStats {
  total_scans: number;
  corrections_applied: number;
  text_verifications: number;
  verification_pass_rate: number;
  hallucination_prevented: number;
  vlm_tokens_saved: number;
  estimated_cost_saved: number;
  active: boolean;
}

export interface BuddyScanResult {
  safe_to_click: boolean;
  location: ComponentLocation | null;
  correction: DpiCorrection | null;
  verification: TextVerification | null;
  skip_reason: string | null;
}

// ─── WorkBuddy: Context Glue ──────────────────────────────────────

export type AppType = "Browser" | "Office" | "Erp" | "Im" | "Database" | "Terminal" | { Custom: string };

export type AppStatus = "Running" | "Paused" | "Stopped" | { Error: string };

export type DataDirection = "OneWay" | "TwoWay";

export type StreamStatus =
  | { Streaming: number }
  | "Idle"
  | "Paused"
  | { Error: string };

export interface AppBinding {
  id: string;
  source_app: string;
  target_app: string;
  mapping_rule: string;
  active: boolean;
  direction: DataDirection;
  stream_status: StreamStatus;
}

export interface AppNode {
  id: string;
  name: string;
  app_type: AppType;
  hwnd: number;
  process_name: string;
  authorized: boolean;
  status: AppStatus;
}

// ─── Session Persistence (Chunked V2) ────────────────────────────

export interface ChatMessageEntity {
  id: string;
  sender: string;
  model: string;
  content: string;
  thinking?: string | null;
  cost_tokens: number;
  timestamp: string;
  /** SHA256 链式累积缓存特征哈希 —— 端侧校验 DeepSeek Context Caching 对齐状态 */
  caching_marker_hash: string;
}

export interface SessionMetaManifest {
  session_id: string;
  title: string;
  bound_project: string;
  last_updated: string;
  total_messages_count: number;
  /** 单会话维度累计 Token 折算成本 (CNY)，供企业财务审计 */
  total_accumulated_cost: number;
  /** 最后一条用户消息预览（前 40 字符） */
  last_message_preview?: string;
}

export interface ChatSessionPayload {
  meta: SessionMetaManifest;
  messages: ChatMessageEntity[];
}

// ─── WorkBuddy: Context Glue ──────────────────────────────────────

export interface ContextGlueStats {
  apps_bound: number;
  active_bindings: number;
  bytes_transferred: number;
  tokens_saved: number;
  estimated_cost_saved: number;
  active: boolean;
  clipboard_managed: boolean;
}

// ─── Approval Gate (第四红线) ─────────────────────────────────────

export type ApprovalStatus = "Pending" | "Approved" | "Rejected" | "Expired" | "AutoApproved" | "AuditorPreScreened";

export interface ApprovalRule {
  id: string;
  name: string;
  action_type: string;
  enabled: boolean;
  auto_approve_below_risk: number;
  timeout_secs: number;
  reject_on_timeout: boolean;
  approver: string;
  project_scope: string | null;
  enable_auditor_prescreen: boolean;
  auditor_risk_reduction: number;
}

export interface ApprovalRequest {
  id: string;
  action_type: string;
  target_id: string;
  description: string;
  risk_level: number;
  status: string;
  submitted_at: string;
  decided_at: string | null;
  decided_by: string | null;
  decision_comment: string | null;
  metadata: string;
  project: string | null;
  estimated_cost: number | null;
  auditor_prescreen: {
    passed: boolean;
    findings_count: number;
    critical_count: number;
    summary: string;
  } | null;
  expires_at: string | null;
}

export interface ApprovalSuggestion {
  rule_name: string;
  action_type: string;
  current_threshold: number;
  suggested_threshold: number;
  reason: string;
  confidence: number;
}

export interface OperationRiskProfile {
  impact_scope: number;
  reversibility: number;
  cost_impact: number;
  compliance_required: number;
  composite_score: number;
  label: string;
}

// ─── Worktree ──────────────────────────────────────────────────────

export type WorktreeState =
  | "Created"
  | { Active: { task_id: string; agent_id: string } }
  | { Completed: { task_id: string } }
  | "Merged"
  | { Error: string };

export interface WorktreeInstance {
  id: string;
  path: string;
  task_id: string | null;
  state: WorktreeState;
  branch: string;
  created_at: string;
}

export interface MergeResult {
  success: boolean;
  conflicts: string[];
  merged_files: string[];
  error: string | null;
}

export interface WorktreeStats {
  total: number;
  active: number;
  completed: number;
  merged: number;
  errors: number;
}

// ─── Web Intelligence ───────────────────────────────────────────────

export interface WebSearchResult {
  title: string;
  url: string;
  snippet: string;
  source: string;
  relevance_score: number;
}

export interface WebFetchResult {
  success: boolean;
  url: string;
  title: string;
  content: string;
  content_length: number;
  distilled: boolean;
  distilled_summary: string | null;
  key_points: string[];
  error: string | null;
}

export interface ResearchReport {
  topic: string;
  summary: string;
  key_findings: string[];
  sources: WebSearchResult[];
  confidence: number;
  timestamp: string;
  recommendations: string[];
}

export interface WebAuditEntry {
  timestamp: string;
  request_type: string;
  target: string;
  result: string;
  bytes_received: number;
  duration_ms: number;
  domain_allowed: boolean;
}

export interface WebIntelStats {
  total_searches: number;
  total_fetches: number;
  total_research: number;
  bytes_downloaded: number;
  domains_whitelisted: number;
  requests_blocked: number;
  estimated_cost_saved: number;
  total_distilled: number;
  total_bytes_saved: number;
  avg_compression_ratio: number;
  cache_hit_rate: number;
  unified_cache_hits: number;
  unified_cache_misses: number;
  api_calls_saved: number;
}
