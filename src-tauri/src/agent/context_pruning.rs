// 上下文压力管理：工具输出剪枝（Tool Result Pruning）+ 历史滑窗截断（History Truncation）
// + 压力红线自动泄压（两级：先剪枝 → 仍超压则截断旧前缀）
//
// 设计参照（Reasonix 压缩机制）：
//   触发阈值 trigger_tokens = floor(context_window × compact_ratio)
//   泄压阶段 1：剪枝超长工具输出（head + 省略标记 + tail）
//   泄压阶段 2：滑动窗口截断最旧前缀（原样保留最新 keep_recent_ratio）
//   约束：tool call 与 result 永不拆开（成对保护）；压力轮上限 max_pressure_runs
//
// 与会话持久化（session_db 分块 Commit）互不影响：本模块只处理发送给 LLM 的
// 临时 chatMessages 视图，落盘的完整历史保持原样。

use serde::{Deserialize, Serialize};

use crate::agent::distillation_engine::estimate_tokens;

/// 默认配置（可被前端按模型覆盖：如 DeepSeek 64K/128K）
pub const DEFAULT_CONTEXT_WINDOW_TOKENS: usize = 65_536;
pub const DEFAULT_COMPACT_RATIO: f64 = 0.80;
pub const DEFAULT_TOOL_RESULT_MAX_CHARS: usize = 8_192;
pub const DEFAULT_HEAD_CHARS: usize = 4_096;
pub const DEFAULT_TAIL_CHARS: usize = 1_024;
pub const DEFAULT_KEEP_RECENT_RATIO: f64 = 0.16;
pub const DEFAULT_MAX_PRESSURE_RUNS: usize = 2;

/// 单条待发送消息（与前端 chatMessages 结构对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneMessage {
    pub role: String,
    pub content: String,
    /// 标记该消息承载工具/命令输出（Action Engine 结果、超长 Coder 输出等）
    #[serde(default)]
    pub tool_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneConfig {
    pub context_window_tokens: usize,
    pub compact_ratio: f64,
    pub tool_result_max_chars: usize,
    pub head_chars: usize,
    pub tail_chars: usize,
    pub keep_recent_ratio: f64,
    pub max_pressure_runs: usize,
}

impl Default for PruneConfig {
    fn default() -> Self {
        Self {
            context_window_tokens: DEFAULT_CONTEXT_WINDOW_TOKENS,
            compact_ratio: DEFAULT_COMPACT_RATIO,
            tool_result_max_chars: DEFAULT_TOOL_RESULT_MAX_CHARS,
            head_chars: DEFAULT_HEAD_CHARS,
            tail_chars: DEFAULT_TAIL_CHARS,
            keep_recent_ratio: DEFAULT_KEEP_RECENT_RATIO,
            max_pressure_runs: DEFAULT_MAX_PRESSURE_RUNS,
        }
    }
}

impl PruneConfig {
    fn clamp(&mut self) {
        self.compact_ratio = self.compact_ratio.clamp(0.30, 0.95);
        self.keep_recent_ratio = self.keep_recent_ratio.clamp(0.05, 0.50);
        self.head_chars = self.head_chars.max(256);
        self.tail_chars = self.tail_chars.max(128);
        if self.tail_chars + self.head_chars >= self.tool_result_max_chars {
            self.tool_result_max_chars = self.head_chars + self.tail_chars + 1;
        }
        self.max_pressure_runs = self.max_pressure_runs.clamp(1, 4);
    }
    fn trigger_tokens(&self) -> usize {
        (self.context_window_tokens as f64 * self.compact_ratio).floor() as usize
    }
}

/// 泄压阶段标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruneStage {
    /// 未触发（低于红线）
    None,
    /// 仅执行了工具输出剪枝
    ToolPruned,
    /// 工具剪枝后仍超压，执行了滑动窗口截断
    HistoryTruncated,
    /// 达到压力轮上限仍超压（返回给前端提示用户新开会话）
    PressureEscalation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStats {
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub trigger_tokens: usize,
    pub tool_results_pruned: usize,
    pub messages_dropped: usize,
    pub chars_saved: usize,
    pub stage_reached: PruneStage,
    pub pressure_ratio: f64,
    pub context_window_tokens: usize,
    pub compact_ratio: f64,
    pub active_window_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneResult {
    pub messages: Vec<PruneMessage>,
    pub stats: PruneStats,
}

/// 判定一条消息是否按"工具输出"处理：
/// 显式标记，或 Action Engine / 系统长输出（工具回显）。
fn is_tool_result(msg: &PruneMessage) -> bool {
    msg.tool_result
        || msg.role != "user" && (msg.content.contains("[Action Engine]") || msg.content.contains("[Action Results]"))
}

/// 工具输出剪枝：超长内容 → head + 省略标记 + tail
fn prune_tool_result(content: &str, cfg: &PruneConfig) -> (String, bool) {
    let char_count = content.chars().count();
    if char_count <= cfg.tool_result_max_chars {
        return (content.to_string(), false);
    }
    let head: String = content.chars().take(cfg.head_chars).collect();
    let tail_start = char_count.saturating_sub(cfg.tail_chars);
    let tail: String = content.chars().skip(tail_start).collect();
    let omitted = char_count - cfg.head_chars - cfg.tail_chars;
    (
        format!(
            "{head}\n[... 已剪枝 {omitted} 字符的中间工具输出（完整内容已随会话分块归档于 Chronos Vault）...]\n{tail}"
        ),
        true,
    )
}

/// 消息总 token 估算
fn total_tokens(messages: &[PruneMessage]) -> usize {
    messages.iter().map(|m| estimate_tokens(&m.content)).sum()
}

/// 滑动窗口截断：保留最新 keep_recent_ratio 预算内的消息；
/// 删除边界对齐到"最旧保留消息若为 assistant 则连同其前导 user 一起保留"（成对保护）。
/// 首条 System 消息（会话开场）永不删除。
fn truncate_history(messages: &[PruneMessage], cfg: &PruneConfig) -> (Vec<PruneMessage>, usize) {
    if messages.is_empty() {
        return (messages.to_vec(), 0);
    }
    let keep_budget = (cfg.trigger_tokens() as f64 * cfg.keep_recent_ratio) as usize;
    // 从最新往回累计
    let mut take = 0usize;
    let mut acc = 0usize;
    for msg in messages.iter().rev() {
        acc += estimate_tokens(&msg.content);
        take += 1;
        if acc >= keep_budget {
            break;
        }
    }
    let cut = messages.len().saturating_sub(take);
    // 成对保护：cut 落在 assistant 上则回退到其前导 user
    let mut cut = cut;
    while cut > 0 && messages[cut].role == "assistant" {
        cut -= 1;
    }
    // 首条 System 永不删除
    if messages[0].role == "system" && cut == 0 {
        return (messages.to_vec(), 0);
    }
    let mut out: Vec<PruneMessage> = Vec::with_capacity(messages.len() - cut);
    if messages[0].role == "system" {
        out.push(messages[0].clone());
    }
    out.extend_from_slice(&messages[cut.max(if messages[0].role == "system" { 1 } else { 0 })..]);
    let dropped = messages.len() - out.len();
    (out, dropped)
}

/// 压力红线泄压主入口：检查 → 阶段 1 剪枝 → 阶段 2 截断（最多 max_pressure_runs 轮）
pub fn apply_context_pruning(messages: &[PruneMessage], cfg: &PruneConfig) -> PruneResult {
    let mut cfg = cfg.clone();
    cfg.clamp();

    let tokens_before = total_tokens(messages);
    let trigger = cfg.trigger_tokens();
    let chars_before: usize = messages.iter().map(|m| m.content.chars().count()).sum();

    if tokens_before < trigger {
        return PruneResult {
            messages: messages.to_vec(),
            stats: PruneStats {
                tokens_before,
                tokens_after: tokens_before,
                trigger_tokens: trigger,
                tool_results_pruned: 0,
                messages_dropped: 0,
                chars_saved: 0,
                stage_reached: PruneStage::None,
                pressure_ratio: if cfg.context_window_tokens == 0 {
                    0.0
                } else {
                    tokens_before as f64 / cfg.context_window_tokens as f64
                },
                context_window_tokens: cfg.context_window_tokens,
                compact_ratio: cfg.compact_ratio,
                active_window_tokens: cfg.context_window_tokens,
            },
        };
    }

    let mut current: Vec<PruneMessage> = messages.to_vec();
    let mut stage = PruneStage::ToolPruned;
    let mut tool_pruned_total = 0usize;
    let mut dropped_total = 0usize;

    for _run in 0..cfg.max_pressure_runs {
        // ── 阶段 1：工具输出剪枝 ──
        let mut pruned_now = 0usize;
        for msg in current.iter_mut() {
            if is_tool_result(msg) {
                let (new_content, did) = prune_tool_result(&msg.content, &cfg);
                if did {
                    msg.content = new_content;
                    pruned_now += 1;
                }
            }
        }
        tool_pruned_total += pruned_now;
        if pruned_now > 0 {
            stage = PruneStage::ToolPruned;
        }

        if total_tokens(&current) < trigger {
            break;
        }

        // ── 阶段 2：滑动窗口截断 ──
        let (truncated, dropped) = truncate_history(&current, &cfg);
        if dropped == 0 {
            break; // 无可删（全部在保护区），避免死循环
        }
        dropped_total += dropped;
        stage = PruneStage::HistoryTruncated;
        current = truncated;

        if total_tokens(&current) < trigger {
            break;
        }
    }

    let tokens_after = total_tokens(&current);
    let chars_after: usize = current.iter().map(|m| m.content.chars().count()).sum();
    if tokens_after >= trigger {
        stage = PruneStage::PressureEscalation;
    }

    PruneResult {
        messages: current,
        stats: PruneStats {
            tokens_before,
            tokens_after,
            trigger_tokens: trigger,
            tool_results_pruned: tool_pruned_total,
            messages_dropped: dropped_total,
            chars_saved: chars_before.saturating_sub(chars_after),
            stage_reached: stage,
            pressure_ratio: if cfg.context_window_tokens == 0 {
                0.0
            } else {
                tokens_after as f64 / cfg.context_window_tokens as f64
            },
            context_window_tokens: cfg.context_window_tokens,
            compact_ratio: cfg.compact_ratio,
            active_window_tokens: cfg.context_window_tokens,
        },
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

// ─── Tauri Command ───────────────────────────────────────────────

/// 对发送给 LLM 的 chatMessages 应用压力泄压（工具剪枝 + 滑窗截断）
/// 配置读取自 AppSettings（SettingsPanel「上下文管理」分区），按 selected_model 应用窗口覆盖
#[tauri::command]
pub async fn context_prune_apply(
    state: tauri::State<'_, crate::state::AppState>,
    selected_model: Option<String>,
    messages: Vec<PruneMessage>,
) -> Result<PruneResult, String> {
    let _ = &state; // AppState 预留（泄压为纯函数，无共享态依赖）
    let s = crate::agent::settings::ensure_settings_loaded();
    let window = selected_model
        .as_ref()
        .and_then(|m| s.context_window_overrides.get(m).copied())
        .unwrap_or(s.context_window_tokens) as usize;
    let cfg = PruneConfig {
        context_window_tokens: window,
        compact_ratio: s.context_compact_ratio,
        ..Default::default()
    };
    Ok(apply_context_pruning(&messages, &cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_strict() -> PruneConfig {
        PruneConfig {
            context_window_tokens: 2_000,
            compact_ratio: 0.5,
            tool_result_max_chars: 1_000,
            head_chars: 300,
            tail_chars: 100,
            keep_recent_ratio: 0.25,
            max_pressure_runs: 2,
        }
    }

    fn user(t: impl Into<String>) -> PruneMessage {
        PruneMessage { role: "user".into(), content: t.into(), tool_result: false }
    }
    fn assistant(t: &str) -> PruneMessage {
        PruneMessage { role: "assistant".into(), content: t.into(), tool_result: false }
    }
    fn tool_out(t: impl Into<String>) -> PruneMessage {
        PruneMessage { role: "assistant".into(), content: t.into(), tool_result: true }
    }

    fn long_text(n: usize) -> String {
        "x".repeat(n)
    }

    #[test]
    fn below_threshold_is_noop() {
        let msgs = vec![user("hi"), assistant("hello")];
        let r = apply_context_pruning(&msgs, &PruneConfig::default());
        assert_eq!(r.stats.stage_reached, PruneStage::None);
        assert_eq!(r.messages.len(), 2);
        assert_eq!(r.stats.tool_results_pruned, 0);
    }

    #[test]
    fn long_tool_output_gets_pruned_with_marker() {
        let msgs = vec![
            user("run tests"),
            tool_out(long_text(5_000)),
        ];
        let r = apply_context_pruning(&msgs, &cfg_strict());
        let pruned = &r.messages[1];
        assert!(pruned.content.contains("已剪枝"));
        assert!(pruned.content.chars().count() <= 1_000 + 200); // head+tail+标记
        assert_eq!(r.stats.tool_results_pruned, 1);
    }

    #[test]
    fn short_tool_output_untouched() {
        let msgs = vec![user("q"), tool_out("short output"), assistant("ok")];
        let r = apply_context_pruning(&msgs, &cfg_strict());
        assert_eq!(r.messages[1].content, "short output");
    }

    #[test]
    fn truncation_drops_oldest_and_keeps_pair_boundary() {
        // 构造大量旧消息 + 少量新消息
        let mut msgs = vec![user("system intro")];
        for i in 0..40 {
            msgs.push(user(&format!("old question {i} {}", long_text(80))));
            msgs.push(assistant(&format!("old answer {i} {}", long_text(80))));
        }
        msgs.push(user("recent question"));
        msgs.push(assistant("recent answer"));
        let r = apply_context_pruning(&msgs, &cfg_strict());
        assert_eq!(r.stats.stage_reached, PruneStage::HistoryTruncated);
        assert!(r.stats.messages_dropped > 0);
        // 最新两条必须保留（保护区）
        let last = r.messages.last().unwrap();
        assert_eq!(last.content, "recent answer");
        // 成对保护：删除边界后首条非 system 消息不应是 assistant
        let first_non_sys = r.messages.iter().find(|m| m.role != "system").unwrap();
        assert_eq!(first_non_sys.role, "user");
    }

    #[test]
    fn system_intro_is_never_dropped() {
        let mut msgs = vec![user("intro")];
        for i in 0..30 {
            msgs.push(user(&format!("q{i} {}", long_text(100))));
        }
        msgs[0] = PruneMessage {
            role: "system".into(),
            content: "session preamble".into(),
            tool_result: false,
        };
        let r = apply_context_pruning(&msgs, &cfg_strict());
        assert_eq!(r.messages[0].content, "session preamble");
    }

    #[test]
    fn idempotent_when_already_pruned() {
        let mut msgs = vec![user("run"), tool_out(long_text(5_000))];
        let r1 = apply_context_pruning(&msgs, &cfg_strict());
        msgs = r1.messages.clone();
        let r2 = apply_context_pruning(&msgs, &cfg_strict());
        assert_eq!(r2.stats.tool_results_pruned, 0, "已剪枝内容不应再次剪枝");
        assert_eq!(r2.messages.len(), r1.messages.len());
    }

    #[test]
    fn pressure_escalation_when_nothing_can_be_dropped() {
        // 单条超巨消息：无法删除（唯一消息），剪枝受限 → 上报 PressureEscalation
        let msgs = vec![user(long_text(50_000))];
        let r = apply_context_pruning(&msgs, &cfg_strict());
        // 单条 user 消息不是 tool_result，也不可删 → 升级
        assert_eq!(r.stats.stage_reached, PruneStage::PressureEscalation);
    }

    #[test]
    fn config_clamped_to_sane_ranges() {
        let mut cfg = PruneConfig {
            compact_ratio: 5.0,
            keep_recent_ratio: 0.01,
            ..Default::default()
        };
        cfg.clamp();
        assert!(cfg.compact_ratio >= 0.30 && cfg.compact_ratio <= 0.95);
        assert!(cfg.keep_recent_ratio >= 0.05);
    }

    #[test]
    fn estimate_tokens_prefers_cjk_weight() {
        assert!(estimate_tokens("中文中文") > estimate_tokens("abcd"));
    }
}
