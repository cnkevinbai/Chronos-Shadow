// src/views/chat/ContextPressureMeter.tsx — 上下文压力可视化面板（自 ChatPanel 拆出）
// 泄压阈值可视化（分段计量条 + 触发线指针）+ 用量分析（泄压统计/压力趋势/消息 token 分布）
import { useMemo } from "react";
import { Gauge, Scissors, TrendingUp, FileWarning } from "lucide-react";
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

const ZONE = (p: number) =>
  p >= 0.95 ? { label: "红线", cls: "text-red-400", bar: "bg-red-500" }
  : p >= 0.8 ? { label: "触发", cls: "text-amber-400", bar: "bg-amber-400" }
  : { label: "安全", cls: "text-emerald-400", bar: "bg-emerald-500" };

export default function ContextPressureMeter({
  pressure, history, lastStats, messages, onClose, onNewSession,
}: ContextPressureMeterProps) {
  const { lang } = useLang();
  const zh = lang === "zh";
  const p = pressure ?? 0;
  const zone = ZONE(p);

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
  const trendMax = Math.max(0.8, ...trend);

  return (
    <div
      className="absolute bottom-full right-2 mb-2 w-[360px] rounded-xl border border-cs-border bg-cs-surface shadow-2xl z-40 overflow-hidden animate-fadeIn"
      role="dialog"
      aria-label={zh ? "上下文压力面板" : "Context pressure panel"}
    >
      {/* ── 标题 ── */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-cs-border bg-cs-header">
        <span className="text-[11px] font-bold text-zinc-300 flex items-center gap-1.5">
          <Gauge size={13} className="text-cyan-400" aria-hidden="true" />
          {zh ? "上下文压力" : "Context Pressure"}
        </span>
        <button onClick={onClose} aria-label={zh ? "关闭" : "Close"}
          className="w-5 h-5 flex items-center justify-center rounded text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors">✕</button>
      </div>

      <div className="p-3 space-y-3">
        {/* ── 压力大数字 + 分段计量条 ── */}
        <div className="flex items-end justify-between">
          <div>
            <div className={`text-2xl font-bold ${zone.cls}`}>{Math.round(p * 100)}%</div>
            <div className="text-[9px] text-zinc-500">{zh ? "当前上下文占用" : "Context utilization"}</div>
          </div>
          <span className={`text-[9px] px-1.5 py-0.5 rounded border ${zone.cls} ${
            p >= 0.95 ? "border-red-500/30 bg-red-950/20" : p >= 0.8 ? "border-amber-500/30 bg-amber-950/20" : "border-emerald-500/30 bg-emerald-950/20"
          }`}>{zone.label}</span>
        </div>

        {/* 分段计量条：0-80 绿 / 80-95 琥珀 / 95-100 红 */}
        <div className="relative h-3 w-full rounded-full overflow-hidden flex" aria-hidden="true">
          <div className="h-full bg-emerald-900/60" style={{ width: "80%" }} />
          <div className="h-full bg-amber-900/60" style={{ width: "15%" }} />
          <div className="h-full bg-red-900/60" style={{ width: "5%" }} />
          {/* 当前压力填充 */}
          <div className={`absolute inset-y-0 left-0 ${zone.bar} opacity-70`} style={{ width: `${Math.min(100, p * 100)}%` }} />
          {/* 触发线（80%） */}
          <div className="absolute inset-y-0 w-0.5 bg-zinc-300/80" style={{ left: "80%" }} />
        </div>
        <div className="flex justify-between text-[8px] text-zinc-600">
          <span>0%</span>
          <span className="text-zinc-400">{zh ? "触发 80%" : "trigger 80%"}</span>
          <span>{zh ? "红线 95%" : "red line 95%"}</span>
          <span>100%</span>
        </div>

        {/* ── 泄压统计（最近一次） ── */}
        {lastStats && lastStats.stage_reached !== "None" && (
          <div className="border border-cs-border rounded-lg p-2.5 space-y-1.5 bg-cs-header/60">
            <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1">
              <Scissors size={10} aria-hidden="true" />
              {zh ? "最近一次泄压" : "Last relief run"}
              <span className={`ml-1 px-1 rounded ${
                lastStats.stage_reached === "PressureEscalation" ? "bg-red-950/40 text-red-400"
                : lastStats.stage_reached === "HistoryTruncated" ? "bg-amber-950/40 text-amber-400"
                : "bg-cyan-950/40 text-cyan-400"}`}>
                {lastStats.stage_reached === "PressureEscalation" ? (zh ? "升级" : "Escalated")
                : lastStats.stage_reached === "HistoryTruncated" ? (zh ? "已截断" : "Truncated")
                : (zh ? "已剪枝" : "Pruned")}
              </span>
            </div>
            <div className="grid grid-cols-2 gap-1.5 text-[9px]">
              <div className="flex justify-between"><span className="text-zinc-500">{zh ? "泄压前" : "Before"}</span><b className="text-zinc-300">{lastStats.tokens_before.toLocaleString()}t</b></div>
              <div className="flex justify-between"><span className="text-zinc-500">{zh ? "泄压后" : "After"}</span><b className="text-emerald-400">{lastStats.tokens_after.toLocaleString()}t</b></div>
              <div className="flex justify-between"><span className="text-zinc-500">{zh ? "工具输出剪枝" : "Tool pruned"}</span><b className="text-zinc-300">{lastStats.tool_results_pruned}</b></div>
              <div className="flex justify-between"><span className="text-zinc-500">{zh ? "消息删除" : "Dropped"}</span><b className="text-zinc-300">{lastStats.messages_dropped}</b></div>
              <div className="flex justify-between col-span-2"><span className="text-zinc-500">{zh ? "字符节省" : "Chars saved"}</span><b className="text-emerald-400">{lastStats.chars_saved.toLocaleString()}</b></div>
            </div>
          </div>
        )}

        {/* ── 压力趋势（最近 12 次发送） ── */}
        {trend.length > 0 && (
          <div>
            <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
              <TrendingUp size={10} aria-hidden="true" />
              {zh ? "压力趋势（最近发送）" : "Pressure trend"}
            </div>
            <div className="flex items-end gap-1 h-10" aria-hidden="true">
              {trend.map((v, i) => (
                <div key={i} className="flex-1 flex flex-col justify-end h-full">
                  <div
                    className={`rounded-sm ${v >= 0.95 ? "bg-red-500" : v >= 0.8 ? "bg-amber-400" : "bg-emerald-500"}`}
                    style={{ height: `${Math.max(8, (v / trendMax) * 100)}%` }}
                    title={`${Math.round(v * 100)}%`}
                  />
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── 消息 token 分布 Top 6 ── */}
        {distribution.length > 0 && (
          <div>
            <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
              <FileWarning size={10} aria-hidden="true" />
              {zh ? "消息 token 占用 Top 6" : "Top message tokens"}
            </div>
            <div className="space-y-1">
              {distribution.map((d) => (
                <div key={d.id} className="flex items-center gap-1.5 text-[9px]">
                  <span className="w-12 text-zinc-500 truncate">{d.sender}</span>
                  <div className="flex-1 h-2 bg-zinc-900 rounded-sm overflow-hidden" aria-hidden="true">
                    <div className="h-full bg-gradient-to-r from-cyan-600 to-cyan-400" style={{ width: `${(d.tokens / maxDist) * 100}%` }} />
                  </div>
                  <span className="w-14 text-right text-zinc-400 font-mono">{d.tokens.toLocaleString()}t</span>
                </div>
              ))}
            </div>
            <div className="text-[8px] text-zinc-600 mt-1">
              {zh ? "占用最大的消息是滑窗截断时的首要剪枝对象" : "Largest messages are pruned first by sliding-window truncation"}
            </div>
          </div>
        )}

        {/* ── 逃生门 ── */}
        {p >= 0.95 && (
          <button
            onClick={onNewSession}
            className="w-full bg-red-500/80 hover:bg-red-500 text-white text-[10px] font-bold py-1.5 rounded transition-colors"
          >
            {zh ? "上下文已达红线 — 新建会话" : "Red line reached — start a new session"}
          </button>
        )}
      </div>
    </div>
  );
}
