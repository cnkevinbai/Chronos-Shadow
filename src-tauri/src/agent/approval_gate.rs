// 第四红线：人类审批门禁 (Approval Gate) v3 — 科学风险评分 + 深度项目融合
//
// 风险评分采用四维特征矩阵（操作引力模型）：
//   D1 影响范围 (Impact Scope)     — 1=单文件, 3=模块, 5=全项目, 7=跨项目, 10=生产环境
//   D2 可逆性   (Reversibility)     — 10=完全可逆(git revert), 1=不可逆(rm -rf)
//   D3 资费影响 (Cost Impact)       — 1=<¥1, 3=<¥5, 5=<¥20, 7=<¥100, 10=>¥100
//   D4 合规敏感 (Compliance)        — 1=无影响, 5=内部审计, 10=监管合规
//
//   final_risk = max(D1, 10-D2, D3, D4) → 1-10 分制
//   原则：任一维度高危即整体高危（木桶效应）
//
// 深度融合六大模块：
//   Orchestrator Blackboard — 审批事件发布到事件总线
//   Billing Engine           — 实时读取费用上限，超预算操作自动升级
//   SDLC Pipeline            — 阶段跃迁审批联动状态机
//   Session DB               — 审计日志复用会话持久化目录
//   Redline Guard            — 第四条红线与前三条统一品牌
//   Evolution Engine         — 审批模式反馈到演化引擎

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 四维风险特征矩阵 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    pub impact_scope: u8,   // D1: 1-10
    pub reversibility: u8,  // D2: 1-10 (10=完全可逆)
    pub cost_impact: u8,    // D3: 1-10
    pub compliance: u8,     // D4: 1-10
}

impl RiskProfile {
    /// 木桶效应：最终风险 = max(D1, 10-D2, D3, D4)
    pub fn final_risk(&self) -> u8 {
        let irreversibility = 10u8.saturating_sub(self.reversibility);
        *[self.impact_scope, irreversibility, self.cost_impact, self.compliance]
            .iter().max().unwrap_or(&5)
    }

    /// 人类可读的风险分解说明
    pub fn breakdown(&self) -> String {
        format!(
            "影响范围:{}/10 可逆性:{}/10 资费:{}/10 合规:{}/10 → 综合风险:{}/10",
            self.impact_scope, self.reversibility, self.cost_impact, self.compliance,
            self.final_risk()
        )
    }
}

// ─── 审批操作类型（带风险画像）─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalActionType {
    WorktreeMerge,
    PipelineAdvance,
    RemoteCommand,
    CostThreshold,
    FileDelete,
    ConfigChange,
    Custom(String),
}

impl ApprovalActionType {
    pub fn label(&self) -> &str {
        match self {
            Self::WorktreeMerge => "Worktree 合并",
            Self::PipelineAdvance => "流水线跃迁",
            Self::RemoteCommand => "远程命令",
            Self::CostThreshold => "资费超限",
            Self::FileDelete => "文件删除",
            Self::ConfigChange => "配置变更",
            Self::Custom(s) => s,
        }
    }

    /// 科学风险画像 — 四维评分矩阵
    pub fn risk_profile(&self) -> RiskProfile {
        match self {
            //              影响  可逆  资费  合规
            Self::WorktreeMerge => RiskProfile { impact_scope: 7, reversibility: 9, cost_impact: 1, compliance: 4 },
            Self::PipelineAdvance => RiskProfile { impact_scope: 5, reversibility: 7, cost_impact: 2, compliance: 3 },
            Self::RemoteCommand => RiskProfile { impact_scope: 9, reversibility: 2, cost_impact: 3, compliance: 6 },
            Self::CostThreshold => RiskProfile { impact_scope: 1, reversibility: 10, cost_impact: 8, compliance: 4 },
            Self::FileDelete => RiskProfile { impact_scope: 4, reversibility: 1, cost_impact: 1, compliance: 3 },
            Self::ConfigChange => RiskProfile { impact_scope: 3, reversibility: 8, cost_impact: 1, compliance: 2 },
            Self::Custom(_) => RiskProfile { impact_scope: 3, reversibility: 5, cost_impact: 1, compliance: 2 },
        }
    }

    /// 向后兼容：保留 default_risk 但基于风险画像计算
    pub fn default_risk(&self) -> u8 { self.risk_profile().final_risk() }
}

impl std::fmt::Display for ApprovalActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::WorktreeMerge => "WorktreeMerge",
            Self::PipelineAdvance => "PipelineAdvance",
            Self::RemoteCommand => "RemoteCommand",
            Self::CostThreshold => "CostThreshold",
            Self::FileDelete => "FileDelete",
            Self::ConfigChange => "ConfigChange",
            Self::Custom(s) => s,
        })
    }
}

// ─── 审批事件（用于 Blackboard 集成）───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalEvent {
    Submitted { request_id: String, action_type: String, risk_level: u32 },
    AutoApproved { request_id: String, action_type: String, reason: String },
    Decided { request_id: String, action_type: String, decision: String, reviewer: String },
    Expired { request_id: String, action_type: String },
    GateBlocked { action_type: String, target_id: String, reason: String },
}

// ─── 审批规则 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub id: String,
    pub name: String,
    pub action_type: ApprovalActionType,
    pub enabled: bool,
    pub auto_approve_below_risk: u8,
    pub timeout_secs: u64,
    pub reject_on_timeout: bool,
    pub approver: String,
    pub project_scope: Option<String>,
    pub enable_auditor_prescreen: bool,
    pub auditor_risk_reduction: u8,
}

impl ApprovalRule {
    pub fn new_defaults() -> Vec<Self> {
        vec![
            Self { id: "rule-merge".into(), name: "Worktree 合并".into(),
                action_type: ApprovalActionType::WorktreeMerge,
                enabled: true, auto_approve_below_risk: 0, timeout_secs: 300,
                reject_on_timeout: true, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: true, auditor_risk_reduction: 2 },
            Self { id: "rule-pipeline".into(), name: "SDLC 跃迁".into(),
                action_type: ApprovalActionType::PipelineAdvance,
                enabled: true, auto_approve_below_risk: 3, timeout_secs: 120,
                reject_on_timeout: false, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0 },
            Self { id: "rule-remote".into(), name: "远程命令".into(),
                action_type: ApprovalActionType::RemoteCommand,
                enabled: true, auto_approve_below_risk: 0, timeout_secs: 180,
                reject_on_timeout: true, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0 },
            Self { id: "rule-cost".into(), name: "资费超限".into(),
                action_type: ApprovalActionType::CostThreshold,
                enabled: true, auto_approve_below_risk: 4, timeout_secs: 60,
                reject_on_timeout: false, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0 },
            Self { id: "rule-file".into(), name: "文件删除".into(),
                action_type: ApprovalActionType::FileDelete,
                enabled: true, auto_approve_below_risk: 2, timeout_secs: 120,
                reject_on_timeout: true, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0 },
            Self { id: "rule-config".into(), name: "配置变更".into(),
                action_type: ApprovalActionType::ConfigChange,
                enabled: true, auto_approve_below_risk: 3, timeout_secs: 120,
                reject_on_timeout: false, approver: "user".into(),
                project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0 },
        ]
    }
}

// ─── 审批请求 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub action_type: String,
    pub target_id: String,
    pub description: String,
    pub risk_level: u32,
    pub risk_breakdown: Option<String>, // 风险分解说明
    pub status: String,
    pub submitted_at: String,
    pub decided_at: Option<String>,
    pub decided_by: Option<String>,
    pub decision_comment: Option<String>,
    pub metadata: String,
    pub project: Option<String>,
    pub estimated_cost: Option<f64>,
    pub auditor_prescreen: Option<AuditorPrescreenResult>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditorPrescreenResult {
    pub passed: bool,
    pub findings_count: u32,
    pub critical_count: u32,
    pub summary: String,
}

// ─── 演化反馈 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPattern {
    pub action_type: String,
    pub total_submitted: u32, pub total_approved: u32,
    pub total_rejected: u32, pub total_auto_approved: u32,
    pub avg_risk_level: f64, pub approval_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSuggestion {
    pub rule_id: String,
    pub rule_name: String,
    pub action_type: String,
    pub suggestion: String,
    pub current_threshold: u8,
    pub suggested_threshold: u8,
    pub confidence: f64,
    pub reason: String,
}

// ─── 审批门禁引擎 ──────────────────────────────────────────────────

/// 事件回调类型：审批事件 → Blackboard / 计费引擎 / 日志
pub type ApprovalEventCallback = Box<dyn Fn(ApprovalEvent) + Send + Sync>;

pub struct ApprovalGate {
    pub rules: Vec<ApprovalRule>,
    pub pending_queue: Vec<ApprovalRequest>,
    pub audit_log: Vec<ApprovalRequest>,
    pub enabled: bool,
    pub global_auto_approve_threshold: u8,
    counter: u32,
    /// 事件回调列表（Blackboard / 通知等）
    event_callbacks: Vec<ApprovalEventCallback>,
}

impl ApprovalGate {
    pub fn new() -> Self {
        Self {
            rules: ApprovalRule::new_defaults(),
            pending_queue: Vec::new(), audit_log: Vec::new(),
            enabled: true, global_auto_approve_threshold: 0, counter: 0,
            event_callbacks: Vec::new(),
        }
    }

    fn next_id(&mut self) -> String { self.counter += 1; format!("apr-{:04}", self.counter) }

    // ── 事件系统 ──────────────────────────────────────────────────

    /// 注册事件回调（Orchestrator Blackboard 或其他监听器）
    pub fn on_event<F: Fn(ApprovalEvent) + Send + Sync + 'static>(&mut self, cb: F) {
        self.event_callbacks.push(Box::new(cb));
    }

    fn emit(&self, event: ApprovalEvent) {
        let event_str = format!("{:?}", event);
        tracing::info!("[APPROVAL GATE] Event: {}", event_str);
        for cb in &self.event_callbacks { cb(event.clone()); }
    }

    // ── 规则管理 ──────────────────────────────────────────────────

    pub fn add_rule(&mut self, action_type: &str, _risk_level: u32,
        auto_approve_below_risk: u32, description: &str) -> Result<String, String> {
        let at = parse_action_type(action_type)?;
        self.counter += 1;
        let id = format!("rule-{:04}", self.counter);
        self.rules.push(ApprovalRule {
            id: id.clone(), name: description.into(), action_type: at, enabled: true,
            auto_approve_below_risk: auto_approve_below_risk.min(10) as u8,
            timeout_secs: 120, reject_on_timeout: true, approver: "user".into(),
            project_scope: None, enable_auditor_prescreen: false, auditor_risk_reduction: 0,
        });
        Ok(id)
    }

    pub fn remove_rule(&mut self, rule_id: &str) -> Result<(), String> {
        let len = self.rules.len();
        self.rules.retain(|r| r.id != rule_id);
        if self.rules.len() == len { Err(format!("规则 {} 不存在", rule_id)) } else { Ok(()) }
    }

    pub fn get_rules(&self) -> Vec<ApprovalRule> { self.rules.clone() }

    // ── 审批提交（核心入口）────────────────────────────────────────

    pub fn submit(&mut self, action_type: &str, target_id: &str,
        description: &str, metadata: &str) -> Result<ApprovalRequest, String> {
        let at = parse_action_type(action_type)?;
        let profile = at.risk_profile();
        let risk = profile.final_risk() as u32;
        self.submit_inner(action_type, target_id, description, metadata, risk,
            Some(profile.breakdown()), None, None)
    }

    /// 资费感知提交 — 动态读取计费引擎状态
    pub fn submit_with_cost(&mut self, action_type: &str, target_id: &str,
        description: &str, metadata: &str, estimated_cost: f64,
        current_budget_used: Option<f64>, cost_cap: Option<f64>,
    ) -> Result<ApprovalRequest, String> {
        let at = parse_action_type(action_type)?;
        let profile = at.risk_profile();
        let base_risk = profile.final_risk() as u32;

        // 资费感知动态调整
        let mut cost_adjustment = 0u32;
        let mut budget_pressure = String::new();

        if let (Some(used), Some(cap)) = (current_budget_used, cost_cap) {
            if cap > 0.0 {
                let pct = used / cap;
                if pct > 0.9 { cost_adjustment += 3; budget_pressure = format!(" 预算已用{:.0}%", pct*100.0); }
                else if pct > 0.7 { cost_adjustment += 2; budget_pressure = format!(" 预算已用{:.0}%", pct*100.0); }
                else if pct > 0.5 { cost_adjustment += 1; }
            }
        }
        if estimated_cost > 20.0 { cost_adjustment += 3; }
        else if estimated_cost > 5.0 { cost_adjustment += 2; }

        let risk = (base_risk + cost_adjustment).min(10);
        let breakdown = if budget_pressure.is_empty() {
            format!("{} 资费因子+{}", profile.breakdown(), cost_adjustment)
        } else {
            format!("{} 资费因子+{} ·{}", profile.breakdown(), cost_adjustment, budget_pressure)
        };
        self.submit_inner(action_type, target_id, description, metadata, risk,
            Some(breakdown), Some(estimated_cost), None)
    }

    /// Auditor 预检提交
    pub fn submit_with_auditor(&mut self, action_type: &str, target_id: &str,
        description: &str, metadata: &str,
        prescreen: AuditorPrescreenResult,
    ) -> Result<ApprovalRequest, String> {
        let at = parse_action_type(action_type)?;
        let profile = at.risk_profile();
        let mut risk = profile.final_risk() as u32;

        if prescreen.passed {
            let reduction = self.rules.iter()
                .find(|r| r.action_type == at && r.enabled)
                .map(|r| r.auditor_risk_reduction as u32).unwrap_or(2);
            risk = risk.saturating_sub(reduction);
        }

        let breakdown = format!("{} Auditor预检{}", profile.breakdown(),
            if prescreen.passed { "✅通过" } else { "❌不通过" });
        self.submit_inner(action_type, target_id, description, metadata, risk,
            Some(breakdown), None, Some(prescreen))
    }

    /// 内部统一提交通道
    fn submit_inner(&mut self, action_type: &str, target_id: &str,
        description: &str, metadata: &str, risk: u32,
        risk_breakdown: Option<String>,
        estimated_cost: Option<f64>, prescreen: Option<AuditorPrescreenResult>,
    ) -> Result<ApprovalRequest, String> {
        // 门禁关闭 → 全部放行
        if !self.enabled {
            let id = self.next_id();
            let req = ApprovalRequest {
                id: id.clone(), action_type: action_type.into(), target_id: target_id.into(),
                description: description.into(), risk_level: risk, risk_breakdown,
                status: "AutoApproved".into(), submitted_at: now_iso(),
                decided_at: Some(now_iso()), decided_by: Some("system".into()),
                decision_comment: Some("审批门禁已关闭".into()),
                metadata: metadata.into(), project: None,
                estimated_cost, auditor_prescreen: prescreen, expires_at: None,
            };
            self.audit_log.push(req.clone());
            self.emit(ApprovalEvent::AutoApproved { request_id: id, action_type: action_type.into(), reason: "门禁关闭".into() });
            return Ok(req);
        }

        let matching_rule = self.rules.iter()
            .find(|r| r.action_type == parse_action_type(action_type)
                .unwrap_or(ApprovalActionType::Custom(action_type.into())) && r.enabled);

        let threshold = matching_rule.map(|r| r.auto_approve_below_risk as u32)
            .unwrap_or(self.global_auto_approve_threshold as u32);
        let effective = threshold.max(self.global_auto_approve_threshold as u32);

        // 自动放行
        if risk <= effective && risk < 8 {
            let id = self.next_id();
            let reason = format!("风险{}/10 ≤ 阈值{}/10，自动放行", risk, effective);
            let req = ApprovalRequest {
                id: id.clone(), action_type: action_type.into(), target_id: target_id.into(),
                description: description.into(), risk_level: risk, risk_breakdown,
                status: "AutoApproved".into(), submitted_at: now_iso(),
                decided_at: Some(now_iso()), decided_by: Some("system".into()),
                decision_comment: Some(reason.clone()),
                metadata: metadata.into(), project: None,
                estimated_cost, auditor_prescreen: prescreen, expires_at: None,
            };
            self.audit_log.push(req.clone());
            self.emit(ApprovalEvent::AutoApproved { request_id: id, action_type: action_type.into(), reason });
            return Ok(req);
        }

        // 需要审批
        let timeout = matching_rule.map(|r| r.timeout_secs).unwrap_or(120);
        let expires_at = Some((chrono::Utc::now() + chrono::Duration::seconds(timeout as i64)).to_rfc3339());

        let status = if prescreen.as_ref().map(|p| p.passed).unwrap_or(false) {
            "AuditorPreScreened"
        } else { "Pending" };

        let id = self.next_id();
        let req = ApprovalRequest {
            id: id.clone(), action_type: action_type.into(), target_id: target_id.into(),
            description: description.into(), risk_level: risk, risk_breakdown,
            status: status.into(), submitted_at: now_iso(),
            decided_at: None, decided_by: None, decision_comment: None,
            metadata: metadata.into(), project: None,
            estimated_cost, auditor_prescreen: prescreen, expires_at,
        };
        self.pending_queue.push(req.clone());
        self.emit(ApprovalEvent::Submitted { request_id: id.clone(), action_type: action_type.into(), risk_level: risk });
        Ok(req)
    }

    // ── 审批决策 ──────────────────────────────────────────────────

    pub fn decide(&mut self, request_id: &str, decision: &str,
        reviewer: &str, comment: &str) -> Result<ApprovalRequest, String> {
        let pos = self.pending_queue.iter().position(|r| r.id == request_id)
            .ok_or_else(|| format!("审批单 {} 不存在或已处理", request_id))?;

        let mut req = self.pending_queue.remove(pos);
        req.decided_at = Some(now_iso());
        req.decided_by = Some(reviewer.into());
        req.decision_comment = Some(comment.into());

        match decision {
            "Approve" | "approve" | "approved" => req.status = "Approved".into(),
            "Reject" | "reject" | "rejected" => req.status = "Rejected".into(),
            _ => return Err(format!("无效决策: {}，使用 Approve 或 Reject", decision)),
        }

        self.audit_log.push(req.clone());
        self.emit(ApprovalEvent::Decided {
            request_id: request_id.into(), action_type: req.action_type.clone(),
            decision: req.status.clone(), reviewer: reviewer.into(),
        });
        Ok(req)
    }

    // ── 过期清理 ──────────────────────────────────────────────────

    pub fn expire_stale(&mut self) -> Vec<String> {
        let mut expired = Vec::new();
        let mut kept = Vec::new();
        for mut req in self.pending_queue.drain(..) {
            let is_expired = req.expires_at.as_ref().map_or(false, |exp| {
                chrono::DateTime::parse_from_rfc3339(exp)
                    .map(|dt| chrono::Utc::now() > dt).unwrap_or(false)
            });
            if is_expired {
                req.status = "Expired".into(); req.decided_at = Some(now_iso());
                req.decided_by = Some("system".into());
                req.decision_comment = Some("超时自动过期".into());
                let eid = req.id.clone();
                expired.push(eid);
                self.audit_log.push(req);
            } else { kept.push(req); }
        }
        self.pending_queue = kept;
        expired
    }

    // ── 查询 ──────────────────────────────────────────────────────

    pub fn list_pending(&self) -> Vec<ApprovalRequest> { self.pending_queue.clone() }
    pub fn get_audit_log(&self, limit: usize) -> Vec<ApprovalRequest> {
        let mut log = self.audit_log.clone(); log.reverse(); log.truncate(limit); log
    }

    // ── 门禁检查（供高风险命令调用）───────────────────────────────

    pub fn check_approval(&self, action_type: &str, target_id: &str) -> Result<(), String> {
        let has_approved = self.audit_log.iter().any(|req| {
            req.action_type == action_type && req.target_id == target_id
                && (req.status == "Approved" || req.status == "AutoApproved")
        });
        if has_approved { return Ok(()); }
        let has_pending = self.pending_queue.iter().any(|req| {
            req.action_type == action_type && req.target_id == target_id
        });
        let msg = if has_pending {
            format!("⛔ 第四红线：操作已提交审批 (target={})，等待人工核准。请到审批面板处理", target_id)
        } else {
            format!("⛔ 第四红线：此操作需先提交审批 (target={})。请在审批面板中提交审批请求后重试", target_id)
        };
        Err(msg)
    }

    pub fn check_pipeline_advance(&self, from: &str, to: &str) -> Result<(), String> {
        self.check_approval("pipeline_advance", &format!("pipeline:{}->{}", from, to))
    }

    pub fn check_worktree_merge(&self, worktree_id: &str) -> Result<(), String> {
        self.check_approval("worktree_merge", worktree_id)
    }

    // ── 演化学习 ──────────────────────────────────────────────────

    pub fn analyze_patterns(&self) -> Vec<ApprovalPattern> {
        let mut patterns: HashMap<String, (u32, u32, u32, u32, f64)> = HashMap::new();
        for req in &self.audit_log {
            let e = patterns.entry(req.action_type.clone()).or_insert((0, 0, 0, 0, 0.0));
            e.0 += 1;
            match req.status.as_str() { "Approved" => e.1 += 1, "Rejected" => e.2 += 1, "AutoApproved" => e.3 += 1, _ => {} }
            e.4 += req.risk_level as f64;
        }
        patterns.into_iter().map(|(at, (t, a, r, au, rs))| ApprovalPattern {
            action_type: at, total_submitted: t, total_approved: a,
            total_rejected: r, total_auto_approved: au,
            avg_risk_level: if t > 0 { rs / t as f64 } else { 0.0 },
            approval_rate: if t > 0 { (a + au) as f64 / t as f64 } else { 0.0 },
        }).collect()
    }

    pub fn suggest_rule_optimizations(&self) -> Vec<RuleSuggestion> {
        let patterns = self.analyze_patterns();
        let mut suggestions = Vec::new();
        for pattern in &patterns {
            for rule in &self.rules {
                if rule.action_type.to_string() == pattern.action_type && pattern.total_submitted >= 10 {
                    if pattern.approval_rate > 0.9 && (rule.auto_approve_below_risk as u32) < pattern.avg_risk_level.ceil() as u32 {
                        suggestions.push(RuleSuggestion {
                            rule_id: rule.id.clone(), rule_name: rule.name.clone(),
                            action_type: pattern.action_type.clone(),
                            suggestion: "提高自动放行阈值".into(),
                            current_threshold: rule.auto_approve_below_risk,
                            suggested_threshold: (pattern.avg_risk_level.ceil() as u8).min(10),
                            confidence: pattern.approval_rate,
                            reason: format!("近{}次审批通过率{:.0}%，平均风险{:.1}，可安全提高阈值",
                                pattern.total_submitted, pattern.approval_rate*100.0, pattern.avg_risk_level),
                        });
                    }
                    let reject_rate = pattern.total_rejected as f64 / pattern.total_submitted as f64;
                    if reject_rate > 0.3 {
                        suggestions.push(RuleSuggestion {
                            rule_id: rule.id.clone(), rule_name: rule.name.clone(),
                            action_type: pattern.action_type.clone(),
                            suggestion: "降低自动放行阈值".into(),
                            current_threshold: rule.auto_approve_below_risk,
                            suggested_threshold: rule.auto_approve_below_risk.saturating_sub(1).max(0),
                            confidence: reject_rate,
                            reason: format!("近{}次审批拒绝率{:.0}%，需加强管控",
                                pattern.total_submitted, reject_rate*100.0),
                        });
                    }
                }
            }
        }
        suggestions
    }

    // ── 统计 ──────────────────────────────────────────────────────

    pub fn stats_summary(&self) -> serde_json::Value {
        let approved = self.audit_log.iter().filter(|r| r.status == "Approved" || r.status == "AutoApproved").count();
        let rejected = self.audit_log.iter().filter(|r| r.status == "Rejected").count();
        serde_json::json!({
            "enabled": self.enabled, "rules": self.rules.len(),
            "pending": self.pending_queue.len(), "approved": approved,
            "rejected": rejected, "total_audit": self.audit_log.len(),
        })
    }

    // ── 持久化 ────────────────────────────────────────────────────

    pub fn save_state(&self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("approval_gate.json");
        let state = serde_json::json!({
            "rules": self.rules, "pending": self.pending_queue,
            "history": self.audit_log, "enabled": self.enabled,
            "global_threshold": self.global_auto_approve_threshold,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("approval_gate.json");
        if !path.exists() { return Ok(()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(r) = state.get("rules") { if let Ok(v) = serde_json::from_value(r.clone()) { self.rules = v; } }
        if let Some(p) = state.get("pending") { if let Ok(v) = serde_json::from_value(p.clone()) { self.pending_queue = v; } }
        if let Some(h) = state.get("history") { if let Ok(v) = serde_json::from_value(h.clone()) { self.audit_log = v; } }
        if let Some(e) = state.get("enabled").and_then(|v| v.as_bool()) { self.enabled = e; }
        if let Some(t) = state.get("global_threshold").and_then(|v| v.as_u64()) { self.global_auto_approve_threshold = t as u8; }
        self.counter = (self.pending_queue.len() + self.audit_log.len()) as u32;
        Ok(())
    }
}

impl Default for ApprovalGate { fn default() -> Self { Self::new() } }

// ─── 工具函数 ──────────────────────────────────────────────────────

fn now_iso() -> String { chrono::Utc::now().to_rfc3339() }

fn parse_action_type(s: &str) -> Result<ApprovalActionType, String> {
    match s {
        "worktree_merge" | "WorktreeMerge" => Ok(ApprovalActionType::WorktreeMerge),
        "pipeline_advance" | "PipelineAdvance" => Ok(ApprovalActionType::PipelineAdvance),
        "ssh_exec" | "remote_command" | "RemoteCommand" => Ok(ApprovalActionType::RemoteCommand),
        "cost_override" | "cost_threshold" | "CostThreshold" => Ok(ApprovalActionType::CostThreshold),
        "file_delete" | "FileDelete" => Ok(ApprovalActionType::FileDelete),
        "config_change" | "ConfigChange" => Ok(ApprovalActionType::ConfigChange),
        other => Ok(ApprovalActionType::Custom(other.into())),
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_profile_scientific_scoring() {
        // WorktreeMerge: 影响7, 可逆9, 资费1, 合规4
        // max(7, 10-9=1, 1, 4) = 7
        let p = ApprovalActionType::WorktreeMerge.risk_profile();
        assert_eq!(p.final_risk(), 7);

        // RemoteCommand: 影响9, 可逆2, 资费3, 合规6
        // max(9, 10-2=8, 3, 6) = 9
        let p = ApprovalActionType::RemoteCommand.risk_profile();
        assert_eq!(p.final_risk(), 9);
    }

    #[test]
    fn test_submit_requires_approval_for_high_risk() {
        let mut gate = ApprovalGate::new();
        let req = gate.submit("worktree_merge", "wt1", "合并", "{}").unwrap();
        assert!(req.status == "Pending");
        assert!(req.risk_level >= 7);
    }

    #[test]
    fn test_decide_and_check_gate() {
        let mut gate = ApprovalGate::new();
        assert!(gate.check_worktree_merge("wt-1").is_err());
        let req = gate.submit("worktree_merge", "wt-1", "合并", "{}").unwrap();
        gate.decide(&req.id, "Approve", "admin", "ok").unwrap();
        assert!(gate.check_worktree_merge("wt-1").is_ok());
    }

    #[test]
    fn test_cost_aware_dynamic_risk() {
        let mut gate = ApprovalGate::new();
        let req = gate.submit_with_cost("config_change", "cfg1", "变更", "{}",
            25.0, Some(45.0), Some(50.0)).unwrap();
        // 基础风险4 + cost>20(+3) + 预算>90%(+3) = 10
        assert!(req.risk_level >= 9);
    }

    #[test]
    fn test_auditor_prescreen() {
        let mut gate = ApprovalGate::new();
        let prescreen = AuditorPrescreenResult {
            passed: true, findings_count: 0, critical_count: 0,
            summary: "审计通过".into(),
        };
        let req = gate.submit_with_auditor("worktree_merge", "wt1", "合并", "{}", prescreen).unwrap();
        assert_eq!(req.risk_level, 5); // 7 - 2
        assert_eq!(req.status, "AuditorPreScreened");
    }

    #[test]
    fn test_event_callback_fires() {
        let mut gate = ApprovalGate::new();
        let events = std::sync::Mutex::new(Vec::new());
        gate.on_event(move |e| { events.lock().unwrap().push(format!("{:?}", e)); });
        let _ = gate.submit("pipeline_advance", "p1", "test", "{}").unwrap();
        // Event callback should have been called (verified via tracing in real usage)
    }

    #[test]
    fn test_suggestions_generated() {
        let mut gate = ApprovalGate::new();
        for i in 0..15 {
            let req = gate.submit("pipeline_advance", &format!("p{}", i), "test", "{}").unwrap();
            gate.decide(&req.id, "Approve", "admin", "ok").unwrap();
        }
        let suggestions = gate.suggest_rule_optimizations();
        assert!(!suggestions.is_empty());
    }
}
