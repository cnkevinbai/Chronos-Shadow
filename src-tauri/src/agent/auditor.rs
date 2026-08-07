// 安全与合规审查 Agent (Auditor)
//
// 在代码进入编译前，精准审计增量部分：
// - 硬编码 Secrets 检测（API keys, tokens, passwords）
// - SQL 注入模式扫描
// - 开源协议合规检查（GPL 传染性协议检测）
// - 文件操作安全审计
// - 依赖合法性扫描

use serde::{Deserialize, Serialize};
use std::path::Path;

// ─── 审计结果类型 ──────────────────────────────────────────────────

/// 审计严重级别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// 单条审计发现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    /// 发现 ID
    pub id: String,
    /// 严重级别
    pub severity: Severity,
    /// 类别
    pub category: String,
    /// 描述
    pub description: String,
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: Option<u32>,
    /// 匹配的代码片段
    pub snippet: String,
    /// 修复建议
    pub recommendation: String,
}

/// 审计报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// 审计的文件数
    pub files_scanned: u32,
    /// 审计的行数
    pub lines_scanned: u32,
    /// 发现列表
    pub findings: Vec<AuditFinding>,
    /// 是否通过（无 Critical/High 发现）
    pub passed: bool,
    /// 各类别统计
    pub summary: AuditSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub secrets: u32,
    pub sql_injection: u32,
    pub license: u32,
    pub dangerous_imports: u32,
    pub other: u32,
}

// ─── 检测模式 ──────────────────────────────────────────────────────

/// Secrets 检测正则模式
const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("API[_-]?KEY\\s*=\\s*['\"]([^'\"]{8,})['\"]", "Hardcoded API Key"),
    ("SECRET[_-]?KEY\\s*=\\s*['\"]([^'\"]{8,})['\"]", "Hardcoded Secret Key"),
    ("TOKEN\\s*=\\s*['\"]([^'\"]{8,})['\"]", "Hardcoded Token"),
    ("PASSWORD\\s*=\\s*['\"]([^'\"]{3,})['\"]", "Hardcoded Password"),
    ("sk-[A-Za-z0-9]{20,}", "OpenAI API Key pattern"),
    ("AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
    ("ghp_[A-Za-z0-9]{36}", "GitHub Personal Access Token"),
    ("glpat-[A-Za-z0-9_-]{20,}", "GitLab Personal Access Token"),
    ("private[_-]?key", "Private key reference"),
    ("\\.pem", "PEM certificate file reference"),
    ("\\.p12", "PKCS#12 keystore reference"),
    ("connectionString.*=.*['\"].*password=", "Connection string with password"),
];

/// SQL 注入检测模式
const SQL_INJECTION_PATTERNS: &[(&str, &str)] = &[
    ("\"SELECT.*\\+.*\\+", "String concatenation in SQL"),
    ("\"INSERT.*\\+.*\\+", "String concatenation in INSERT"),
    ("\"UPDATE.*\\+.*\\+", "String concatenation in UPDATE"),
    ("\"DELETE.*\\+.*\\+", "String concatenation in DELETE"),
    ("format!.*SELECT", "format!() in SQL query"),
    ("f!.*SELECT", "f-string in SQL query"),
    ("execute\\(.*\\+.*\\)", "execute() with concatenation"),
    ("rawQuery|raw_query|unsafeQuery", "Unsafe raw query method"),
    ("\\.sql\\(.*\\$\\{", "Template literal in SQL"),
];

/// GPL/传染性协议检测
const LICENSE_PATTERNS: &[(&str, &str)] = &[
    ("GNU GENERAL PUBLIC LICENSE", "GPL detected"),
    ("GPL-3.0", "GPL v3 license"),
    ("GPL-2.0", "GPL v2 license"),
    ("AGPL", "AGPL license (network copyleft)"),
    ("GNU AFFERO", "AGPL detected"),
    ("SSPL", "Server Side Public License"),
    ("BUSL-1.1", "Business Source License (time-limited)"),
    ("license.*=.*\"GPL", "GPL license in Cargo.toml"),
    ("\"GPL", "GPL license reference"),
];

/// 危险 import/依赖检测
const DANGEROUS_IMPORT_PATTERNS: &[(&str, &str)] = &[
    ("unsafe\\s*\\{", "Unsafe block usage"),
    ("std::mem::transmute", "Type transmutation (memory unsafe)"),
    ("std::process::Command::new\\(\"cmd\"", "Shell command on Windows"),
    ("std::process::Command::new\\(\"sh\"", "Shell command on Unix"),
    ("eval\\(", "eval() — code injection risk"),
    ("exec\\(", "exec() — code injection risk"),
    ("system\\(", "system() call — command injection risk"),
    ("shell_exec\\(", "shell_exec() — command injection risk"),
    ("os\\.system\\(", "os.system() — unsafe"),
    ("subprocess\\.call\\(.*shell\\s*=\\s*True", "subprocess with shell=True"),
];

// ─── 审计引擎 ──────────────────────────────────────────────────────

/// 安全审计引擎
pub struct Auditor {
    /// 审计计数
    pub total_files: u32,
    /// 总发现数
    pub total_findings: u32,
    /// 是否启用 Secrets 检测
    pub check_secrets: bool,
    /// 是否启用 SQL 注入检测
    pub check_sql_injection: bool,
    /// 是否启用协议检测
    pub check_licenses: bool,
    /// 是否启用危险导入检测
    pub check_dangerous_imports: bool,
}

impl Auditor {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            total_findings: 0,
            check_secrets: true,
            check_sql_injection: true,
            check_licenses: true,
            check_dangerous_imports: true,
        }
    }

    /// 审计单个文件内容
    pub fn audit_file(&mut self, path: &Path, content: &str) -> AuditReport {
        self.total_files += 1;
        let lines: Vec<&str> = content.lines().collect();
        let mut findings = Vec::new();
        let mut counter: u32 = 0;

        let mut next_id = || {
            counter += 1;
            format!("AUDIT-{:04}", counter)
        };

        // 逐行扫描
        for (line_num, line) in lines.iter().enumerate() {
            let line_num = line_num as u32 + 1;

            // Secrets 检测
            if self.check_secrets {
                for (pattern, desc) in SECRET_PATTERNS {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if re.is_match(line) {
                            findings.push(AuditFinding {
                                id: next_id(),
                                severity: Severity::Critical,
                                category: "Secrets".into(),
                                description: desc.to_string(),
                                file: path.to_string_lossy().into(),
                                line: Some(line_num),
                                snippet: line.to_string(),
                                recommendation: "Move to environment variable or secrets manager".into(),
                            });
                        }
                    }
                }
            }

            // SQL 注入检测
            if self.check_sql_injection {
                for (pattern, desc) in SQL_INJECTION_PATTERNS {
                    if line.contains(pattern) || (pattern.contains("SELECT") && line.to_lowercase().contains("select") && line.contains("+") && (line.contains("\"") || line.contains("'") || line.contains("`"))) {
                        if line.contains(pattern.split('.').next().unwrap_or(pattern)) {
                            findings.push(AuditFinding {
                                id: next_id(),
                                severity: Severity::High,
                                category: "SQL Injection".into(),
                                description: desc.to_string(),
                                file: path.to_string_lossy().into(),
                                line: Some(line_num),
                                snippet: line.to_string(),
                                recommendation: "Use parameterized queries or prepared statements".into(),
                            });
                        }
                    }
                }
            }

            // License 检测
            if self.check_licenses {
                for (pattern, desc) in LICENSE_PATTERNS {
                    if line.to_uppercase().contains(&pattern.to_uppercase()) {
                        findings.push(AuditFinding {
                            id: next_id(),
                            severity: Severity::High,
                            category: "License".into(),
                            description: desc.to_string(),
                            file: path.to_string_lossy().into(),
                            line: Some(line_num),
                            snippet: line.to_string(),
                            recommendation: "Review license compatibility; GPL may impose copyleft obligations".into(),
                        });
                        break; // One license finding per line is enough
                    }
                }
            }

            // 危险 import 检测
            if self.check_dangerous_imports {
                for (pattern, desc) in DANGEROUS_IMPORT_PATTERNS {
                    if line.contains(pattern) {
                        findings.push(AuditFinding {
                            id: next_id(),
                            severity: Severity::Medium,
                            category: "Dangerous Import".into(),
                            description: desc.to_string(),
                            file: path.to_string_lossy().into(),
                            line: Some(line_num),
                            snippet: line.to_string(),
                            recommendation: "Review necessity; consider safer alternatives".into(),
                        });
                    }
                }
            }
        }

        self.total_findings += findings.len() as u32;

        let critical_or_high = findings
            .iter()
            .filter(|f| f.severity == Severity::Critical || f.severity == Severity::High)
            .count();

        let summary = AuditSummary {
            secrets: findings.iter().filter(|f| f.category == "Secrets").count() as u32,
            sql_injection: findings.iter().filter(|f| f.category == "SQL Injection").count() as u32,
            license: findings.iter().filter(|f| f.category == "License").count() as u32,
            dangerous_imports: findings.iter().filter(|f| f.category == "Dangerous Import").count() as u32,
            other: 0,
        };

        AuditReport {
            files_scanned: 1,
            lines_scanned: lines.len() as u32,
            findings,
            passed: critical_or_high == 0,
            summary,
        }
    }

    /// 审计多个文件
    pub fn audit_files(&mut self, files: &[(String, String)]) -> Vec<AuditReport> {
        files
            .iter()
            .map(|(path, content)| self.audit_file(Path::new(path), content))
            .collect()
    }

    /// 重置统计
    pub fn reset(&mut self) {
        self.total_files = 0;
        self.total_findings = 0;
    }

    /// 统计信息
    pub fn stats(&self) -> AuditorStats {
        AuditorStats {
            total_files: self.total_files,
            total_findings: self.total_findings,
        }
    }
}

impl Default for Auditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Auditor 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditorStats {
    pub total_files: u32,
    pub total_findings: u32,
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_detection() {
        let mut auditor = Auditor::new();
        let content = r#"
            const API_KEY = "sk-abcdefghijklmnopqrstuvwxyz123456"
            const normal = "hello"
        "#;
        let report = auditor.audit_file(Path::new("test.js"), content);
        assert!(!report.passed);
        assert!(report.summary.secrets > 0);
    }

    #[test]
    fn test_sql_injection_detection() {
        let mut auditor = Auditor::new();
        let content = r#"
            const query = "SELECT * FROM users WHERE id = " + userId
        "#;
        let report = auditor.audit_file(Path::new("test.js"), content);
        assert!(report.summary.sql_injection > 0);
    }

    #[test]
    fn test_license_detection() {
        let mut auditor = Auditor::new();
        let content = r#"
            This software is licensed under GNU GENERAL PUBLIC LICENSE v3
        "#;
        let report = auditor.audit_file(Path::new("LICENSE"), content);
        assert!(report.summary.license > 0);
    }

    #[test]
    fn test_clean_code_passes() {
        let mut auditor = Auditor::new();
        let content = r#"
            fn main() {
                println!("Hello, world!");
            }
        "#;
        let report = auditor.audit_file(Path::new("main.rs"), content);
        assert!(report.passed);
        assert_eq!(report.summary.secrets, 0);
    }

    #[test]
    fn test_aws_key_detected() {
        let mut auditor = Auditor::new();
        let content = "AWS_ACCESS_KEY_ID=AKIA1234567890ABCDEF";
        let report = auditor.audit_file(Path::new(".env"), content);
        assert!(report.summary.secrets > 0);
    }

    #[test]
    fn test_github_token_detected() {
        let mut auditor = Auditor::new();
        let content = r#"GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz1234"#;
        let report = auditor.audit_file(Path::new(".env"), content);
        assert!(report.summary.secrets > 0);
    }
}
