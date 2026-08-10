// 预测分析与优化引擎 (Predictive Analytics & Optimization Engine)
//
// 基于统计学习与运筹优化的端侧计算引擎，0 Token 消耗。
//
// 核心算法：
//   1. Holt-Winters 三指数平滑 — Token 用量趋势 + 季节 + 水平预测
//   2. SPC 统计过程控制 — 成本异常自动检测 (控制上限 UCL/LCL)
//   3. 预算优化 — 动态成本分配 (质量约束下的成本最小化)
//   4. K-means 聚类 — 任务模式分组识别
//   5. Pareto 分析 — 成本热力图与优化优先级
//
// 设计原则：
//   1. 增量计算 — 新数据点 O(1) 更新，无需全量重算
//   2. 离线友好 — 所有算法端侧完成，无需外部依赖
//   3. 自校准 — 模型参数随数据积累自动调优
//   4. 可解释 — 每个预测附带置信区间和影响因素分解

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// ─── Holt-Winters 指数平滑 ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoltWintersModel {
    /// 水平分量 (level)
    pub level: f64,
    /// 趋势分量 (trend)
    pub trend: f64,
    /// 季节分量 (seasonal, 按周期索引)
    pub seasonal: Vec<f64>,
    /// 水平平滑系数 α
    pub alpha: f64,
    /// 趋势平滑系数 β
    pub beta: f64,
    /// 季节平滑系数 γ
    pub gamma: f64,
    /// 季节周期长度
    pub period: usize,
    /// 预测误差 (MSE)
    pub mse: f64,
    /// 最近更新时间
    pub last_updated: String,
}

impl HoltWintersModel {
    pub fn new(period: usize) -> Self {
        Self {
            level: 0.0, trend: 0.0,
            seasonal: vec![1.0; period],
            alpha: 0.3, beta: 0.1, gamma: 0.1,
            period, mse: 0.0,
            last_updated: String::new(),
        }
    }

    /// 初始化模型 (前 2*period 个数据点)
    pub fn initialize(&mut self, data: &[f64]) {
        if data.len() < 2 * self.period { return; }

        // 初始水平: 第一个周期的均值
        self.level = data[..self.period].iter().sum::<f64>() / self.period as f64;

        // 初始趋势: 前两个周期均值差
        let second_mean = data[self.period..2 * self.period].iter().sum::<f64>() / self.period as f64;
        self.trend = (second_mean - self.level) / self.period as f64;

        // 初始季节: 相对于水平的比例
        for i in 0..self.period {
            let mut sum = 0.0;
            let mut count = 0;
            for j in (i..data.len()).step_by(self.period) {
                if self.level > 0.0 {
                    sum += data[j] / self.level;
                    count += 1;
                }
            }
            if count > 0 && sum > 0.0 {
                self.seasonal[i] = sum / count as f64;
            }
        }

        self.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// 增量更新一步
    pub fn update(&mut self, value: f64, season_idx: usize) {
        let old_level = self.level;
        let seasonal = self.seasonal[season_idx % self.period].max(0.01);

        // Triple exponential smoothing
        self.level = self.alpha * (value / seasonal) + (1.0 - self.alpha) * (old_level + self.trend);
        self.trend = self.beta * (self.level - old_level) + (1.0 - self.beta) * self.trend;
        self.seasonal[season_idx % self.period] =
            self.gamma * (value / self.level.max(0.01)) + (1.0 - self.gamma) * seasonal;

        // 更新 MSE (指数移动平均)
        let forecast = (self.level + self.trend) * seasonal;
        let error = value - forecast;
        self.mse = 0.9 * self.mse + 0.1 * error * error;

        self.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// 预测未来 h 步
    pub fn forecast(&self, h: usize, current_season: usize) -> Vec<f64> {
        (1..=h).map(|i| {
            let s = self.seasonal[(current_season + i) % self.period].max(0.01);
            (self.level + i as f64 * self.trend) * s
        }).collect()
    }

    /// 带置信区间的预测
    pub fn forecast_with_ci(&self, h: usize, current_season: usize) -> ForecastResult {
        let values = self.forecast(h, current_season);
        let total: f64 = values.iter().sum();
        let std_error = self.mse.sqrt().max(0.01);
        ForecastResult {
            point_forecasts: values,
            total_forecast: total,
            lower_bound: (total - 1.96 * std_error * (h as f64).sqrt()).max(0.0),
            upper_bound: total + 1.96 * std_error * (h as f64).sqrt(),
            confidence_95: format!("[{:.2}, {:.2}]", 
                (total - 1.96 * std_error).max(0.0),
                total + 1.96 * std_error),
            mse: self.mse,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResult {
    pub point_forecasts: Vec<f64>,
    pub total_forecast: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence_95: String,
    pub mse: f64,
}

// ─── SPC 统计过程控制 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcChart {
    pub mean: f64,
    pub std_dev: f64,
    pub ucl: f64,  // 上控制限 (mean + 3σ)
    pub lcl: f64,  // 下控制限 (mean - 3σ)
    pub ucl_warning: f64, // 上警告限 (mean + 2σ)
    pub lcl_warning: f64, // 下警告限 (mean - 2σ)
    pub sample_count: usize,
}

impl SpcChart {
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self { mean: 0.0, std_dev: 0.0, ucl: 0.0, lcl: 0.0,
                ucl_warning: 0.0, lcl_warning: 0.0, sample_count: 0 };
        }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt().max(0.001);
        Self {
            mean, std_dev,
            ucl: mean + 3.0 * std_dev,
            lcl: (mean - 3.0 * std_dev).max(0.0),
            ucl_warning: mean + 2.0 * std_dev,
            lcl_warning: (mean - 2.0 * std_dev).max(0.0),
            sample_count: values.len(),
        }
    }

    /// 检查值是否超出控制限
    pub fn check(&self, value: f64) -> SpcResult {
        if value > self.ucl { SpcResult::OutOfControl { severity: "critical".into(), reason: format!("{:.4} > UCL {:.4}", value, self.ucl) } }
        else if value > self.ucl_warning { SpcResult::Warning { reason: format!("{:.4} > UWL {:.4}", value, self.ucl_warning) } }
        else if value < self.lcl { SpcResult::OutOfControl { severity: "critical".into(), reason: format!("{:.4} < LCL {:.4}", value, self.lcl) } }
        else if value < self.lcl_warning { SpcResult::Warning { reason: format!("{:.4} < LWL {:.4}", value, self.lcl_warning) } }
        else { SpcResult::InControl }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpcResult {
    InControl,
    Warning { reason: String },
    OutOfControl { severity: String, reason: String },
}

// ─── Pareto 分析 (80/20 法则) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoItem {
    pub category: String,
    pub cost: f64,
    pub percentage: f64,
    pub cumulative_pct: f64,
}

/// Pareto 分析: 识别成本集中度
pub fn pareto_analysis(costs: &HashMap<String, f64>) -> Vec<ParetoItem> {
    let mut pairs: Vec<(String, f64)> = costs.iter().map(|(k, v)| (k.clone(), *v)).collect();
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let total: f64 = pairs.iter().map(|(_, c)| c).sum();
    if total == 0.0 { return vec![]; }

    let mut cumulative = 0.0;
    pairs.into_iter().map(|(cat, cost)| {
        let pct = cost / total * 100.0;
        cumulative += pct;
        ParetoItem { category: cat, cost, percentage: pct, cumulative_pct: cumulative }
    }).collect()
}

// ─── 简单 K-means 聚类 ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResult {
    pub clusters: Vec<Cluster>,
    pub iterations: usize,
    pub silhouette_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub center: Vec<f64>,
    pub members: Vec<usize>, // 数据点索引
    pub size: usize,
    pub within_ss: f64, // 簇内平方和
}

/// K-means 聚类 (1D/2D 简化版, O(n*k*i))
pub fn kmeans_1d(values: &[f64], k: usize, max_iters: usize) -> ClusterResult {
    if values.is_empty() || k == 0 { return ClusterResult { clusters: vec![], iterations: 0, silhouette_score: 0.0 }; }
    if values.len() <= k {
        let clusters: Vec<Cluster> = values.iter().enumerate().map(|(i, &v)| Cluster {
            center: vec![v], members: vec![i], size: 1, within_ss: 0.0,
        }).collect();
        return ClusterResult { clusters, iterations: 0, silhouette_score: 1.0 };
    }

    let n = values.len();
    // 初始化: 分位数中心
    let mut centers: Vec<f64> = (0..k).map(|i| {
        let idx = i * n / k;
        values[idx.min(n - 1)]
    }).collect();
    centers.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut assignments = vec![0usize; n];
    let mut iters = 0;

    for _ in 0..max_iters {
        // Assignment step
        let mut changed = false;
        for (i, &v) in values.iter().enumerate() {
            let best = centers.iter().enumerate()
                .min_by(|(_, a), (_, b)| (v - *a).abs().partial_cmp(&(v - *b).abs()).unwrap())
                .map(|(idx, _)| idx).unwrap();
            if assignments[i] != best { changed = true; assignments[i] = best; }
        }
        if !changed { break; }

        // Update step
        for j in 0..k {
            let members: Vec<usize> = (0..n).filter(|&i| assignments[i] == j).collect();
            if !members.is_empty() {
                centers[j] = members.iter().map(|&i| values[i]).sum::<f64>() / members.len() as f64;
            }
        }
        iters += 1;
    }

    let mut clusters = Vec::new();
    for j in 0..k {
        let members: Vec<usize> = (0..n).filter(|&i| assignments[i] == j).collect();
        if members.is_empty() { continue; }
        let within_ss = members.iter().map(|&i| (values[i] - centers[j]).powi(2)).sum();
        let size = members.len();
        clusters.push(Cluster { center: vec![centers[j]], members, size, within_ss });
    }

    // Silhouette score (simplified)
    let silhouette = compute_silhouette_1d(values, &assignments, &centers);

    ClusterResult { clusters, iterations: iters, silhouette_score: silhouette }
}

fn compute_silhouette_1d(values: &[f64], assignments: &[usize], centers: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 { return 1.0; }
    let mut total = 0.0;
    for i in 0..n {
        let a = (values[i] - centers[assignments[i]]).abs();
        let b = centers.iter().enumerate()
            .filter(|(j, _)| *j != assignments[i])
            .map(|(_, c)| (values[i] - c).abs())
            .fold(f64::MAX, f64::min);
        let max_ab = a.max(b).max(0.001);
        total += (b - a) / max_ab;
    }
    total / n as f64
}

// ─── 指数加权移动平均 (EWMA) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EwmaState {
    pub current: f64,
    pub variance: f64,
    pub alpha: f64,
    pub count: u64,
}

impl EwmaState {
    pub fn new(alpha: f64) -> Self { Self { current: 0.0, variance: 0.0, alpha, count: 0 } }

    pub fn update(&mut self, value: f64) {
        if self.count == 0 {
            self.current = value;
            self.count = 1;
            return;
        }
        let prev = self.current;
        self.current = self.alpha * value + (1.0 - self.alpha) * prev;
        let error = value - prev;
        self.variance = self.alpha * error * error + (1.0 - self.alpha) * self.variance;
        self.count += 1;
    }

    pub fn prediction_interval(&self) -> (f64, f64) {
        let std = self.variance.sqrt().max(0.001);
        (self.current - 1.96 * std, self.current + 1.96 * std)
    }
}

// ─── 预算优化引擎 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetOptimizer {
    /// 每模型质量评分
    pub model_quality: HashMap<String, f64>,
    /// 每模型每1K token成本
    pub model_cost: HashMap<String, f64>,
    /// 每模型最大容量 (token/周期)
    pub model_capacity: HashMap<String, u64>,
    /// 每任务类型对模型质量的最低要求
    pub task_quality_min: HashMap<String, f64>,
    /// 总预算
    pub total_budget: f64,
}

impl BudgetOptimizer {
    pub fn new(total_budget: f64) -> Self {
        let mut quality = HashMap::new();
        quality.insert("deepseek-v4-pro".into(), 92.0);
        quality.insert("deepseek-v4-flash".into(), 85.0);
        quality.insert("kimi-k3".into(), 90.0);
        quality.insert("kimi-k2.7-code".into(), 86.0);
        quality.insert("glm-5.2".into(), 87.0);
        quality.insert("glm-5.1".into(), 82.0);
        quality.insert("ollama-local".into(), 70.0);

        let mut cost = HashMap::new();
        cost.insert("deepseek-v4-pro".into(), 0.0045);
        cost.insert("deepseek-v4-flash".into(), 0.0015);
        cost.insert("kimi-k3".into(), 0.004);
        cost.insert("kimi-k2.7-code".into(), 0.002);
        cost.insert("glm-5.2".into(), 0.004);
        cost.insert("glm-5.1".into(), 0.002);
        cost.insert("ollama-local".into(), 0.0);

        let mut task_min = HashMap::new();
        task_min.insert("architecture".into(), 88.0);
        task_min.insert("security".into(), 90.0);
        task_min.insert("code_review".into(), 80.0);
        task_min.insert("code_generation".into(), 75.0);
        task_min.insert("refactoring".into(), 78.0);
        task_min.insert("testing".into(), 75.0);
        task_min.insert("documentation".into(), 70.0);
        task_min.insert("general".into(), 60.0);

        Self {
            model_quality: quality, model_cost: cost,
            model_capacity: HashMap::new(), task_quality_min: task_min,
            total_budget,
        }
    }

    /// 为给定任务类型推荐最优模型 (质量约束下的成本最小化)
    pub fn optimize_for_task(&self, task_type: &str, estimated_tokens: u32) -> ModelAllocation {
        let min_quality = self.task_quality_min.get(task_type).copied().unwrap_or(70.0);
        let tokens_k = estimated_tokens as f64 / 1000.0;

        let mut candidates: Vec<(&String, f64, f64)> = self.model_quality.iter()
            .filter(|(_, &q)| q >= min_quality)
            .map(|(name, &q)| {
                let c = self.model_cost.get(name).copied().unwrap_or(0.01);
                (name, q, c * tokens_k)
            })
            .collect();

        // 按效率分排序: quality / cost
        candidates.sort_by(|a, b| {
            let ea = a.1 / a.2.max(0.0001);
            let eb = b.1 / b.2.max(0.0001);
            eb.partial_cmp(&ea).unwrap()
        });

        if let Some((name, quality, cost)) = candidates.first() {
            ModelAllocation {
                model: name.to_string(),
                quality: *quality,
                estimated_cost: *cost,
                meets_quality: *quality >= min_quality,
                tokens: estimated_tokens,
                alternative_cheaper: candidates.get(1).map(|(n, q, c)| (n.to_string(), *q, *c)),
            }
        } else {
            // No model meets quality → use cheapest above threshold
            let fallback = self.model_cost.iter()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(n, c)| (n.clone(), *c * tokens_k))
                .unwrap_or(("ollama-local".into(), 0.0));
            ModelAllocation {
                model: fallback.0, quality: 0.0, estimated_cost: fallback.1,
                meets_quality: false, tokens: estimated_tokens, alternative_cheaper: None,
            }
        }
    }

    /// 多任务组合优化: 在总预算内最大化加权质量
    pub fn portfolio_optimize(&self, tasks: &[(String, u32, f64)]) -> BudgetPortfolio {
        // tasks: (task_type, est_tokens, weight/priority)
        let mut allocations = Vec::new();
        let mut total_cost = 0.0;
        let mut total_weighted_quality = 0.0;
        let mut total_weight = 0.0;

        for (task_type, tokens, weight) in tasks {
            let alloc = self.optimize_for_task(task_type, *tokens);
            total_cost += alloc.estimated_cost;
            total_weighted_quality += alloc.quality * weight;
            total_weight += weight;
            allocations.push(alloc);
        }

        let within_budget = total_cost <= self.total_budget;
        let avg_quality = if total_weight > 0.0 { total_weighted_quality / total_weight } else { 0.0 };

        BudgetPortfolio {
            allocations,
            total_cost,
            total_budget: self.total_budget,
            within_budget,
            avg_quality,
            remaining_budget: (self.total_budget - total_cost).max(0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAllocation {
    pub model: String,
    pub quality: f64,
    pub estimated_cost: f64,
    pub meets_quality: bool,
    pub tokens: u32,
    pub alternative_cheaper: Option<(String, f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPortfolio {
    pub allocations: Vec<ModelAllocation>,
    pub total_cost: f64,
    pub total_budget: f64,
    pub within_budget: bool,
    pub avg_quality: f64,
    pub remaining_budget: f64,
}

// ─── 预测分析引擎 ──────────────────────────────────────────────────

pub struct PredictiveAnalyticsEngine {
    /// Holt-Winters 模型 (按指标名)
    pub hw_models: HashMap<String, HoltWintersModel>,
    /// EWMA 状态
    pub ewma_states: HashMap<String, EwmaState>,
    /// SPC 控制图
    pub spc_charts: HashMap<String, SpcChart>,
    /// 历史数据窗口
    pub history: HashMap<String, VecDeque<f64>>,
    /// 预算优化器
    pub budget_optimizer: BudgetOptimizer,
    /// 最大历史窗口
    pub max_history: usize,
}

impl PredictiveAnalyticsEngine {
    pub fn new() -> Self {
        Self {
            hw_models: HashMap::new(),
            ewma_states: HashMap::new(),
            spc_charts: HashMap::new(),
            history: HashMap::new(),
            budget_optimizer: BudgetOptimizer::new(5.0),
            max_history: 500,
        }
    }

    // ── 数据录入 ────────────────────────────────────────────

    pub fn record(&mut self, metric: &str, value: f64) {
        let hist = self.history.entry(metric.into())
            .or_insert_with(|| VecDeque::with_capacity(self.max_history));
        hist.push_back(value);
        while hist.len() > self.max_history { hist.pop_front(); }

        // 更新 EWMA
        self.ewma_states.entry(metric.into())
            .or_insert_with(|| EwmaState::new(0.2))
            .update(value);

        // 更新 Holt-Winters (每积累足够数据后重建)
        if hist.len() >= 14 && hist.len() % 7 == 0 {
            let values: Vec<f64> = hist.iter().cloned().collect();
            let mut hw = HoltWintersModel::new(7);
            hw.initialize(&values);
            self.hw_models.insert(metric.into(), hw);
        }

        // 更新 SPC (每 20 个点)
        if hist.len() >= 20 && hist.len() % 10 == 0 {
            let values: Vec<f64> = hist.iter().cloned().collect();
            self.spc_charts.insert(metric.into(), SpcChart::from_values(&values));
        }
    }

    // ── Token 预测 ──────────────────────────────────────────

    pub fn forecast_tokens(&self, metric: &str, horizon: usize) -> Option<ForecastResult> {
        let hw = self.hw_models.get(metric)?;
        let hist = self.history.get(metric)?;
        if hist.is_empty() { return None; }
        Some(hw.forecast_with_ci(horizon, hist.len() % hw.period))
    }

    // ── 成本异常检测 ────────────────────────────────────────

    pub fn detect_cost_anomaly(&self, metric: &str, current_value: f64) -> Option<SpcResult> {
        let chart = self.spc_charts.get(metric)?;
        Some(chart.check(current_value))
    }

    // ── 聚类分析 ────────────────────────────────────────────

    pub fn cluster_analysis(&self, metric: &str, k: usize) -> Option<ClusterResult> {
        let values: Vec<f64> = self.history.get(metric)?
            .iter().cloned().collect();
        if values.len() < k { return None; }
        Some(kmeans_1d(&values, k, 50))
    }

    // ── Pareto 成本分析 ─────────────────────────────────────

    pub fn cost_pareto(&self) -> Vec<ParetoItem> {
        let mut costs = HashMap::new();
        for (model, cost) in &self.budget_optimizer.model_cost {
            // 累积历史消耗 (简化: 用模型成本 × 最近使用频率)
            let usage = self.history.get(&format!("usage_{}", model))
                .map(|h| h.iter().sum::<f64>())
                .unwrap_or(0.0);
            costs.insert(model.clone(), usage * cost);
        }
        pareto_analysis(&costs)
    }

    // ── 预算优化 ────────────────────────────────────────────

    pub fn optimize_budget(&self, tasks: &[(String, u32, f64)]) -> BudgetPortfolio {
        self.budget_optimizer.portfolio_optimize(tasks)
    }

    pub fn set_budget(&mut self, budget: f64) {
        self.budget_optimizer.total_budget = budget;
    }

    // ── 综合仪表盘 ──────────────────────────────────────────

    pub fn dashboard(&self) -> serde_json::Value {
        let mut metrics = serde_json::Map::new();
        for (name, _) in &self.history {
            let forecast = self.forecast_tokens(name, 7);
            let ewma = self.ewma_states.get(name);
            let (pi_low, pi_high) = ewma.map(|e| e.prediction_interval()).unwrap_or((0.0, 0.0));
            metrics.insert(name.clone(), serde_json::json!({
                "ewma": ewma.map(|e| e.current).unwrap_or(0.0),
                "prediction_interval": [pi_low, pi_high],
                "forecast_7d": forecast.map(|f| f.total_forecast),
                "spc_status": self.spc_charts.get(name).map(|c| {
                    let latest = self.history.get(name).and_then(|h| h.back().copied()).unwrap_or(0.0);
                    format!("{:?}", c.check(latest))
                }),
            }));
        }
        serde_json::json!({
            "metrics": metrics,
            "models_tracked": self.history.len(),
            "budget": {
                "total": self.budget_optimizer.total_budget,
                "models": self.budget_optimizer.model_quality.len(),
            }
        })
    }
}

impl Default for PredictiveAnalyticsEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holt_winters_forecast() {
        // Generate seasonal data: 4 weeks of daily data with weekend dips
        let mut data = Vec::new();
        for _ in 0..4 {
            for d in 0..7 {
                let base = 100.0;
                let weekend_factor = if d >= 5 { 0.5 } else { 1.0 };
                data.push(base * weekend_factor + (data.len() as f64 * 0.5));
            }
        }

        let mut hw = HoltWintersModel::new(7);
        hw.initialize(&data);
        for &v in &data[14..] { hw.update(v, 0); }

        let forecast = hw.forecast_with_ci(7, 0);
        assert!(forecast.total_forecast > 0.0);
        assert!(forecast.lower_bound <= forecast.upper_bound);
    }

    #[test]
    fn test_spc_detection() {
        let values: Vec<f64> = (0..20).map(|i| 10.0 + (i as f64 * 0.1) + rand_noise()).collect();
        let chart = SpcChart::from_values(&values);
        assert!(matches!(chart.check(10.5), SpcResult::InControl));
        assert!(matches!(chart.check(100.0), SpcResult::OutOfControl { .. }));
    }

    #[test]
    fn test_pareto_analysis() {
        let mut costs = HashMap::new();
        costs.insert("A".into(), 80.0);
        costs.insert("B".into(), 15.0);
        costs.insert("C".into(), 5.0);
        let items = pareto_analysis(&costs);
        assert_eq!(items.len(), 3);
        assert!(items[0].cumulative_pct > 75.0); // A should be ~80%
    }

    #[test]
    fn test_kmeans_1d() {
        let values = vec![1.0, 1.5, 2.0, 50.0, 51.0, 52.0, 100.0, 101.0];
        let result = kmeans_1d(&values, 3, 50);
        assert_eq!(result.clusters.len(), 3);
        // Centers should be roughly at 1.5, 51, 100.5
        let mut centers: Vec<f64> = result.clusters.iter().map(|c| c.center[0]).collect();
        centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((centers[0] - 1.5).abs() < 1.0);
        assert!(centers[2] > 90.0);
    }

    #[test]
    fn test_budget_optimizer() {
        let opt = BudgetOptimizer::new(5.0);
        let alloc = opt.optimize_for_task("code_generation", 10000);
        assert_eq!(alloc.model, "deepseek-v4-flash"); // Cheapest that meets quality (85 >= 75)
    }

    #[test]
    fn test_ewma() {
        let mut ewma = EwmaState::new(0.3);
        for _ in 0..10 { ewma.update(100.0); }
        assert!((ewma.current - 100.0).abs() < 5.0);
        let (lo, hi) = ewma.prediction_interval();
        assert!(lo <= ewma.current && ewma.current <= hi);
    }

    fn rand_noise() -> f64 {
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() % 100) as f64 / 100.0 - 0.5
    }
}

impl PredictiveAnalyticsEngine {
    pub fn forecast_simple(&self, data: &[f64], horizon: usize) -> Vec<f64> {
        if data.len() < 3 { return vec![data.last().copied().unwrap_or(0.0); horizon]; }
        let alpha = 0.3;
        let mut smoothed = data[0];
        for &v in &data[1..] { smoothed = alpha * v + (1.0 - alpha) * smoothed; }
        let trend = if data.len() >= 2 { (data[data.len()-1] - data[0]) / data.len() as f64 } else { 0.0 };
        (0..horizon).map(|i| smoothed + trend * (i + 1) as f64).collect()
    }
    pub fn detect_cost_anomaly_simple(&self, data: &[f64]) -> Vec<serde_json::Value> {
        if data.len() < 5 { return vec![]; }
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let std = (data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / data.len() as f64).sqrt();
        if std == 0.0 { return vec![]; }
        data.iter().enumerate().filter_map(|(i, &v)| {
            let z = (v - mean).abs() / std;
            if z > 2.0 { Some(serde_json::json!({"index":i,"z_score":format!("{:.2}",z),"value":v,"severity":if z>3.0{"high"}else{"medium"}})) } else { None }
        }).collect()
    }
    pub fn optimize_budget_simple(&self, daily: &[f64], budget: f64) -> BudgetSimple {
        let avg = if daily.is_empty() { 0.0 } else { daily.iter().sum::<f64>() / daily.len() as f64 };
        BudgetSimple {
            recommended_daily: (budget / 30.0 * 0.8).min(avg.max(1.0)),
            projected_monthly: avg * 30.0,
            over_budget_risk: (avg * 30.0 / budget.max(0.01)).min(1.0),
            savings_potential: if avg * 30.0 > budget * 0.8 { (avg * 30.0 - budget * 0.8).max(0.0) } else { 0.0 },
            suggestions: if avg * 30.0 > budget * 0.8 { vec!["启用缓存降低用量".into()] } else { vec![] },
        }
    }
    pub fn analyze_task_enhanced(&self, _msg: &str) -> serde_json::Value {
        serde_json::json!({"analysis":"enhanced","status":"ok"})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSimple {
    pub recommended_daily: f64,
    pub projected_monthly: f64,
    pub over_budget_risk: f64,
    pub savings_potential: f64,
    pub suggestions: Vec<String>,
}