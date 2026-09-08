// src/views/chat/ContextPressureMeter.tsx — 上下文压力面板（语义化直观版）
// 设计原则：一眼看懂「还能聊多少」，而不是让用户解读 token 百分比。
// 结构：状态词 + 剩余可输入估算 → 分段计量条（触发线/红线标注）→
//       泄压事件时间线 → 压力趋势 → 消息占用 Top 6 → 逃生按钮。
import { useMemo } from "react";
import { Gauge, Scissors, TrendingUp, FileWarning, Sparkles } from "lucide-react";
import { useLang } from "@/lib/i18n-context";
import type { PruneStats } from "@/lib/tauri";
import type { Message } from "@/lib/types";

/** 与 Rust estimate_tokens 一致的启发式（ascii 0.25 / 非 ascii 0.6） */
// oxlint-disable-next-line react/only-export-components -- 仅组件内部使用
function estimateTokensFrontend(text: string): number {
  let t = 0;
  for (const c of text) t += c.charCodeAt(0) < 128 ? 0.25 : 0.6;
  return Math.ceil(t);
}

interface ContextPressureMeterProps {
  pressure: number | null;
  history: number[];
  lastStats: PruneStats | null;
  messages: Message[];
  onClose: () => void;
  onNewSession: () => void;
}

interface Zone {
  label: string; cls: string; bar: string; ring: string;
}

function zoneOf(p: number, zh: boolean): Zone {
  if (p >= 0.95) return { label: zh ? "已达红线" : "Red line", cls: "text-red-400", bar: "bg-red-500", ring: "border-red-500/40" };
  if (p >= 0.8) return { label: zh ? "接近上限" : "Near limit", cls: "text-amber-400", bar: "bg-amber-400", ring: "border-amber-500/40" };
  if (p >= 0.5) return { label: zh ? "正常" : "Normal", cls: "text-cyan-400", bar: "bg-cyan-400", ring: "border-cyan-500/30" };
  return { label: zh ? "充足" : "Plenty", cls: "text-emerald-400", bar: "bg-emerald-500", ring: "border-emerald-500/30" };
}

export default function ContextPressureMeter({
  pressure, history, lastStats, messages, onClose, onNewSession,
}: ContextPressureMeterProps) {
  const { lang } = useLang();
  const zh = lang === "zh";
  const p = pressure ?? 0;
  const zone = zoneOf(p, zh);

  // 活动窗口（后端按模型覆盖后的值）与剩余可输入估算
  const windowTokens = lastStats?.active_window_tokens || lastStats?.context_window_tokens || 65536;
  const remainTokens = Math.max(0, Math.floor(windowTokens * (1 - p)));
  // 中文权重 0.6 token/字 → 1 token ≈ 1.67 字；保守给「万」级估算
  const remainCharsZh = Math.round(remainTokens / 0.6);
  const remainWan = (remainCharsZh / 10000).toFixed(1);

  // 触发线在计量条上的位置（窗口占比 80% 处；ratio 自适应）
  const triggerPos = Math.min(100, (lastStats?.compact_ratio ?? 0.8) * 100);

  // 按消息 token 分布 Top 6
  const distribution = useMemo(
    () => messages
      .map((m) => ({ id: m.id, sender: m.sender, tokens: estimateTokensFrontend(m.content) }))
      .sort((a, b) => b.tokens - a.tokens)
      .slice(0, 6),
    [messages],
  );
  const maxDist = distribution[0]?.tokens || 1;
  const trend = history.slice(-12);

  // 泄压事件描述（一句话，替代数字表格）
  const reliefLine = lastStats && lastStats.stage_reached !== "None"
    ? (zh
        ? `最近泄压：剪枝 ${lastStats.tool_results_pruned} 条工具输出 · 移除 ${lastStats.messages_dropped} 条旧消息 · 节省 ${lastStats.chars_saved.toLocaleString()} 字符`
        : `Last relief: pruned ${lastStats.tool_results_pruned} tool outputs · dropped ${lastStats.messages_dropped} old messages · saved ${lastStats.chars_saved.toLocaleString()} chars`)
    : null;

  return (
    <div
      className="absolute bottom-full right-2 mb-2 w-[380px] rounded-xl border border-cs-border bg-cs-surface shadow-2xl z-50 overflow-hidden animate-fadeIn"
      role="dialog"
      aria-label={zh ? "上下文压力面板" : "Context pressure panel"}
    >
      {/* ── 头部：状态词 + 关闭 ── */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-cs-border bg-cs-header">
        <span className="text-[11px] font-bold text-zinc-300 flex items-center gap-1.5">
          <Gauge size={13} className="text-cyan-400" aria-hidden="true" />
          {zh ? "上下文用量" : "Context Usage"}
        </span>
        <button onClick={onClose} aria-label={zh ? "关闭" : "Close"}
          className="w-5 h-5 flex items-center justify-center rounded text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors">✕</button>
      </div>

      <div className="p-3 space-y-3">
        {/* ── 一眼看懂：状态词 + 剩余可输入估算 ── */}
        <div className="flex items-center justify-between">
          <div>
            <div className={`text-xl font-bold leading-tight ${zone.cls}`}>{zone.label}</div>
            <div className="text-[10px] text-zinc-500">
              {zh
                ? <>已用 {Math.round(p * 100)}% · <b className="text-zinc-300">约还能输入 {remainWan} 万字</b></>
                : <>Used {Math.round(p * 100)}% · <b className="text-zinc-300">~{remainWan}0k chars left</b></>}
            </div>
          </div>
          <div className="text-right">
            <div className="text-[9px] text-zinc-500">{zh ? "触发线" : "Trigger"}</div>
            <div className="text-[11px] font-bold text-zinc-300">{Math.round(p * 100)}%</div>
          </div>
        </div>

        {/* ── 分段计量条（触发线 + 红线标注在条上） ── */}
        <div className="relative h-3.5 w-full rounded-full overflow-hidden flex" aria-hidden="true">
          <div className="h-full bg-emerald-900/50" style={{ width: "80%" }} />
          <div className="h-full bg-amber-900/50" style={{ width: "15%" }} />
          <div className="h-full bg-red-900/50" style={{ width: "5%" }} />
          <div className={`absolute inset-y-0 left-0 ${zone.bar} opacity-75 transition-all`} style={{ width: `${Math.min(100, p * 100)}%` }} />
          <div className="absolute inset-y-0 w-0.5 bg-zinc-200/90" style={{ left: `${triggerPos}%` }} title={zh ? "泄压触发线" : "Relief trigger"} />
        </div>
        <div className="flex justify-between text-[8px] text-zinc-600">
          <span>0</span>
          <span className="text-zinc-400">↑ {zh ? "自动泄压" : "auto-relief"} ({triggerPos.toFixed(0)}%)</span>
          <span>100%</span>
        </div>

        {/* ── 泄压事件（一句话时间线） ── */}
        {reliefLine && (
          <div className="flex items-start gap-1.5 text-[10px] text-zinc-400 bg-cs-header/60 border border-cs-border rounded-lg px-2 py-1.5">
            <Scissors size={10} className="mt-0.5 shrink-0 text-cyan-400" aria-hidden="true" />
            <span>{reliefLine}</span>
          </div>
        )}

        {/* ── 压力趋势 ── */}
        {trend.length > 1 && (
          <div>
            <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
              <TrendingUp size={10} aria-hidden="true" />
              {zh ? "压力趋势" : "Trend"}
            </div>
            <div className="flex items-end gap-1 h-9" aria-hidden="true">
              {trend.map((v, i) => {
                const zz = zoneOf(v, zh);
                return (
                  <div key={i} className="flex-1 flex flex-col justify-end h-full">
                    <div className={`rounded-sm ${zz.bar}`} style={{ height: `${Math.max(10, v * 100)}%` }} title={`${Math.round(v * 100)}%`} />
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* ── 消息占用 Top 6 ── */}
        {distribution.length > 0 && (
          <div>
            <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
              <FileWarning size={10} aria-hidden="true" />
              {zh ? "占用最大的消息" : "Largest messages"}
            </div>
            <div className="space-y-1">
              {distribution.map((d) => (
                <div key={d.id} className="flex items-center gap-1.5 text-[9px]">
                  <span className="w-14 text-zinc-500 truncate">{d.sender}</span>
                  <div className="flex-1 h-2 bg-zinc-900 rounded-sm overflow-hidden" aria-hidden="true">
                    <div className="h-full bg-gradient-to-r from-cyan-600 to-cyan-400" style={{ width: `${(d.tokens / maxDist) * 100}%` }} />
                  </div>
                  <span className="w-16 text-right text-zinc-400 font-mono">{d.tokens.toLocaleString()}t</span>
                </div>
              ))}
            </div>
            <div className="text-[8px] text-zinc-600 mt-1">
              {zh ? "占用最大的消息是滑窗截断时的首要剪枝对象" : "Largest messages are pruned first by sliding-window truncation"}
            </div>
          </div>
        )}

        {/* ── 逃生按钮（红线） ── */}
        {p >= 0.95 && (
          <button
            onClick={onNewSession}
            className="w-full bg-red-500/80 hover:bg-red-500 text-white text-[10px] font-bold py-1.5 rounded transition-colors flex items-center justify-center gap-1"
          >
            <Sparkles size={11} aria-hidden="true" />
            {zh ? "上下文已达红线 — 新建会话" : "Red line reached — start a new session"}
          </button>
        )}
      </div>
    </div>
  );
}
