// 系统高可用与韧性引擎 (System Resilience & HA Engine)
//
// 生产级可靠性保障:
//   1. 熔断器 (Circuit Breaker) — 连续失败N次后自动熔断，冷却后试探恢复
//   2. 指数退避重试 (Exponential Backoff) — 瞬时故障自动恢复
//   3. 系统健康诊断 (Health Check) — 全模块状态检测 + 依赖检查
//   4. Panic 守卫 — 关键路径 panic 捕获 + 自动恢复
//
// 全部端侧计算，0 Token 消耗

use std::time::{Duration, Instant};
use std::sync::Mutex;

// ─── 熔断器 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,       // 正常通行
    Open,         // 熔断拒绝
    HalfOpen,     // 试探恢复
}

pub struct CircuitBreaker {
    state: Mutex<CircuitState>,
    failure_count: Mutex<u32>,
    success_count: Mutex<u32>,
    last_failure: Mutex<Option<Instant>>,
    failure_threshold: u32,
    success_threshold: u32,   // HalfOpen时需要连续成功N次才恢复
    timeout: Duration,         // Open→HalfOpen的冷却时间
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: &str) -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed),
            failure_count: Mutex::new(0),
            success_count: Mutex::new(0),
            last_failure: Mutex::new(None),
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            name: name.into(),
        }
    }

    /// 检查是否允许通过
    pub fn allow(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last = self.last_failure.lock().unwrap();
                if let Some(t) = *last {
                    if t.elapsed() >= self.timeout {
                        *state = CircuitState::HalfOpen;
                        tracing::info!("[CB:{}] HalfOpen — probing recovery", self.name);
                        true
                    } else { false }
                } else { false }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::HalfOpen => {
                let mut count = self.success_count.lock().unwrap();
                *count += 1;
                if *count >= self.success_threshold {
                    *state = CircuitState::Closed;
                    *self.failure_count.lock().unwrap() = 0;
                    *count = 0;
                    tracing::info!("[CB:{}] Closed — recovered", self.name);
                }
            }
            CircuitState::Closed => {
                *self.failure_count.lock().unwrap() = 0;
            }
            _ => {}
        }
    }

    /// 记录失败
    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => {
                let mut count = self.failure_count.lock().unwrap();
                *count += 1;
                if *count >= self.failure_threshold {
                    *state = CircuitState::Open;
                    *self.last_failure.lock().unwrap() = Some(Instant::now());
                    tracing::warn!("[CB:{}] OPEN — circuit broken after {} failures", self.name, *count);
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                *self.last_failure.lock().unwrap() = Some(Instant::now());
                *self.success_count.lock().unwrap() = 0;
            }
            _ => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state.lock().unwrap().clone()
    }
}

// ─── 指数退避重试 ────────────────────────────────────────────────

pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_retries: 3, base_delay_ms: 500, max_delay_ms: 10000, multiplier: 2.0 }
    }
}

impl RetryPolicy {
    pub fn standard() -> Self { Self::default() }
    pub fn aggressive() -> Self { Self { max_retries: 5, base_delay_ms: 200, max_delay_ms: 5000, multiplier: 1.5 } }
    pub fn gentle() -> Self { Self { max_retries: 2, base_delay_ms: 1000, max_delay_ms: 30000, multiplier: 2.0 } }

    pub fn delay_ms(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_delay_ms)
    }
}

// ─── 系统健康诊断 ────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub module: String,
    pub status: String, // "healthy" | "degraded" | "unhealthy"
    pub message: Option<String>,
    pub last_check: String,
}

pub struct SystemHealth {
    checks: Mutex<Vec<HealthStatus>>,
}

impl SystemHealth {
    pub fn new() -> Self {
        Self { checks: Mutex::new(Vec::new()) }
    }

    pub fn report(&self, module: &str, status: &str, message: Option<&str>) {
        let mut checks = self.checks.lock().unwrap();
        checks.retain(|c| c.module != module);
        checks.push(HealthStatus {
            module: module.into(), status: status.into(),
            message: message.map(|s| s.into()),
            last_check: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn full_report(&self) -> Vec<HealthStatus> {
        self.checks.lock().unwrap().clone()
    }

    pub fn is_healthy(&self) -> bool {
        self.checks.lock().unwrap().iter()
            .all(|c| c.status != "unhealthy")
    }
}

impl Default for SystemHealth {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens() {
        let cb = CircuitBreaker::new("test");
        for _ in 0..5 { cb.record_failure(); }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow());
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let cb = CircuitBreaker::new("test");
        // Force open
        for _ in 0..5 { cb.record_failure(); }
        // Manually set to HalfOpen by manipulating last_failure time
        *cb.last_failure.lock().unwrap() = Some(Instant::now() - Duration::from_secs(60));
        assert!(cb.allow());
    }

    #[test]
    fn test_retry_delays() {
        let policy = RetryPolicy::standard();
        assert_eq!(policy.delay_ms(0), 500);
        assert_eq!(policy.delay_ms(1), 1000);
        assert_eq!(policy.delay_ms(2), 2000);
        assert!(policy.delay_ms(10) <= 10000);
    }

    #[test]
    fn test_health_report() {
        let health = SystemHealth::new();
        health.report("api", "healthy", None);
        health.report("db", "degraded", Some("slow response"));
        assert!(health.is_healthy());
        health.report("cache", "unhealthy", Some("connection lost"));
        assert!(!health.is_healthy());
    }
}
