// 端侧科学化本地分析引擎 (Local Analytics Engine)
//
// 全部端侧计算，0 Token 消耗。提供：
//   1. 滑动窗口统计 — 均值/方差/标准差
//   2. 趋势检测 — 线性回归斜率方向
//   3. 异常检测 — Z-score 离群值标记
//   4. 自适应阈值 — 基于历史数据动态调整
//   5. 频率分析 — 事件模式计数与排序
//   6. 移动平均预测 — 简单指数平滑

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// ─── 统计快照 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatSnapshot {
    pub count: u32,
    pub sum: f64,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub latest: f64,
}

impl StatSnapshot {
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self { count: 0, sum: 0.0, mean: 0.0, variance: 0.0, std_dev: 0.0, min: 0.0, max: 0.0, latest: 0.0 };
        }
        let count = values.len() as u32;
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;
        let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / count as f64;
        Self {
            count, sum, mean, variance,
            std_dev: variance.sqrt(),
            min: values.iter().cloned().fold(f64::MAX, f64::min),
            max: values.iter().cloned().fold(f64::MIN, f64::max),
            latest: *values.last().unwrap_or(&0.0),
        }
    }
}

// ─── 趋势方向 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Rising,      // ↑ 上升
    Falling,     // ↓ 下降
    Stable,      // → 平稳
    Volatile,    // ~ 波动
}

impl TrendDirection {
    pub fn emoji(&self) -> &str {
        match self { Self::Rising => "↑", Self::Falling => "↓", Self::Stable => "→", Self::Volatile => "~" }
    }
}

// ─── 异常标记 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFlag {
    pub index: usize,
    pub value: f64,
    pub z_score: f64,
    pub severity: String, // "low" | "medium" | "high"
    pub description: String,
}

// ─── 窗口指标 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowMetrics {
    pub current: StatSnapshot,
    pub trend: TrendDirection,
    pub anomalies: Vec<AnomalyFlag>,
    pub adaptive_threshold: f64,
    pub prediction_next: f64,
}

// ─── 本地分析引擎 ──────────────────────────────────────────────────

pub struct LocalAnalytics {
    /// 滑动窗口数据 (按指标名分组)
    pub windows: HashMap<String, VecDeque<f64>>,
    /// 窗口大小
    pub window_size: usize,
    /// 异常检测 Z-score 阈值
    pub anomaly_threshold: f64,
    /// 指数平滑系数 (0-1, 越大越敏感)
    pub smoothing_alpha: f64,
    /// 历史统计 (持久化)
    pub history: HashMap<String, Vec<StatSnapshot>>,
}

impl LocalAnalytics {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_size: 100,
            anomaly_threshold: 2.5,
            smoothing_alpha: 0.3,
            history: HashMap::new(),
        }
    }

    // ── 数据录入 ──────────────────────────────────────────────────

    /// 记录一个数据点到指定指标
    pub fn record(&mut self, metric: &str, value: f64) {
        let window = self.windows.entry(metric.into()).or_insert_with(|| VecDeque::with_capacity(self.window_size));
        window.push_back(value);
        while window.len() > self.window_size {
            window.pop_front();
        }
    }

    // ── 统计分析 ──────────────────────────────────────────────────

    /// 当前窗口统计快照
    pub fn snapshot(&self, metric: &str) -> StatSnapshot {
        let values: Vec<f64> = self.windows.get(metric)
            .map(|w| w.iter().cloned().collect())
            .unwrap_or_default();
        StatSnapshot::from_values(&values)
    }

    /// 所有指标快照
    pub fn all_snapshots(&self) -> HashMap<String, StatSnapshot> {
        self.windows.iter().map(|(k, _)| (k.clone(), self.snapshot(k))).collect()
    }

    // ── 趋势检测 ──────────────────────────────────────────────────

    /// 简单线性回归检测趋势方向
    pub fn detect_trend(&self, metric: &str) -> TrendDirection {
        let values: Vec<f64> = self.windows.get(metric)
            .map(|w| w.iter().cloned().collect())
            .unwrap_or_default();
        if values.len() < 5 { return TrendDirection::Stable; }

        let n = values.len() as f64;
        let mean_x = (n - 1.0) / 2.0;
        let mean_y = values.iter().sum::<f64>() / n;

        let mut num = 0.0;
        let mut den = 0.0;
        for (i, &y) in values.iter().enumerate() {
            let dx = i as f64 - mean_x;
            num += dx * (y - mean_y);
            den += dx * dx;
        }
        if den == 0.0 { return TrendDirection::Stable; }
        let slope = num / den;
        let std_dev = StatSnapshot::from_values(&values).std_dev;
        let rel_slope = slope / (mean_y.max(0.001));

        if rel_slope.abs() < 0.02 { TrendDirection::Stable }
        else if std_dev / mean_y.max(0.001) > 0.5 { TrendDirection::Volatile }
        else if slope > 0.0 { TrendDirection::Rising }
        else { TrendDirection::Falling }
    }

    // ── 异常检测 ──────────────────────────────────────────────────

    /// 稳健 Z-score (MAD) 异常检测：对离群值稳健，避免单一极端值抬高 σ 掩盖自身
    pub fn detect_anomalies(&self, metric: &str) -> Vec<AnomalyFlag> {
        let values: Vec<f64> = self.windows.get(metric)
            .map(|w| w.iter().cloned().collect())
            .unwrap_or_default();
        if values.len() < 10 { return vec![]; }

        // 中位数 + MAD（中位数绝对偏差）
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let mut abs_devs: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
        abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mad = abs_devs[abs_devs.len() / 2];

        // MAD 为 0（数据高度集中）→ 退化到普通 Z-score
        if mad == 0.0 {
            let snap = StatSnapshot::from_values(&values);
            if snap.std_dev == 0.0 { return vec![]; }
            return values.iter().enumerate()
                .filter_map(|(i, &v)| {
                    let z = (v - snap.mean).abs() / snap.std_dev;
                    if z > self.anomaly_threshold {
                        Some(AnomalyFlag {
                            index: i, value: v, z_score: z,
                            severity: if z > 4.0 { "high" } else if z > 3.0 { "medium" } else { "low" }.into(),
                            description: format!("{}: value={:.2} is {:.1}σ from mean {:.2}", metric, v, z, snap.mean),
                        })
                    } else { None }
                })
                .collect();
        }

        // 稳健 Z-score: 0.6745 = 1/Φ⁻¹(0.75)，使 MAD 与正态 σ 对齐
        values.iter().enumerate()
            .filter_map(|(i, &v)| {
                let z = (0.6745 * (v - median) / mad).abs();
                if z > self.anomaly_threshold {
                    Some(AnomalyFlag {
                        index: i, value: v, z_score: z,
                        severity: if z > 4.0 { "high" } else if z > 3.0 { "medium" } else { "low" }.into(),
                        description: format!("{}: value={:.2} is {:.1}σ (MAD) from median {:.2}", metric, v, z, median),
                    })
                } else { None }
            })
            .collect()
    }

    // ── 自适应阈值 ────────────────────────────────────────────────

    /// 基于历史窗口动态计算阈值 (mean + k*std_dev)
    pub fn adaptive_threshold(&self, metric: &str, k: f64) -> f64 {
        let snap = self.snapshot(metric);
        if snap.count < 5 { return f64::MAX; }
        snap.mean + k * snap.std_dev
    }

    /// 获取指标的完整分析窗口
    pub fn window_metrics(&self, metric: &str) -> WindowMetrics {
        let snap = self.snapshot(metric);
        let trend = self.detect_trend(metric);
        let anomalies = self.detect_anomalies(metric);
        let threshold = self.adaptive_threshold(metric, 2.0);
        let prediction = self.predict_next(metric);

        WindowMetrics { current: snap, trend, anomalies, adaptive_threshold: threshold, prediction_next: prediction }
    }

    // ── 指数平滑预测 ──────────────────────────────────────────────

    /// 简单指数平滑预测下一个值
    pub fn predict_next(&self, metric: &str) -> f64 {
        let values: Vec<f64> = self.windows.get(metric)
            .map(|w| w.iter().cloned().collect())
            .unwrap_or_default();
        if values.is_empty() { return 0.0; }
        if values.len() == 1 { return values[0]; }

        let alpha = self.smoothing_alpha;
        let mut smoothed = values[0];
        for &v in &values[1..] {
            smoothed = alpha * v + (1.0 - alpha) * smoothed;
        }
        smoothed
    }

    // ── 频率分析 ──────────────────────────────────────────────────

    /// 事件频率统计 (Top-N)
    pub fn frequency_analysis<T: AsRef<str>>(&self, events: &[T], top_n: usize) -> Vec<(String, u32, f64)> {
        let mut freq: HashMap<String, u32> = HashMap::new();
        for e in events {
            *freq.entry(e.as_ref().to_string()).or_insert(0) += 1;
        }
        let total = events.len() as f64;
        let mut sorted: Vec<_> = freq.into_iter().collect();
        sorted.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        sorted.truncate(top_n);
        sorted.into_iter().map(|(k, c)| (k, c, c as f64 / total.max(1.0))).collect()
    }

    // ── 相关性分析 ──────────────────────────────────────────────

    /// Pearson 相关系数: 度量两个指标之间的线性相关程度 (-1到1)
    pub fn pearson_correlation(&self, metric_a: &str, metric_b: &str) -> f64 {
        let a: Vec<f64> = self.windows.get(metric_a).map(|w| w.iter().cloned().collect()).unwrap_or_default();
        let b: Vec<f64> = self.windows.get(metric_b).map(|w| w.iter().cloned().collect()).unwrap_or_default();
        let n = a.len().min(b.len());
        if n < 3 { return 0.0; }

        let mean_a = a.iter().take(n).sum::<f64>() / n as f64;
        let mean_b = b.iter().take(n).sum::<f64>() / n as f64;
        let mut cov = 0.0;
        let mut var_a = 0.0;
        let mut var_b = 0.0;
        for i in 0..n {
            let da = a[i] - mean_a;
            let db = b[i] - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
        if var_a == 0.0 || var_b == 0.0 { return 0.0; }
        cov / (var_a.sqrt() * var_b.sqrt())
    }

    // ── 变化率分析 ──────────────────────────────────────────────

    /// 一阶变化率 (最近N个点的平均斜率)
    pub fn rate_of_change(&self, metric: &str, n: usize) -> f64 {
        let values: Vec<f64> = self.windows.get(metric).map(|w| w.iter().cloned().collect()).unwrap_or_default();
        if values.len() < n || n < 2 { return 0.0; }
        let recent: Vec<_> = values.iter().rev().take(n).rev().cloned().collect();
        (recent[n-1] - recent[0]) / (n - 1) as f64
    }

    /// 加速度 (二阶变化率)
    pub fn acceleration(&self, metric: &str) -> f64 {
        let values: Vec<f64> = self.windows.get(metric).map(|w| w.iter().cloned().collect()).unwrap_or_default();
        if values.len() < 5 { return 0.0; }
        let half = values.len() / 2;
        let first_half_rate = (values[half] - values[0]) / half as f64;
        let second_half_rate = (values[values.len()-1] - values[half]) / (values.len() - half) as f64;
        second_half_rate - first_half_rate
    }

    // ── 置信区间 ────────────────────────────────────────────────

    /// 95% 置信区间 (Z 分位 1.96，大样本近似)
    pub fn confidence_interval(&self, metric: &str) -> (f64, f64) {
        let snap = self.snapshot(metric);
        if snap.count < 3 { return (0.0, 0.0); }
        let margin = 1.96 * snap.std_dev / (snap.count as f64).sqrt();
        (snap.mean - margin, snap.mean + margin)
    }

    // ── 变点检测 ──────────────────────────────────────────────

    /// 滑动窗口均值偏移变点检测：检测最近的显著均值变化
    pub fn detect_change_point(&self, metric: &str) -> Option<(usize, f64, String)> {
        let values: Vec<f64> = self.windows.get(metric).map(|w| w.iter().cloned().collect()).unwrap_or_default();
        if values.len() < 10 { return None; }

        let n = values.len();
        let global_mean = values.iter().sum::<f64>() / n as f64;
        let global_var = values.iter().map(|v| (v - global_mean).powi(2)).sum::<f64>() / n as f64;

        // 滑动窗口检测最大均值偏移点
        let window = n / 3;
        let mut max_change = 0.0;
        let mut change_idx = 0;

        for i in window..(n - window) {
            let before_mean = values[..i].iter().sum::<f64>() / i as f64;
            let after_mean = values[i..].iter().sum::<f64>() / (n - i) as f64;
            let change = (after_mean - before_mean).abs();
            if change > max_change { max_change = change; change_idx = i; }
        }

        if max_change > global_var.sqrt() * 0.5 {
            let before = values[..change_idx].iter().sum::<f64>() / change_idx as f64;
            let after = values[change_idx..].iter().sum::<f64>() / (n - change_idx) as f64;
            let direction = if after > before { "↗ 上升突变" } else { "↘ 下降突变" };
            Some((change_idx, (after - before).abs(), direction.into()))
        } else { None }
    }

    // ── 综合健康评分 ────────────────────────────────────────────

    /// 多指标综合健康评分 (0-100)
    pub fn health_score(&self) -> serde_json::Value {
        let metrics = ["api_cost", "hallucination_rate", "approval_reject_rate", "agent_error_rate"];
        let mut scores = serde_json::Map::new();
        let mut total = 0.0;
        let mut count = 0;

        for m in &metrics {
            if let Some(w) = self.windows.get(*m) {
                if w.len() >= 5 {
                    let snap = self.snapshot(m);
                    let trend = self.detect_trend(m);
                    // 越低越好的指标: 成本/幻觉率/拒绝率/错误率
                    let score = if snap.mean < 0.001 { 100.0 }
                        else if trend == TrendDirection::Falling { 90.0 }
                        else if trend == TrendDirection::Stable { 70.0 }
                        else { 50.0 };
                    scores.insert(m.to_string(), serde_json::json!({
                        "score": score, "mean": snap.mean, "trend": trend.emoji()
                    }));
                    total += score;
                    count += 1;
                }
            }
        }
        let overall = if count > 0 { (total / count as f64).round() as u32 } else { 85 };
        serde_json::json!({ "overall": overall, "breakdown": scores, "metrics_tracked": count })
    }

    // ── 持久化 ──────────────────────────────────────────────────

    pub fn save_state(&self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("analytics_state.json");
        let windows: HashMap<String, Vec<f64>> = self.windows.iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect();
        let state = serde_json::json!({ "windows": windows, "history": self.history });
        std::fs::write(&path, serde_json::to_string(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("analytics_state.json");
        if !path.exists() { return Ok(()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(w) = state.get("windows").and_then(|v| v.as_object()) {
            for (k, vals) in w {
                if let Some(arr) = vals.as_array() {
                    let deq: VecDeque<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                    self.windows.insert(k.clone(), deq);
                }
            }
        }
        Ok(())
    }
}

impl Default for LocalAnalytics {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_snapshot() {
        let snap = StatSnapshot::from_values(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(snap.count, 5);
        assert!((snap.mean - 3.0).abs() < 0.01);
        assert_eq!(snap.min, 1.0);
        assert_eq!(snap.max, 5.0);
    }

    #[test]
    fn test_rising_trend() {
        let mut la = LocalAnalytics::new();
        for i in 0..20 { la.record("test", i as f64); }
        assert_eq!(la.detect_trend("test"), TrendDirection::Rising);
    }

    #[test]
    fn test_stable_trend() {
        let mut la = LocalAnalytics::new();
        for _ in 0..20 { la.record("test", 5.0); }
        assert_eq!(la.detect_trend("test"), TrendDirection::Stable);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut la = LocalAnalytics::new();
        for _ in 0..20 { la.record("test", 10.0); }
        la.record("test", 100.0); // clear outlier
        let anomalies = la.detect_anomalies("test");
        assert!(!anomalies.is_empty());
        assert!(anomalies[0].z_score > 3.0);
    }

    #[test]
    fn test_prediction() {
        let mut la = LocalAnalytics::new();
        for i in 0..10 { la.record("test", i as f64 * 1.0); }
        let pred = la.predict_next("test");
        assert!(pred > 5.0 && pred < 10.0);
    }

    #[test]
    fn test_adaptive_threshold() {
        let mut la = LocalAnalytics::new();
        for _ in 0..20 { la.record("test", 10.0); }
        let thresh = la.adaptive_threshold("test", 2.0);
        assert!((thresh - 10.0).abs() < 1.0);
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn analytics_record(state: tauri::State<crate::state::AppState>, metric: String, value: f64) -> String {
    let mut a = state.analytics.lock().unwrap();
    a.record(&metric, value);
    format!("Recorded {}={:.4}", metric, value)
}

#[tauri::command]
pub fn analytics_snapshot(state: tauri::State<crate::state::AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    let snap = a.snapshot(&metric);
    serde_json::json!({
        "metric": metric, "count": snap.count, "mean": snap.mean,
        "std_dev": snap.std_dev, "min": snap.min, "max": snap.max, "latest": snap.latest,
    })
}

#[tauri::command]
pub fn analytics_window_metrics(state: tauri::State<crate::state::AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    let wm = a.window_metrics(&metric);
    serde_json::json!({
        "metric": metric,
        "trend": wm.trend.emoji(),
        "mean": wm.current.mean,
        "std_dev": wm.current.std_dev,
        "anomaly_count": wm.anomalies.len(),
        "adaptive_threshold": wm.adaptive_threshold,
        "prediction_next": wm.prediction_next,
    })
}

#[tauri::command]
pub fn analytics_detect_anomalies(state: tauri::State<crate::state::AppState>, metric: String) -> Vec<serde_json::Value> {
    let a = state.analytics.lock().unwrap();
    a.detect_anomalies(&metric).iter().map(|anom| serde_json::json!({
        "value": anom.value, "z_score": anom.z_score,
        "severity": anom.severity, "description": anom.description,
    })).collect()
}

#[tauri::command]
pub fn analytics_correlation(state: tauri::State<crate::state::AppState>, a: String, b: String) -> serde_json::Value {
    let analytics = state.analytics.lock().unwrap();
    let r = analytics.pearson_correlation(&a, &b);
    let (ci_lo, ci_hi) = analytics.confidence_interval(&a);
    let roc = analytics.rate_of_change(&a, 5);
    serde_json::json!({
        "correlation": r, "strength": if r.abs() > 0.7 { "strong" } else if r.abs() > 0.4 { "moderate" } else { "weak" },
        "ci_95": [ci_lo, ci_hi], "rate_of_change_5": roc,
    })
}

#[tauri::command]
pub fn analytics_health_score(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    state.analytics.lock().unwrap().health_score()
}

#[tauri::command]
pub fn analytics_change_point(state: tauri::State<crate::state::AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    match a.detect_change_point(&metric) {
        Some((idx, magnitude, direction)) => serde_json::json!({
            "detected": true, "index": idx, "magnitude": magnitude, "direction": direction,
        }),
        None => serde_json::json!({ "detected": false }),
    }
}
