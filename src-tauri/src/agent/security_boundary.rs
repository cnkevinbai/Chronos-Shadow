// 安全边界与用户许可管理系统 (Security Boundary & Consent Manager)
//
// 未经用户明确许可授权，AI/Agent 绝对不得：
//   ❌ 删除项目/数据库/文件 (删库)
//   ❌ 访问外网/社交媒体/发布内容
//   ❌ 联系用户社交平台联系人
//   ❌ 读取/修改用户隐私数据
//   ❌ 绕过审批门禁执行高危操作
//   ❌ 将本地数据发送到外部服务
//
// 设计原则：
//   1. 默认拒绝 (Deny-by-Default) — 未授权操作一律拦截
//   2. 最小权限 (Least Privilege) — 仅授予完成任务所需最小权限
//   3. 显式许可 (Explicit Consent) — 高危操作必须用户明确批准
//   4. 完整审计 (Full Audit) — 所有边界决策可追溯

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 操作许可等级 ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// 0级: 完全禁止 — 删库/访问外网/联系社交联系人/数据外泄
    Forbidden,
    /// 1级: 需要用户显式审批 (通过第四红线)
    RequireApproval,
    /// 2级: 需要用户确认 (弹窗确认)
    RequireConfirmation,
    /// 3级: 沙盒内只读操作
    SandboxReadOnly,
    /// 4级: 沙盒内读写操作
    SandboxReadWrite,
    /// 5级: 允许但记录审计日志
    AllowedAudited,
    /// 6级: 完全允许
    Allowed,
}

impl PermissionLevel {
    pub fn label(&self) -> &str {
        match self {
            Self::Forbidden => "🚫 禁止",
            Self::RequireApproval => "🛡️ 需审批",
            Self::RequireConfirmation => "⚠️ 需确认",
            Self::SandboxReadOnly => "📖 沙盒只读",
            Self::SandboxReadWrite => "✏️ 沙盒读写",
            Self::AllowedAudited => "✅ 允许(审计)",
            Self::Allowed => "🟢 允许",
        }
    }
}

// ─── 操作分类 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationCategory {
    // 🚫 完全禁止类
    DeleteProject,       // 删除整个项目
    DeleteDatabase,      // 删除数据库
    AccessExternalNetwork, // 访问外网/社交媒体（兜底规则）
    ContactSocialContacts, // 联系社交平台联系人
    DataExfiltration,    // 数据外泄
    SystemModification,  // 修改系统文件
    SocialPostWrite,     // 社交平台发布/写入（永远禁止）
    DataUploadExternal,  // 数据上传到外部服务（永远禁止）

    // 🛡️ 需要审批类
    WorktreeMerge,       // 合并代码
    PipelineAdvance,     // 流水线跃迁
    RemoteCommand,       // 远程命令
    CostOverride,        // 费用超限
    ConfigChange,        // 配置变更
    // ── 外网信息获取（审批后可放行） ──
    WebSearch,           // 搜索引擎查询（只读）
    WebFetchReadonly,    // 只读网页抓取
    ApiCallReadonly,     // 只读 API 调用（白名单域名）

    // ⚠️ 需要确认类
    FileDelete,          // 删除文件
    SessionDelete,       // 删除会话
    CheckpointDelete,    // 删除检查点

    // 📖 沙盒只读
    FileRead,            // 读文件
    ProjectList,         // 列出项目
    SessionRead,         // 读会话

    // ✏️ 沙盒读写
    FileWrite,           // 写文件
    CodeGeneration,      // 代码生成
    CheckpointCreate,    // 创建检查点

    // 🟢 允许
    ChatMessage,         // 聊天消息
    StatusQuery,         // 状态查询
}

impl OperationCategory {
    /// 默认许可等级
    pub fn default_permission(&self) -> PermissionLevel {
        match self {
            Self::DeleteProject | Self::DeleteDatabase | Self::AccessExternalNetwork |
            Self::ContactSocialContacts | Self::DataExfiltration | Self::SystemModification |
            Self::SocialPostWrite | Self::DataUploadExternal => PermissionLevel::Forbidden,
            Self::WorktreeMerge | Self::PipelineAdvance | Self::RemoteCommand |
            Self::CostOverride | Self::ConfigChange |
            Self::WebSearch | Self::WebFetchReadonly | Self::ApiCallReadonly => PermissionLevel::RequireApproval,
            Self::FileDelete | Self::SessionDelete | Self::CheckpointDelete => PermissionLevel::RequireConfirmation,
            Self::FileRead | Self::ProjectList | Self::SessionRead => PermissionLevel::SandboxReadOnly,
            Self::FileWrite | Self::CodeGeneration | Self::CheckpointCreate => PermissionLevel::SandboxReadWrite,
            Self::ChatMessage | Self::StatusQuery => PermissionLevel::Allowed,
        }
    }

    /// 检测LLM输出中是否包含越界意图
    pub fn detect_in_llm_output(text: &str) -> Vec<(OperationCategory, String)> {
        let lower = text.to_lowercase();
        let mut detected = Vec::new();

        // 🚫 删库检测
        if lower.contains("drop database") || lower.contains("drop table") || lower.contains("truncate") {
            detected.push((Self::DeleteDatabase, "检测到数据库删除意图".into()));
        }
        if lower.contains("rm -rf") || lower.contains("del /f") || lower.contains("format c:") {
            detected.push((Self::DeleteProject, "检测到危险系统命令".into()));
        }

        // 🚫 外网恶意访问检测（写操作/数据外泄）
        if (lower.contains("curl") || lower.contains("wget") || lower.contains("fetch("))
           && (lower.contains("--data") || lower.contains("-d ") || lower.contains("POST") || lower.contains("upload") || lower.contains("send")) {
            detected.push((Self::DataExfiltration, "检测到数据外发意图 (curl/wget/fetch + 写操作)".into()));
        }

        // 🚫 社交平台发布检测
        let social_domains = ["wechat", "weibo", "facebook", "twitter", "instagram", "tiktok", "linkedin",
            "微信", "微博", "抖音", "小红书", "朋友圈", "联系人列表"];
        for domain in &social_domains {
            if lower.contains(domain) {
                detected.push((Self::ContactSocialContacts, format!("检测到社交平台 '{}' 访问意图", domain)));
                break;
            }
        }

        // 🚫 数据上传到外部检测
        if (lower.contains("upload") || lower.contains("post to") || lower.contains("publish")) &&
           (lower.contains("http") || lower.contains("api.") || lower.contains(".com")) {
            detected.push((Self::DataUploadExternal, "检测到数据上传外部服务意图".into()));
        }

        // 🛡️ Web 只读搜索/抓取检测（可审批放行，不直接拦截）
        if lower.contains("web search") || lower.contains("search for") || lower.contains("搜索") ||
           lower.contains("google") || lower.contains("bing") || lower.contains("duckduckgo") {
            detected.push((Self::WebSearch, "检测到 Web 搜索意图（可审批放行）".into()));
        }
        if lower.contains("fetch url") || lower.contains("抓取") || lower.contains("crawl") ||
           (lower.contains("read") && (lower.contains("http://") || lower.contains("https://"))) {
            detected.push((Self::WebFetchReadonly, "检测到网页只读抓取意图（可审批放行）".into()));
        }

        // 🚫 系统修改检测
        if lower.contains("chmod 777") || lower.contains("sudo ") || lower.contains("registry") {
            detected.push((Self::SystemModification, "检测到系统修改意图".into()));
        }

        detected
    }
}

// ─── 许可决策结果 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub operation: OperationCategory,
    pub level: PermissionLevel,
    pub allowed: bool,
    pub reason: String,
    pub requires_approval_id: Option<String>,
    pub timestamp: String,
}

// ─── 安全边界管理器 ────────────────────────────────────────────────

pub struct SecurityBoundary {
    /// 操作许可矩阵
    permission_matrix: HashMap<OperationCategory, PermissionLevel>,
    /// 审计日志
    audit_trail: Vec<PermissionDecision>,
    /// 全局启用状态
    pub enabled: bool,
    /// 累计拦截次数
    pub blocked_count: u32,
}

impl SecurityBoundary {
    pub fn new() -> Self {
        let mut matrix = HashMap::new();
        // 初始化所有操作类型的默认许可
        let categories = [
            OperationCategory::DeleteProject, OperationCategory::DeleteDatabase,
            OperationCategory::AccessExternalNetwork, OperationCategory::ContactSocialContacts,
            OperationCategory::DataExfiltration, OperationCategory::SystemModification,
            OperationCategory::SocialPostWrite, OperationCategory::DataUploadExternal,
            OperationCategory::WorktreeMerge, OperationCategory::PipelineAdvance,
            OperationCategory::RemoteCommand, OperationCategory::CostOverride,
            OperationCategory::ConfigChange,
            OperationCategory::WebSearch, OperationCategory::WebFetchReadonly, OperationCategory::ApiCallReadonly,
            OperationCategory::FileDelete,
            OperationCategory::SessionDelete, OperationCategory::CheckpointDelete,
            OperationCategory::FileRead, OperationCategory::ProjectList,
            OperationCategory::SessionRead, OperationCategory::FileWrite,
            OperationCategory::CodeGeneration, OperationCategory::CheckpointCreate,
            OperationCategory::ChatMessage, OperationCategory::StatusQuery,
        ];
        for cat in &categories {
            matrix.insert(cat.clone(), cat.default_permission());
        }
        Self { permission_matrix: matrix, audit_trail: Vec::new(), enabled: true, blocked_count: 0 }
    }

    /// 核心方法: 检查操作是否被允许
    pub fn check_permission(&mut self, operation: OperationCategory, context: &str) -> PermissionDecision {
        let level = self.permission_matrix.get(&operation).cloned()
            .unwrap_or(PermissionLevel::RequireApproval);

        let allowed = match &level {
            PermissionLevel::Forbidden => {
                self.blocked_count += 1;
                false
            }
            PermissionLevel::RequireApproval | PermissionLevel::RequireConfirmation => false,
            _ => true,
        };

        let reason = if !allowed {
            match &level {
                PermissionLevel::Forbidden =>
                    format!("⛔ 安全边界拦截: {} — 此操作被永久禁止。{} 未经用户许可不得执行", operation_name(&operation), context),
                PermissionLevel::RequireApproval =>
                    format!("🛡️ 需要审批: {} — 请通过第四红线审批门禁提交", operation_name(&operation)),
                PermissionLevel::RequireConfirmation =>
                    format!("⚠️ 需要确认: {} — 请在前端弹窗确认", operation_name(&operation)),
                _ => String::new(),
            }
        } else {
            format!("✅ 允许: {}", operation_name(&operation))
        };

        let decision = PermissionDecision {
            operation: operation.clone(), level: level.clone(), allowed,
            reason: reason.clone(), requires_approval_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.audit_trail.push(decision.clone());

        if !allowed && matches!(level, PermissionLevel::Forbidden) {
            tracing::warn!("[SECURITY BOUNDARY] BLOCKED: {:?} — {}", operation, reason);
        }

        decision
    }

    /// 扫描LLM输出中的越界意图
    pub fn scan_llm_output(&mut self, text: &str) -> Vec<PermissionDecision> {
        let violations = OperationCategory::detect_in_llm_output(text);
        violations.into_iter()
            .map(|(cat, reason)| self.check_permission(cat, &reason))
            .filter(|d| !d.allowed)
            .collect()
    }

    /// 审计日志
    pub fn audit_log(&self, limit: usize) -> Vec<&PermissionDecision> {
        self.audit_trail.iter().rev().take(limit).collect()
    }

    /// 安全报告
    pub fn security_report(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.enabled,
            "blocked_count": self.blocked_count,
            "total_decisions": self.audit_trail.len(),
            "permissions": self.permission_matrix.iter().map(|(k, v)| {
                serde_json::json!({ "operation": format!("{:?}", k), "level": v.label() })
            }).collect::<Vec<_>>(),
        })
    }
}

impl Default for SecurityBoundary {
    fn default() -> Self { Self::new() }
}

fn operation_name(op: &OperationCategory) -> &str {
    match op {
        OperationCategory::DeleteProject => "删除项目",
        OperationCategory::DeleteDatabase => "删除数据库",
        OperationCategory::AccessExternalNetwork => "访问外网(兜底)",
        OperationCategory::ContactSocialContacts => "联系社交联系人",
        OperationCategory::DataExfiltration => "数据外泄",
        OperationCategory::SystemModification => "系统修改",
        OperationCategory::SocialPostWrite => "社交平台发布",
        OperationCategory::DataUploadExternal => "数据上传外部",
        OperationCategory::WorktreeMerge => "Worktree合并",
        OperationCategory::PipelineAdvance => "流水线跃迁",
        OperationCategory::RemoteCommand => "远程命令",
        OperationCategory::CostOverride => "费用超限",
        OperationCategory::ConfigChange => "配置变更",
        OperationCategory::WebSearch => "Web搜索",
        OperationCategory::WebFetchReadonly => "网页只读抓取",
        OperationCategory::ApiCallReadonly => "只读API调用",
        OperationCategory::FileDelete => "删除文件",
        OperationCategory::SessionDelete => "删除会话",
        OperationCategory::CheckpointDelete => "删除检查点",
        OperationCategory::FileRead => "读文件",
        OperationCategory::ProjectList => "列出项目",
        OperationCategory::SessionRead => "读会话",
        OperationCategory::FileWrite => "写文件",
        OperationCategory::CodeGeneration => "代码生成",
        OperationCategory::CheckpointCreate => "创建检查点",
        OperationCategory::ChatMessage => "聊天消息",
        OperationCategory::StatusQuery => "状态查询",
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forbidden_operations_blocked() {
        let mut boundary = SecurityBoundary::new();
        let decision = boundary.check_permission(OperationCategory::DeleteDatabase, "test");
        assert!(!decision.allowed);
        assert_eq!(boundary.blocked_count, 1);
    }

    #[test]
    fn test_allowed_operations_pass() {
        let mut boundary = SecurityBoundary::new();
        let decision = boundary.check_permission(OperationCategory::ChatMessage, "hello");
        assert!(decision.allowed);
    }

    #[test]
    fn test_detect_drop_database() {
        let violations = OperationCategory::detect_in_llm_output("Let me DROP DATABASE users to fix this");
        assert!(violations.iter().any(|(c, _)| *c == OperationCategory::DeleteDatabase));
    }

    #[test]
    fn test_detect_social_media() {
        let violations = OperationCategory::detect_in_llm_output("I'll post this on 微信 to notify your contacts");
        assert!(violations.iter().any(|(c, _)| *c == OperationCategory::ContactSocialContacts));
    }

    #[test]
    fn test_detect_rm_rf() {
        let violations = OperationCategory::detect_in_llm_output("Just run rm -rf / to clean up");
        assert!(violations.iter().any(|(c, _)| *c == OperationCategory::DeleteProject));
    }

    #[test]
    fn test_scan_llm_output_blocks() {
        let mut boundary = SecurityBoundary::new();
        let blocks = boundary.scan_llm_output("I'll DROP DATABASE and post on facebook");
        assert_eq!(blocks.len(), 2);
        assert_eq!(boundary.blocked_count, 2);
    }

    #[test]
    fn test_web_search_detected_not_blocked() {
        let violations = OperationCategory::detect_in_llm_output("I will search for Rust async patterns on google");
        assert!(violations.iter().any(|(c, _)| *c == OperationCategory::WebSearch));
    }

    #[test]
    fn test_web_fetch_detected_not_blocked() {
        let violations = OperationCategory::detect_in_llm_output("Let me read https://docs.rs/tokio to check the API");
        assert!(violations.iter().any(|(c, _)| *c == OperationCategory::WebFetchReadonly));
    }

    #[test]
    fn test_web_search_requires_approval() {
        let mut boundary = SecurityBoundary::new();
        let decision = boundary.check_permission(OperationCategory::WebSearch, "test query");
        assert!(!decision.allowed); // requires approval, not forbidden
        assert_eq!(decision.level, PermissionLevel::RequireApproval);
    }

    #[test]
    fn test_data_upload_still_forbidden() {
        let mut boundary = SecurityBoundary::new();
        let decision = boundary.check_permission(OperationCategory::DataUploadExternal, "test");
        assert!(!decision.allowed);
        assert_eq!(decision.level, PermissionLevel::Forbidden);
    }

    #[test]
    fn test_data_exfiltration_still_forbidden() {
        let mut boundary = SecurityBoundary::new();
        let decision = boundary.check_permission(OperationCategory::DataExfiltration, "test");
        assert!(!decision.allowed);
        assert_eq!(decision.level, PermissionLevel::Forbidden);
    }
}
