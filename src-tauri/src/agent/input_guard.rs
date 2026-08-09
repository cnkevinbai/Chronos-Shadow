// 输入验证与数据完整性保护模块 (Input Validation & Integrity Guard)
//
// 集中管理所有用户输入校验，防止:
//   - 路径注入 (../ 越权访问)
//   - 数值溢出 (NaN/Inf/负数费用/超限阈值)
//   - 字符串注入 (超长输入/控制字符)
//   - 数据损坏检测 (校验和验证)
//
// 设计原则: Fail-Closed — 默认拒绝，显式白名单

// ─── 路径安全 ──────────────────────────────────────────────────────

/// 校验文件路径，拒绝路径穿越攻击
pub fn validate_path_safe(path: &str) -> Result<(), String> {
    if path.is_empty() || path.len() > 1024 {
        return Err("路径长度不合法 (1-1024)".into());
    }
    if path.contains("..") {
        return Err(format!("路径穿越被拦截: {}", path));
    }
    // 拒绝绝对路径中的敏感目录
    let lower = path.to_lowercase();
    if lower.contains("\\windows\\system32") || lower.contains("/etc/passwd") {
        return Err("敏感系统路径被拦截".into());
    }
    Ok(())
}

/// 校验 ID 字符串 (字母数字 + 短横线/下划线, 1-128字符)
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 {
        return Err("ID 长度不合法 (1-128)".into());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("ID 包含非法字符: {}", id));
    }
    Ok(())
}

// ─── 数值安全 ──────────────────────────────────────────────────────

/// 校验成本值 (0.0 ~ 100000.0, 拒绝NaN/Inf)
pub fn validate_cost(value: f64) -> Result<(), String> {
    if value.is_nan() || value.is_infinite() {
        return Err("费用值不合法 (NaN/Inf)".into());
    }
    if value < 0.0 || value > 100000.0 {
        return Err(format!("费用值超出范围 [0, 100000]: {}", value));
    }
    Ok(())
}

/// 校验风险评分 (0-10)
pub fn validate_risk_score(score: u32) -> Result<(), String> {
    if score > 10 {
        return Err(format!("风险评分超出范围 [0, 10]: {}", score));
    }
    Ok(())
}

/// 校验置信度 (0.0-1.0)
pub fn validate_confidence(conf: f32) -> Result<(), String> {
    if conf.is_nan() || conf.is_infinite() || conf < 0.0 || conf > 1.0 {
        return Err(format!("置信度超出范围 [0, 1]: {}", conf));
    }
    Ok(())
}

/// 校验计数器 (拒绝溢出)
pub fn validate_count_limit(count: u32, limit: u32) -> Result<(), String> {
    if count > limit {
        return Err(format!("数量超限: {} > {}", count, limit));
    }
    Ok(())
}

// ─── 字符串安全 ──────────────────────────────────────────────────

/// 校验用户输入文本长度
pub fn validate_text_len(text: &str, max_len: usize) -> Result<(), String> {
    if text.len() > max_len {
        return Err(format!("输入文本超长: {} > {}", text.len(), max_len));
    }
    Ok(())
}

/// 校验标签列表
pub fn validate_tags(tags: &[String]) -> Result<(), String> {
    if tags.len() > 20 {
        return Err("标签数量超限 (最大20)".into());
    }
    for tag in tags {
        if tag.len() > 64 || tag.contains(',') || tag.contains(';') {
            return Err(format!("标签不合法: {}", tag));
        }
    }
    Ok(())
}

// ─── 数据完整性 ──────────────────────────────────────────────────

/// 简单校验和 (用于状态文件完整性验证)
pub fn compute_checksum(data: &str) -> String {
    let mut hash: u64 = 0x1505;
    for b in data.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u64);
    }
    format!("{:016x}", hash)
}

/// 验证状态版本兼容性
pub fn validate_state_version(current: u32, stored: u32) -> Result<(), String> {
    if stored > current {
        return Err(format!("状态文件版本过高: {} > {} (可能需要升级Chronos-Shadow)", stored, current));
    }
    Ok(())
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path_safe("../etc/passwd").is_err());
        assert!(validate_path_safe("normal/path/file.txt").is_ok());
    }

    #[test]
    fn test_validate_id() {
        assert!(validate_id("valid-id_123").is_ok());
        assert!(validate_id("invalid;id").is_err());
        assert!(validate_id("").is_err());
    }

    #[test]
    fn test_validate_cost() {
        assert!(validate_cost(5.0).is_ok());
        assert!(validate_cost(-1.0).is_err());
        assert!(validate_cost(f64::NAN).is_err());
    }

    #[test]
    fn test_validate_risk() {
        assert!(validate_risk_score(5).is_ok());
        assert!(validate_risk_score(11).is_err());
    }

    #[test]
    fn test_checksum_consistency() {
        let cs1 = compute_checksum("hello");
        let cs2 = compute_checksum("hello");
        assert_eq!(cs1, cs2);
        assert_ne!(cs1, compute_checksum("world"));
    }
}
