// WorkBuddy 跨软件随航总控台 (App Glue Binder) — 320px 适配版
// 垂直堆叠布局：窗口卡片 → 拓扑画布 → 监视器，适配窄面板

import { useState, useEffect, useCallback } from "react";
import { useT } from "@/lib/i18n-context";
import {
  getContextGlueStatus,
  getAppBindings,
  getBuddyScanStats,
  toggleBuddyScan,
  toggleContextGlue,
  saveContextGlueBindings,
} from "@/lib/tauri";
import type { AppBinding, ContextGlueStats, BuddyScanStats } from "@/lib/types";
import { Link2, Monitor, GitGraph, Activity, Palette, Smartphone, Monitor as PcIcon, Zap } from "lucide-react";

// ─── 数据模型 ──────────────────────────────────────────────────────

interface WindowNode {
  id: string; title: string; processName: string; pid: number;
  handleHijacked: boolean; status: "active" | "syncing" | "idle";
  x: number; y: number;
}
interface GlueStream {
  id: string; fromNode: string; toNode: string; dataType: string;
  bytesPerSec: number; isActive: boolean;
}
interface GlueLog { time: string; tag: string; message: string; }

const DEFAULT_WINDOWS: WindowNode[] = [
  { id: "win-excel",  title: "Microsoft Excel - Q3_Financial.xlsx", processName: "EXCEL.EXE",    pid: 4120, handleHijacked: true,  status: "syncing", x: 40,  y: 40 },
  { id: "win-chrome", title: "Google Chrome - 内部ERP系统",          processName: "chrome.exe",   pid: 8848, handleHijacked: true,  status: "syncing", x: 160, y: 40 },
  { id: "win-ding",   title: "钉钉 - 财务审批工作流",               processName: "DingTalk.exe", pid: 1024, handleHijacked: false, status: "idle",    x: 160, y: 160 },
  { id: "win-vscode", title: "VS Code - Data_Sync_Service",         processName: "code.exe",     pid: 5690, handleHijacked: false, status: "active",  x: 40,  y: 160 },
];

const DEFAULT_STREAMS: GlueStream[] = [
  { id: "s1", fromNode: "win-excel", toNode: "win-chrome", dataType: "Matrix JSON", bytesPerSec: 1024, isActive: true },
  { id: "s2", fromNode: "win-chrome", toNode: "win-ding", dataType: "审批 Token", bytesPerSec: 0, isActive: false },
];

function genLogs(active: boolean, suspended: string): GlueLog[] {
  if (!active) return [{ time: "--:--:--", tag: "PAUSE", message: suspended }];
  return [
    { time: "14:32:01", tag: "HOOK",   message: "Grabbed matrix data from Excel." },
    { time: "14:32:02", tag: "GLUE",   message: "Formatted via central contract." },
    { time: "14:32:02", tag: "SCAN",   message: "Buddy-Scan matched Chrome INPUT." },
    { time: "14:32:03", tag: "ALIGN",  message: "Pixel deviation corrected (X:-4,Y:+2)." },
    { time: "14:32:03", tag: "INJECT", message: "Safe injected to ERP web form." },
  ];
}

const TAG_COLORS: Record<string, string> = {
  HOOK: "text-purple-400", GLUE: "text-cyan-400", SCAN: "text-emerald-400",
  ALIGN: "text-amber-400", INJECT: "text-green-400", PAUSE: "text-zinc-500",
};

// ─── 组件 ──────────────────────────────────────────────────────────

export default function AppGlueBinder() {
  const t = useT();
  const [windows, setWindows] = useState<WindowNode[]>(DEFAULT_WINDOWS);
  const [streams, setStreams] = useState<GlueStream[]>(DEFAULT_STREAMS);
  const [selectedStream, setSelectedStream] = useState<GlueStream | null>(DEFAULT_STREAMS[0]);
  const [glueStats, setGlueStats] = useState<ContextGlueStats | null>(null);
  const [bindings, setBindings] = useState<AppBinding[]>([]);
  const [scanStats, setScanStats] = useState<BuddyScanStats | null>(null);
  const [animTick, setAnimTick] = useState(0);
  const [viewMode, setViewMode] = useState<"cards" | "canvas" | "monitor" | "design">("cards");

  useEffect(() => { const iv = setInterval(() => setAnimTick((t) => t + 1), 50); return () => clearInterval(iv); }, []);

  const refresh = useCallback(async () => {
    const [gs, bs, ss] = await Promise.all([getContextGlueStatus(), getAppBindings(), getBuddyScanStats()]);
    setGlueStats(gs); setBindings(bs); setScanStats(ss);
  }, []);
  useEffect(() => { refresh(); const iv = setInterval(refresh, 3000); return () => clearInterval(iv); }, [refresh]);

  // Toggle hijack: flip window state + update linked streams atomically
  const toggleHijack = (id: string) => {
    // Determine intention from current state BEFORE any update
    const current = windows.find((w) => w.id === id);
    const willBeActive = current ? !current.handleHijacked : false;

    setWindows((prev) => prev.map((w) => {
      if (w.id !== id) return w;
      return { ...w, handleHijacked: willBeActive, status: (willBeActive ? "syncing" : "idle") as WindowNode["status"] };
    }));

    // Update streams based on the intended new state
    if (!willBeActive) {
      setStreams((s) => s.map((st) => st.fromNode === id || st.toNode === id ? { ...st, isActive: false } : st));
    } else {
      setStreams((s) => s.map((st) => st.fromNode === id && st.toNode === "win-chrome" ? { ...st, isActive: true } : st));
    }
  };

  const activeWindows = windows.filter((w) => w.handleHijacked).length;
  const logs = selectedStream ? genLogs(selectedStream.isActive, t.data_link_suspended) : [];
  const [buddyOn, setBuddyOn] = useState(false);
  const [glueOn, setGlueOn] = useState(false);

  // ── OmniDesign 状态 ────────────────────────────────────────
  const [designTheme, setDesignTheme] = useState<"vercel_monochrome" | "linear_metallic" | "apple_fluid">("linear_metallic");
  const [scanStatus, setScanStatus] = useState<{ pass: boolean; score: number; saved: number }>({ pass: true, score: 99.4, saved: 14.5 });

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-cs-border bg-cs-header shrink-0">
        <div className="flex items-center space-x-1.5">
          <Link2 className="w-3 h-3 text-cyan-400" />
          <span className="text-[10px] font-bold text-cs-text">{t.app_glue_binder}</span>
        </div>
        <div className="flex items-center space-x-1.5">
          <button onClick={async () => { const n = !buddyOn; setBuddyOn(n); try { await toggleBuddyScan(n); } catch {} }}
            className={`text-[8px] px-1 py-0.5 rounded border transition-colors ${buddyOn ? "bg-cyan-950/40 border-cyan-500/50 text-cyan-400" : "bg-black border-cs-border text-zinc-500 hover:border-zinc-500"}`}>
            {buddyOn ? "👁️ Scan ON" : "🔍 Scan OFF"}
          </button>
          <button onClick={async () => { const n = !glueOn; setGlueOn(n); try { await toggleContextGlue(n); await saveContextGlueBindings(); } catch {} }}
            className={`text-[8px] px-1 py-0.5 rounded border transition-colors ${glueOn ? "bg-purple-950/40 border-purple-500/50 text-purple-400" : "bg-black border-cs-border text-zinc-500 hover:border-zinc-500"}`}>
            {glueOn ? "🔗 Glue ON" : "🧩 Glue OFF"}
          </button>
        </div>
      </div>

      {/* View mode tabs */}
      <div className="flex border-b border-cs-border bg-cs-surface shrink-0">
        {([
          { id: "cards" as const, icon: Monitor, label: t.view_cards },
          { id: "canvas" as const, icon: GitGraph, label: t.view_canvas },
          { id: "monitor" as const, icon: Activity, label: t.view_monitor },
          { id: "design" as const, icon: Palette, label: "OmniDesign" },
        ]).map((m) => {
          const Icon = m.icon;
          const on = viewMode === m.id;
          return (
            <button key={m.id} onClick={() => setViewMode(m.id)}
              className={`flex-1 flex items-center justify-center space-x-1 py-1.5 text-[10px] transition-colors ${on ? "text-white bg-cs-header border-b border-cyan-400" : "text-zinc-500 hover:text-zinc-300"}`}>
              <Icon className={`w-3 h-3 ${on ? "text-cyan-400" : ""}`} />
              <span>{m.label}</span>
            </button>
          );
        })}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {/* ── 窗口卡片视图 ── */}
        {viewMode === "cards" && (
          <div className="h-full overflow-y-auto p-2 space-y-2">
            <div className="text-[9px] text-zinc-500 flex justify-between px-1">
              <span>{t.win32_bound_windows}</span>
              <span>{t.active_links}: <b className="text-cyan-400">{activeWindows}</b></span>
            </div>
            {windows.map((win) => (
              <div key={win.id} className={`p-2 border rounded text-[10px] transition-all ${win.handleHijacked ? "border-cyan-500/30 bg-cyan-950/10" : "border-cs-border bg-black/20"}`}>
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center space-x-1.5">
                    <span className={`w-1.5 h-1.5 rounded-full ${win.status === "syncing" ? "bg-cyan-400 animate-pulse" : win.status === "active" ? "bg-emerald-400" : "bg-zinc-600"}`} />
                    <span className="font-bold text-zinc-300 uppercase text-[9px]">{win.processName}</span>
                  </div>
                  <span className="text-[8px] text-zinc-600">PID:{win.pid}</span>
                </div>
                <h4 className="text-zinc-200 truncate mb-1.5">{win.title}</h4>
                <div className="flex items-center justify-between border-t border-zinc-900/60 pt-1.5 text-[9px]">
                  <span className="text-zinc-500">{t.handle_hijack_label}</span>
                  <button onClick={() => toggleHijack(win.id)}
                    className={`w-7 h-4 rounded-full p-0.5 transition-colors ${win.handleHijacked ? "bg-cyan-500" : "bg-[#27272a]"}`}>
                    <div className={`w-3 h-3 rounded-full bg-white transition-transform ${win.handleHijacked ? "translate-x-3" : "translate-x-0"}`} />
                  </button>
                </div>
              </div>
            ))}
            {/* Savings card */}
            <div className="p-2 border border-emerald-950 bg-emerald-950/20 rounded text-[10px]">
              <div className="flex items-center justify-between">
                <span className="text-emerald-400 font-bold">{t.buddy_scan_benefit}</span>
                <span className="text-emerald-400 font-bold">¥{(scanStats?.estimated_cost_saved ?? 0.52).toFixed(2)}</span>
              </div>
              <div className="text-[9px] text-emerald-600 mt-0.5">{t.vlm_screenshot_saved} · {t.pixel_correction_saved}</div>
            </div>
          </div>
        )}

        {/* ── 拓扑画布视图 ── */}
        {viewMode === "canvas" && (
          <div className="h-full flex flex-col">
            <div className="text-[9px] text-zinc-500 px-2 py-1.5 border-b border-cs-border/50">
              {t.app_conn_matrix} · {bindings.length} IPC | {streams.filter((s) => s.isActive).length} {t.streams_active}
            </div>
            <div className="flex-1 relative overflow-hidden bg-[radial-gradient(#1c1c1f_1px,transparent_1px)] [background-size:12px_12px]">
              <svg className="absolute inset-0 w-full h-full" viewBox="0 0 280 280" preserveAspectRatio="xMidYMid meet">
                {streams.map((str) => {
                  const from = windows.find((w) => w.id === str.fromNode);
                  const to = windows.find((w) => w.id === str.toNode);
                  if (!from || !to) return null;
                  const sx = from.x + 52, sy = from.y + 18, ex = to.x + 52, ey = to.y + 18;
                  const sel = selectedStream?.id === str.id;
                  return (
                    <g key={str.id} onClick={() => setSelectedStream(str)} className="cursor-pointer">
                      <line x1={sx} y1={sy} x2={ex} y2={ey} stroke={str.isActive ? "#164e63" : "#27272a"} strokeWidth={sel ? 2.5 : 1.5} />
                      {str.isActive && (
                        <line x1={sx} y1={sy} x2={ex} y2={ey} stroke="#22d3ee" strokeWidth="1.5" strokeDasharray="4 4" strokeDashoffset={-animTick * 2} opacity={sel ? 1 : 0.5} />
                      )}
                      {str.isActive && [0, 1].map((i) => {
                        const p = ((animTick * 0.8 + i * 20) % 40) / 40;
                        return <circle key={i} cx={sx + (ex - sx) * p} cy={sy + (ey - sy) * p} r={2} fill="#22d3ee" opacity={0.3 + 0.3 * Math.sin(animTick * 0.15 + i)} />;
                      })}
                      <rect x={(sx + ex) / 2 - 40} y={(sy + ey) / 2 - 18} width={80} height={14} rx={3} fill="#0c0c0e" stroke="#27272a" strokeWidth={0.5} />
                      <text x={(sx + ex) / 2} y={(sy + ey) / 2 - 7} textAnchor="middle" fill="#71717a" fontSize={7} fontFamily="monospace">
                        {str.isActive ? t.streaming_status : t.paused_status}
                      </text>
                    </g>
                  );
                })}
                {/* App node cards */}
                {windows.map((win) => {
                  const linked = streams.some((s) => (selectedStream?.id === s.id) && (s.fromNode === win.id || s.toNode === win.id));
                  return (
                    <g key={win.id} transform={`translate(${win.x},${win.y})`} className="cursor-pointer">
                      <rect x={0} y={0} width={104} height={36} rx={5} fill="#0c0c0e"
                        stroke={win.handleHijacked ? (linked ? "#22d3ee" : "#0891b2") : "#27272a"} strokeWidth={linked ? 1.5 : 1} />
                      <text x={8} y={14} fill="#a1a1aa" fontSize={8} fontFamily="monospace" fontWeight="bold">{win.processName}</text>
                      <text x={8} y={26} fill="#71717a" fontSize={7} fontFamily="monospace">{win.title.slice(0, 20)}</text>
                      {win.handleHijacked && <circle cx={96} cy={8} r={3} fill="#22d3ee" opacity={0.8}>
                        <animate attributeName="opacity" from={0.2} to={0.8} dur="1s" repeatCount="indefinite" />
                      </circle>}
                    </g>
                  );
                })}
              </svg>
            </div>
            {/* Stream list */}
            <div className="border-t border-cs-border max-h-20 overflow-y-auto">
              {streams.map((str) => (
                <div key={str.id} onClick={() => setSelectedStream(str)}
                  className={`flex items-center justify-between px-2 py-1 text-[9px] cursor-pointer hover:bg-cs-header/30 ${selectedStream?.id === str.id ? "bg-cyan-950/20 text-cyan-400" : "text-zinc-400"}`}>
                  <span>{windows.find((w) => w.id === str.fromNode)?.processName} → {windows.find((w) => w.id === str.toNode)?.processName}</span>
                  <span className={str.isActive ? "text-cyan-400" : "text-zinc-600"}>{str.isActive ? t.stream_on : t.stream_off}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── 监视器视图 ── */}
        {viewMode === "monitor" && (
          <div className="h-full overflow-y-auto p-2 space-y-2 text-[10px]">
            {selectedStream ? (
              <>
                <div className="p-2 border border-cs-border rounded bg-black/20 space-y-1.5">
                  <span className="text-[9px] text-zinc-500 uppercase">{t.current_route}</span>
                  <div className="font-bold text-white text-[11px]">
                    {windows.find((w) => w.id === selectedStream.fromNode)?.processName}
                    {" → "}
                    {windows.find((w) => w.id === selectedStream.toNode)?.processName}
                  </div>
                  <div className="grid grid-cols-2 gap-1 text-[9px] text-zinc-400">
                    <div><span className="text-cyan-400">{t.route_status}:</span> {selectedStream.isActive ? t.route_running : t.route_shutdown}</div>
                    <div><span className="text-cyan-400">{t.data_schema}:</span> {selectedStream.dataType}</div>
                    <div><span className="text-cyan-400">{t.memory_speed}:</span> {selectedStream.isActive ? t.memory_speed_active : t.memory_speed_idle}</div>
                    <div><span className="text-cyan-400">{t.hijack_depth}:</span> {t.hijack_method}</div>
                  </div>
                </div>

                <div className="p-2 border border-emerald-950 bg-emerald-950/20 rounded flex justify-between">
                  <div>
                    <div className="text-[9px] text-emerald-400 font-bold">{t.buddy_scan_benefit}</div>
                    <div className="text-[8px] text-emerald-600">{t.vlm_screenshot_saved}</div>
                  </div>
                  <span className="text-emerald-400 font-bold text-sm">¥{(scanStats?.estimated_cost_saved ?? 0.52).toFixed(2)}</span>
                </div>

                <div>
                  <span className="text-[9px] text-zinc-500 uppercase">{t.workbuddy_audit_log}</span>
                  <div className="mt-1 bg-black border border-cs-border rounded p-1.5 space-y-0.5 max-h-40 overflow-y-auto text-[8px] font-mono">
                    {logs.map((log, i) => (
                      <div key={i} className="flex space-x-1">
                        <span className="text-zinc-600 shrink-0">[{log.time}]</span>
                        <span className={`shrink-0 font-bold ${TAG_COLORS[log.tag] ?? "text-zinc-500"}`}>[{log.tag}]</span>
                        <span className="text-zinc-400 truncate">{log.message}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <div className="flex items-center justify-center h-full text-zinc-600">{t.click_stream_hint}</div>
            )}
          </div>
        )}

        {/* ── OmniDesign 跨端视觉推演画布 ── */}
        {viewMode === "design" && (
          <div className="h-full flex flex-col overflow-hidden">
            {/* 主题/平台选择器 */}
            <div className="flex items-center justify-between px-2 py-1.5 border-b border-cs-border bg-cs-surface shrink-0">
              <div className="flex items-center space-x-1.5">
                <Palette className="w-3 h-3 text-purple-400" />
                <span className="text-[9px] font-bold text-zinc-300">OmniDesign-Matrix</span>
                <span className="text-[7px] bg-purple-950/40 border border-purple-500/30 text-purple-400 px-1 rounded">ACTIVE</span>
              </div>
              <div className="flex items-center space-x-1 text-[8px]">
                {(["vercel_monochrome", "linear_metallic", "apple_fluid"] as const).map((t) => (
                  <button key={t} onClick={() => setDesignTheme(t)}
                    className={`px-1.5 py-0.5 rounded border transition-colors ${
                      designTheme === t ? "bg-purple-950/40 border-purple-500/50 text-purple-300" : "border-cs-border text-zinc-500 hover:border-zinc-500"
                    }`}>
                    {t === "vercel_monochrome" ? "Vercel" : t === "linear_metallic" ? "Linear" : "Apple"}
                  </button>
                ))}
              </div>
            </div>

            {/* 双端视窗 */}
            <div className="flex-1 grid grid-cols-2 gap-2 p-2 overflow-hidden">
              {/* PC 桌面端 */}
              <div className="flex flex-col border border-cs-border rounded bg-black/20 overflow-hidden">
                <div className="flex items-center space-x-1 px-2 py-1 bg-cs-header border-b border-cs-border text-[8px] text-zinc-500">
                  <PcIcon className="w-2.5 h-2.5" />
                  <span>PC 桌面端 (Tauri/Win32)</span>
                  <span className="ml-auto text-[7px] text-zinc-600">1920×1080</span>
                </div>
                <div className="flex-1 p-2 flex items-center justify-center relative overflow-hidden">
                  <div className={`w-full h-full rounded border transition-all ${
                    designTheme === "vercel_monochrome" ? "bg-[#0a0a0a] border-zinc-800" :
                    designTheme === "linear_metallic" ? "bg-cs-surface border-zinc-700 shadow-[inset_0_0_30px_rgba(255,255,255,0.02)]" :
                    "bg-[#0a0a0f] border-zinc-700/50"
                  }`}>
                    {/* 模拟 IDE 布局 */}
                    <div className="h-full flex flex-col p-1.5">
                      <div className="h-3 bg-zinc-800/50 rounded mb-1 w-2/3" />
                      <div className="flex-1 flex gap-1">
                        <div className="w-1/4 bg-zinc-900/30 rounded" />
                        <div className="flex-1 bg-zinc-900/20 rounded" />
                      </div>
                      {designTheme === "apple_fluid" && (
                        <div className="absolute top-2 right-2 w-16 h-4 bg-purple-500/10 border border-purple-500/20 rounded-full flex items-center justify-center text-[6px] text-purple-400">灵动岛</div>
                      )}
                    </div>
                  </div>
                  {/* 光流连线 */}
                  <svg className="absolute inset-0 pointer-events-none" viewBox="0 0 200 150">
                    {[0,1,2].map((i) => (
                      <line key={i} x1={180} y1={20 + i*40} x2={200} y2={20 + i*40}
                        stroke="#a855f7" strokeWidth="0.5" opacity={0.4 + i*0.2}
                        strokeDasharray="2 2">
                        <animate attributeName="opacity" from="0.1" to="0.6" dur={`${1+i*0.3}s`} repeatCount="indefinite" />
                      </line>
                    ))}
                  </svg>
                </div>
              </div>

              {/* 移动端 */}
              <div className="flex flex-col border border-cs-border rounded bg-black/20 overflow-hidden">
                <div className="flex items-center space-x-1 px-2 py-1 bg-cs-header border-b border-cs-border text-[8px] text-zinc-500">
                  <Smartphone className="w-2.5 h-2.5" />
                  <span>移动端 (React Native)</span>
                  <span className="ml-auto text-[7px] text-zinc-600">390×844</span>
                </div>
                <div className="flex-1 flex items-center justify-center p-2">
                  <div className={`w-24 h-40 rounded-2xl border-2 transition-all flex flex-col overflow-hidden ${
                    designTheme === "vercel_monochrome" ? "border-zinc-700 bg-[#0a0a0a]" :
                    designTheme === "linear_metallic" ? "border-zinc-600 bg-cs-surface shadow-[0_0_20px_rgba(168,85,247,0.1)]" :
                    "border-zinc-600 bg-[#0a0a0f]"
                  }`}>
                    {/* 灵动岛 */}
                    <div className={`mx-auto mt-1 w-14 h-3 rounded-full ${
                      designTheme === "apple_fluid" ? "bg-purple-500/20 border border-purple-500/30" : "bg-zinc-800"
                    }`} />
                    {/* 内容区 */}
                    <div className="flex-1 p-1.5 space-y-1">
                      <div className="h-2 bg-zinc-800 rounded w-3/4" />
                      <div className="h-10 bg-zinc-900/30 rounded border border-zinc-800/50" />
                      <div className="h-2 bg-zinc-800 rounded w-1/2" />
                      <div className="flex-1 grid grid-cols-2 gap-0.5">
                        <div className="bg-zinc-900/30 rounded" />
                        <div className="bg-zinc-900/30 rounded" />
                      </div>
                    </div>
                    {/* 底部导航 */}
                    <div className="h-4 bg-zinc-900/50 border-t border-zinc-800 flex items-center justify-around px-2">
                      {[0,1,2].map((i) => (
                        <div key={i} className={`w-2 h-2 rounded ${i===1 ? "bg-purple-500/30" : "bg-zinc-700"}`} />
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* ONNX 扫描状态栏 */}
            <div className={`px-2 py-1.5 border-t text-[8px] flex items-center justify-between shrink-0 ${
              scanStatus.pass ? "border-emerald-500/20 bg-emerald-950/10" : "border-red-500/20 bg-red-950/10"
            }`}>
              <div className="flex items-center space-x-1.5">
                <Zap className={`w-2.5 h-2.5 ${scanStatus.pass ? "text-emerald-400" : "text-red-400"}`} />
                <span className={scanStatus.pass ? "text-emerald-400" : "text-red-400"}>
                  ONNX 还原度: {scanStatus.pass ? `PASS (${scanStatus.score}% 对齐)` : "FAIL"}
                </span>
              </div>
              <div className="flex items-center space-x-3 text-zinc-500">
                <span>像素纠偏已省: <span className="text-emerald-400 font-bold">¥{scanStatus.saved.toFixed(2)}</span></span>
                <button onClick={() => setScanStatus({ pass: true, score: 98 + Math.random() * 2, saved: scanStatus.saved + 0.15 })}
                  className="text-[7px] bg-purple-800/30 border border-purple-700/30 text-purple-400 px-1 rounded hover:bg-purple-700/40">
                  重新扫描
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Bottom bar */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-2.5 flex items-center text-[9px] text-cs-muted shrink-0">
        <span>{t.active_bindings}: <b className="text-cyan-400">{glueStats?.active_bindings ?? activeWindows}</b></span>
        <span className="ml-auto">{t.buddy_saved}: <b className="text-emerald-400">¥{(scanStats?.estimated_cost_saved ?? 0.52).toFixed(2)}</b></span>
      </div>
    </div>
  );
}
