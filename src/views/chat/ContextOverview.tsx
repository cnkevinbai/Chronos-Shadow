// src/views/chat/ContextOverview.tsx — 上下文状态概览（右侧专项 Tab，280px 侧栏适配）
// 内容：状态词 + 剩余可输入估算 + 分段计量条（触发线/红线）+ 泄压事件时间线
//      + 压力趋势 + 消息 token 占用 Top 5 + 红线逃生按钮
import { Scissors, TrendingUp, FileWarning, Sparkles } from "lucide-react";
import { useLang } from "@/lib/i18n-context";
import type { PruneStats } from "@/lib/tauri";

interface ContextOverviewProps {
  pressure: number | null;
  history: number[];
  lastStats: PruneStats | null;
  distribution: { sender: string; tokens: number }[] | null;
  onNewSession: () => void;
}

function zoneOf(p: number, zh: boolean) {
  if (p >= 0.95) return { label: zh ? "已达红线" : "Red line", cls: "text-red-400", bar: "bg-red-500" };
  if (p >= 0.8) return { label: zh ? "接近上限" : "Near limit", cls: "text-amber-400", bar: "bg-amber-400" };
  if (p >= 0.5) return { label: zh ? "正常" : "Normal", cls: "text-cyan-400", bar: "bg-cyan-400" };
  return { label: zh ? "充足" : "Plenty", cls: "text-emerald-400", bar: "bg-emerald-500" };
}

export default function ContextOverview({
  pressure, history, lastStats, distribution, onNewSession,
}: ContextOverviewProps) {
  const { lang } = useLang();
  const zh = lang === "zh";
  const p = pressure ?? 0;
  const zone = zoneOf(p, zh);

  const windowTokens = lastStats?.active_window_tokens || lastStats?.context_window_tokens || 65536;
  const remainTokens = Math.max(0, Math.floor(windowTokens * (1 - p)));
  const remainWan = (Math.round(remainTokens / 0.6 / 100) / 100).toFixed(2);
  const triggerPos = Math.min(100, (lastStats?.compact_ratio ?? 0.8) * 100);

  const distList = distribution ?? [];
  const maxDist = distList[0]?.tokens || 1;
  const trend = history.slice(-14);

  const reliefLine = lastStats && lastStats.stage_reached !== "None"
    ? (zh
        ? `剪枝 ${lastStats.tool_results_pruned} 条工具输出 · 移除 ${lastStats.messages_dropped} 条旧消息 · 节省 ${lastStats.chars_saved.toLocaleString()} 字符`
        : `pruned ${lastStats.tool_results_pruned} tool outputs · dropped ${lastStats.messages_dropped} messages · saved ${lastStats.chars_saved.toLocaleString()} chars`)
    : null;

  return (
    <div className="h-full overflow-y-auto bg-cs-surface text-cs-text p-2.5 space-y-3" data-testid="context-overview">
      {/* 状态词 + 剩余可输入 */}
      <div className="flex items-center justify-between">
        <span className={`text-base font-bold ${zone.cls}`}>{zone.label}</span>
        <span className="text-[10px] text-zinc-500">{Math.round(p * 100)}%</span>
      </div>
      <div className="text-[10px] text-zinc-400 bg-cs-header/60 border border-cs-border rounded px-2 py-1.5">
        {zh
          ? <>约还能输入 <b className="text-zinc-200">{remainWan} 万字</b>（1 token ≈ 1.7 字）</>
          : <>~<b className="text-zinc-200">{remainWan}0k chars</b> left (1 token ≈ 1.7 chars)</>}
      </div>

      {/* 分段计量条 */}
      <div>
        <div className="relative h-3 w-full rounded-full overflow-hidden flex" aria-hidden="true">
          <div className="h-full bg-emerald-900/50" style={{ width: "80%" }} />
          <div className="h-full bg-amber-900/50" style={{ width: "15%" }} />
          <div className="h-full bg-red-900/50" style={{ width: "5%" }} />
          <div className={`absolute inset-y-0 left-0 ${zone.bar} opacity-75 transition-all`} style={{ width: `${Math.min(100, p * 100)}%` }} />
          <div className="absolute inset-y-0 w-0.5 bg-zinc-200/90" style={{ left: `${triggerPos}%` }} title={zh ? "泄压触发线" : "Relief trigger"} />
        </div>
        <div className="flex justify-between text-[8px] text-zinc-600 mt-0.5">
          <span>0</span>
          <span className="text-zinc-400">{zh ? "自动泄压" : "auto-relief"} {triggerPos.toFixed(0)}%</span>
          <span>100%</span>
        </div>
      </div>

      {/* 泄压事件时间线 */}
      {reliefLine && (
        <div className="flex items-start gap-1.5 text-[9px] text-zinc-400 bg-cs-header/60 border border-cs-border rounded px-2 py-1.5">
          <Scissors size={10} className="mt-0.5 shrink-0 text-cyan-400" aria-hidden="true" />
          <span>{reliefLine}</span>
        </div>
      )}

      {/* 压力趋势 */}
      {trend.length > 1 && (
        <div>
          <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
            <TrendingUp size={10} aria-hidden="true" />
            {zh ? "压力趋势（最近发送）" : "Pressure trend"}
          </div>
          <div className="flex items-end gap-0.5 h-9" aria-hidden="true">
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

      {/* 消息占用 Top 5 */}
      {distList.length > 0 && (
        <div>
          <div className="text-[9px] font-bold text-zinc-400 flex items-center gap-1 mb-1">
            <FileWarning size={10} aria-hidden="true" />
            {zh ? "占用最大的消息" : "Largest messages"}
          </div>
          <div className="space-y-1">
            {distList.map((d) => (
              <div key={d.sender + String(d.tokens)} className="flex items-center gap-1.5 text-[9px]">
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

      {/* 逃生按钮 */}
      {p >= 0.95 && (
        <button
          onClick={onNewSession}
          className="w-full bg-red-500/80 hover:bg-red-500 text-white text-[10px] font-bold py-1.5 rounded transition-colors flex items-center justify-center gap-1"
        >
          <Sparkles size={11} aria-hidden="true" />
          {zh ? "上下文已达红线 — 新建会话" : "Red line — start a new session"}
        </button>
      )}
    </div>
  );
}
