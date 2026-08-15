// Chronos-Shadow 主框架 — 工作台/全局配置 双视图路由
// 白皮书 6.1 + 6.7 最终联调版本

import { useState, useEffect, useCallback } from "react";
import { I18nProvider, useT } from "@/lib/i18n-context";
import { getModel, getLLMs, getVLMs, classifyModelKeys } from "@/lib/models";
import CommandPalette, { buildPaletteCommands } from "@/components/CommandPalette";
import Modal from "@/components/Modal";
import SdlcPipelinePanel from "@/views/SdlcPipelinePanel";
import ProjectExplorer from "@/views/ProjectExplorer";
import ChatPanel from "@/views/ChatPanel";
import SettingsPanel from "@/views/SettingsPanel";
import EvolutionConsole from "@/views/EvolutionConsole";
import AppGlueBinder from "@/views/AppGlueBinder";
import SkillMcpHub from "@/views/SkillMcpHub";
import WebIntelligencePanel from "@/views/WebIntelligencePanel";
import AutoRoutingPanel from "@/views/AutoRoutingPanel";
import RemoteHub from "@/views/RemoteHub";
import RedlineGuardPanel from "@/views/RedlineGuardPanel";
import SecurityShieldPanel from "@/components/SecurityShieldPanel";
import ApprovalPanel from "@/views/ApprovalPanel";
import FooterBar from "@/components/FooterBar";
import FloatingBubble from "@/components/FloatingBubble";
import ErrorBoundary from "@/components/ErrorBoundary";
import { ToastProvider, useToast } from "@/components/ToastProvider";
import { ChatIcon, PipelineIcon, GlueIcon, McpIcon, ChronosFolderIcon, RemoteIcon, ChronosLogo } from "@/components/SvgIcons";
import {
  getSessionCost,
  getSavedCost,
  getSavingRate,
  getBuddySaved,
  loadSettings,
  saveSettings,
  getSandboxStatus,
  getRedlineStatus,
  getPipelineStats,
  startPipeline,
  pausePipeline,
  resumePipeline,
  advancePipeline,
  onPipelineEvent,
  getAvailableModels,
  setRouteMode as setRouteModeIpc,
  submitForApproval,
  getGreeting,
  getUserProfile,
  getHeartbeat,
  touchInteraction,
} from "@/lib/tauri";
import type { RedlineStatus, OrchestratorStats, Heartbeat } from "@/lib/types";

const SHORTCUTS: { keys: string[]; desc: string }[] = [
  { keys: ["Ctrl", "K"], desc: "打开 / 关闭命令面板" },
  { keys: ["Ctrl", "N"], desc: "新建会话" },
  { keys: ["Ctrl", "S"], desc: "保存当前会话" },
  { keys: ["Ctrl", "F"], desc: "搜索消息" },
  { keys: ["Ctrl", "Shift", "E"], desc: "导出会话 JSON" },
  { keys: ["Esc"], desc: "关闭面板 / 菜单 / 清空附件" },
  { keys: ["↑", "↓"], desc: "命令面板内导航" },
  { keys: ["↵"], desc: "执行选中命令" },
];

function modelLabel(m: string): string {
  return getModel(m)?.display ?? m;
}

function AppInner() {
  const t = useT();
  const toast = useToast();
  // ── 全局视图路由 ────────────────────────────────────────────────
  const [activeView, setActiveView] = useState<"workbench" | "settings" | "evolution">("workbench");
  // Dock 导航
  const [dockView, setDockView] = useState<"chat" | "pipeline" | "glue" | "skills" | "webintel" | "autoroute" | "remote" | "explorer" | "approval">("chat");
  // Command Palette
  const [showPalette, setShowPalette] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);

  // ── 模型配置 ────────────────────────────────────────────────────
  const [routeMode, setRouteMode] = useState<"auto" | "manual">("auto");
  const [selectedLLM, setSelectedLLM] = useState("deepseek-v4-pro");
  const [selectedVLM, setSelectedVLM] = useState("glm-5v-turbo");
  const [availableLLMs, setAvailableLLMs] = useState<string[]>([]);
  const [availableVLMs, setAvailableVLMs] = useState<string[]>([]);

  // ── 项目状态 ────────────────────────────────────────────────────
  const [currentProject, setCurrentProject] = useState("Chronos-Core-Demo");
  // 项目切换时持久化 (合并现有设置, 避免覆盖其他字段)
  useEffect(() => {
    if (currentProject === "Chronos-Core-Demo") return;
    loadSettings().then(s => {
      saveSettings({ ...s, current_project: currentProject } as any).catch(() => {});
    }).catch(() => {});
  }, [currentProject]);
  const [sandboxStatus, setSandboxStatus] = useState("Protected (Global Node.js Symlinked)");

  // ── 实时 IPC 数据 ───────────────────────────────────────────────
  // Initial values before first IPC poll (replaced within 2s)
  const [sessionCost, setSessionCost] = useState(0.0);
  const [savedCost, setSavedCost] = useState(0.0);
  const [savingRate, setSavingRate] = useState(0);
  const [buddySaved, setBuddySaved] = useState(0.0);
  const [redlineStatus, setRedlineStatus] = useState<RedlineStatus | null>(null);
  const [pipelineStats, setPipelineStats] = useState<OrchestratorStats | null>(null);

  // 迷你悬浮球模式
  const [minimized, setMinimized] = useState(false);

  // ── API 密钥 ────────────────────────────────────────────────────
  // API key presence flags — actual keys never leave Rust backend
  const [hasKeys, setHasKeys] = useState({ deepseek: false, kimi: false, glm: false });

  // ── 个性化 (用户画像) ──────────────────────────────────────────
  const [greeting, setGreeting] = useState("");
  const [avatar, setAvatar] = useState("🦀");
  const [heartbeat, setHeartbeat] = useState<Heartbeat | null>(null);

  // 启动时加载个性化问候 + 头像 + 心跳，并记录一次交互
  useEffect(() => {
    getGreeting().then(setGreeting).catch(() => {});
    getUserProfile().then((p) => { setAvatar(p.avatar); }).catch(() => {});
    getHeartbeat().then(setHeartbeat).catch(() => {});
    touchInteraction().catch(() => {});
  }, []);

  // ── 启动时恢复持久化配置 (延迟确保 IPC 就绪) ─────────────────
  useEffect(() => {
    let tid1: ReturnType<typeof setTimeout> | undefined;
    let tid2: ReturnType<typeof setTimeout> | undefined;
    const load = async () => {
      try {
        const s = await loadSettings();
        setHasKeys({
          deepseek: s.has_key_deepseek ?? false,
          kimi: s.has_key_kimi ?? false,
          glm: s.has_key_glm ?? false,
        });
        if (s.current_project) setCurrentProject(s.current_project);
      } catch {
        tid2 = setTimeout(async () => {
          try {
            const s = await loadSettings();
            setHasKeys({
              deepseek: s.has_key_deepseek ?? false,
              kimi: s.has_key_kimi ?? false,
              glm: s.has_key_glm ?? false,
            });
          } catch { /* silent */ }
        }, 1000);
      }
    };
    tid1 = setTimeout(load, 500);
    return () => { if (tid1) clearTimeout(tid1); if (tid2) clearTimeout(tid2); };
  }, []);

  // ── 数据轮询 ────────────────────────────────────────────────────

  const refreshCosts = useCallback(async () => {
    const [cost, saved, rate, buddy] = await Promise.all([getSessionCost(), getSavedCost(), getSavingRate(), getBuddySaved()]);
    setSessionCost(cost);
    setSavedCost(saved);
    setSavingRate(rate);
    setBuddySaved(buddy);
  }, []);

  const refreshStatus = useCallback(async () => {
    const [sandbox, redline, pipeline] = await Promise.all([getSandboxStatus(), getRedlineStatus(), getPipelineStats()]);
    setSandboxStatus(sandbox);
    setRedlineStatus(redline);
    setPipelineStats(pipeline);
  }, []);

  useEffect(() => {
    getAvailableModels().then((models) => {
      const { llms, vlms, unknown } = classifyModelKeys(models);
      // 注册表缺失的模型降级为文本模型，避免丢失可选模型
      setAvailableLLMs([...llms, ...unknown]);
      setAvailableVLMs(vlms);
      if (unknown.length > 0) {
        console.warn("[models] Rust 返回了 models.ts 注册表中不存在的模型:", unknown);
      }
    });
  }, []);

  useEffect(() => {
    refreshCosts();
    refreshStatus();
    const t1 = setInterval(refreshCosts, 2000);
    const t2 = setInterval(refreshStatus, 3000);
    let unlisten: (() => void) | undefined;
    onPipelineEvent(() => refreshStatus()).then((fn) => { unlisten = fn; });
    return () => { clearInterval(t1); clearInterval(t2); unlisten?.(); };
  }, [refreshCosts, refreshStatus]);

  // ── 模型路由同步 ────────────────────────────────────────────────

  const syncRouteMode = (mode: "auto" | "manual") => {
    setRouteMode(mode);
    if (mode === "auto") {
      setRouteModeIpc('"AutoMatrix"').catch(() => {});
      toast.showToast("info", "ROUTER STATE", t.toast_router_auto);
    } else {
      setRouteModeIpc(JSON.stringify({ Manual: { text_model: selectedLLM, vision_model: selectedVLM || null } })).catch(() => {});
      toast.showToast("warning", "MANUAL OVERRIDE", t.toast_router_manual);
    }
  };

  const onLLMChange = (v: string) => {
    setSelectedLLM(v);
    if (routeMode === "manual") {
      setRouteModeIpc(JSON.stringify({ Manual: { text_model: v, vision_model: selectedVLM || null } })).catch(() => {});
    }
    toast.showToast("success", "LLM CONFIG SYNCED", `${t.toast_llm_changed} ${modelLabel(v)}。`);
  };

  const onVLMChange = (v: string) => {
    setSelectedVLM(v);
    if (routeMode === "manual") {
      setRouteModeIpc(JSON.stringify({ Manual: { text_model: selectedLLM, vision_model: v || null } })).catch(() => {});
    }
  };

  // ── Pipeline 控制 ───────────────────────────────────────────────

  const isRunning = pipelineStats?.pipeline_running ?? false;

  const handlePipeline = {
    start: async () => { await startPipeline(); refreshStatus(); toast.showToast("info", "PIPELINE", t.toast_pipeline_start); },
    pause: async () => { await pausePipeline(); refreshStatus(); toast.showToast("warning", "PIPELINE", t.toast_pipeline_pause); },
    resume: async () => { await resumePipeline(); refreshStatus(); toast.showToast("success", "PIPELINE", t.toast_pipeline_resume); },
    advance: async () => {
      try {
        await advancePipeline();
        refreshStatus();
        toast.showToast("info", "PIPELINE", t.toast_pipeline_advance);
      } catch (e: unknown) {
        const msg = String(e);
        if (msg.includes("第四红线")) {
          // 从错误消息中提取 target_id 并自动提交审批
          const targetMatch = msg.match(/target=([^)]+)/);
          const targetId = targetMatch ? targetMatch[1] : "pipeline:unknown";
          try {
            await submitForApproval("pipeline_advance", targetId,
              `SDLC 流水线跃迁: ${targetId}`, "{}");
            toast.showToast("warning", "⛔ 审批门禁",
              `已自动提交审批 (${targetId})。请切换到审批面板审核后重试推进。`);
          } catch {
            toast.showToast("error", "⛔ 审批门禁", msg);
          }
        } else {
          toast.showToast("error", "PIPELINE ERROR", msg);
        }
      }
    },
  };

  // ── 命令面板 (Ctrl+K) ──────────────────────────────────────────
  const dispatchChatCommand = (cmd: string) => {
    window.dispatchEvent(new CustomEvent("chronos:command", { detail: cmd }));
  };
  const goChatThen = (cmd: string) => {
    setActiveView("workbench");
    setDockView("chat");
    setTimeout(() => dispatchChatCommand(cmd), 0);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setShowPalette((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const paletteCommands = buildPaletteCommands({
    onNavigate: (v) => { setActiveView("workbench"); setDockView(v as typeof dockView); setMinimized(false); },
    onNewSession: () => goChatThen("new-session"),
    onSaveSession: () => goChatThen("save-session"),
    onExportSession: () => goChatThen("export-session"),
    onClearAll: () => goChatThen("clear-all"),
    onToggleSidebar: () => goChatThen("toggle-sidebar"),
    onFocusInput: () => goChatThen("focus-input"),
    onToggleRouteMode: () => syncRouteMode(routeMode === "auto" ? "manual" : "auto"),
    onOpenSettings: () => setActiveView("settings"),
    onShowShortcuts: () => setShowShortcuts(true),
  });

  // ── 渲染 ────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-screen bg-cs-bg text-cs-text font-mono select-none">
      {/* 1. Header — 全局主控栏 */}
      <header className="flex items-center justify-between px-4 py-2.5 border-b border-cs-border bg-cs-header shrink-0">
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <ChronosLogo size={20} className="stroke-cyan-400" />
            <span className="font-bold text-sm tracking-wider text-white">{t.app_title}</span>
          </div>
          <div className="h-4 w-[1px] bg-[#27272a]" />

          {/* 工作台 / 全局配置 路由切换 */}
          <div className="flex border border-cs-border rounded p-0.5 bg-black">
            <button
              onClick={() => setActiveView("workbench")}
              className={`px-3 py-1 rounded transition-all duration-150 active:scale-95 text-xs ${activeView === "workbench" ? "bg-[#27272a] text-white font-bold" : "text-zinc-500 hover:text-zinc-300"}`}
            >
              {t.workbench}
            </button>
            <button
              onClick={() => setActiveView("settings")}
              className={`px-3 py-1 rounded transition-all duration-150 active:scale-95 text-xs ${activeView === "settings" ? "bg-[#27272a] text-white font-bold" : "text-zinc-500 hover:text-zinc-300"}`}
            >
              {t.settings}
            </button>
            <button
              onClick={() => setActiveView("evolution")}
              className={`px-3 py-1 rounded transition-all duration-150 active:scale-95 text-xs ${activeView === "evolution" ? "bg-amber-500/20 text-amber-400 font-bold border border-amber-500/30" : "text-zinc-500 hover:text-zinc-300"}`}
            >
              {t.evolution_tab}
            </button>
          </div>

          {/* 工作台模式：项目指示器 */}
          {activeView === "workbench" && (
            <div className="flex items-center space-x-2 text-xs animate-fadeIn">
              <div className="h-4 w-[1px] bg-[#27272a]" />
              <span className="text-cs-dim">{t.sandbox_workspace}</span>
              <span className="text-cs-accent font-bold border border-cs-accent-border bg-cs-accent-dim/30 px-2 py-0.5 rounded">
                {currentProject}
              </span>
              <span className="text-[10px] text-cs-muted">({sandboxStatus})</span>
            </div>
          )}
        </div>

        {/* 右侧：模型配置矩阵 + Console 按钮 */}
        <div className="flex items-center space-x-3 text-xs">
          <div className="flex border border-cs-border rounded p-0.5 bg-black">
            <button onClick={() => syncRouteMode("auto")} className={`px-2.5 py-1 rounded transition-all duration-150 active:scale-95 text-[11px] ${routeMode === "auto" ? "bg-[#27272a] text-white font-bold" : "text-cs-muted"}`}>
              {t.auto_rule}
            </button>
            <button onClick={() => syncRouteMode("manual")} className={`px-2.5 py-1 rounded transition-all duration-150 active:scale-95 text-[11px] ${routeMode === "manual" ? "bg-[#27272a] text-white font-bold" : "text-cs-muted"}`}>
              {t.manual_control}
            </button>
          </div>

          <div className="flex items-center space-x-1">
            <span className="text-cs-muted">{t.text_llm}</span>
            <select disabled={routeMode === "auto"} value={selectedLLM} onChange={(e) => onLLMChange(e.target.value)}
              className="bg-black border border-cs-border rounded px-1.5 py-1 text-white disabled:opacity-40 disabled:cursor-not-allowed outline-none text-[11px]">
              {(availableLLMs.length > 0 ? availableLLMs : getLLMs().map(m => m.key)).map(m => (
                <option key={m} value={m}>{modelLabel(m)}</option>
              ))}
            </select>
          </div>

          <div className="flex items-center space-x-1">
            <span className="text-cs-muted">{t.vision_vlm}</span>
            <select value={selectedVLM} onChange={(e) => onVLMChange(e.target.value)}
              className="bg-black border border-cs-border rounded px-1.5 py-1 text-white outline-none text-[11px]">
              {(availableVLMs.length > 0 ? availableVLMs : getVLMs().map(m => m.key)).map(m => (
                <option key={m} value={m}>{modelLabel(m)}</option>
              ))}
            </select>
          </div>

          {/* 灵动岛无感随航 */}
          <button onClick={() => setMinimized(!minimized)}
            className="text-[10px] px-2 py-0.5 rounded border border-cyan-500/30 text-cyan-400 hover:border-cyan-400 hover:bg-cyan-950/20 transition-all duration-150 active:scale-95 font-bold">
            {minimized ? t.restore_mode : t.mini_mode_button}
          </button>
        </div>
      </header>

      {/* 个性化问候条 — 时间问候 + 头像 + 连续天数 */}
      {!minimized && greeting && (
        <div className="flex items-center justify-between px-4 py-1.5 border-b border-cs-border/50 bg-cs-surface/40 text-xs text-zinc-400 select-none">
          <div className="flex items-center space-x-2">
            <span className="text-sm leading-none">{avatar}</span>
            <span className="text-zinc-300">{greeting}</span>
          </div>
          {heartbeat && (
            <div className="flex items-center space-x-1.5 text-[10px] text-zinc-600 shrink-0">
              <span className={`w-1.5 h-1.5 rounded-full ${
                heartbeat.energy === "high" ? "bg-emerald-400" : heartbeat.energy === "medium" ? "bg-amber-400" : "bg-zinc-600"
              }`} />
              <span>🔥 连续 {heartbeat.streak} 天</span>
              <span className="text-zinc-700">·</span>
              <span>今日 {heartbeat.today} 次</span>
            </div>
          )}
        </div>
      )}

      {/* Mini 模式：隐藏主界面，仅显示悬浮球 */}
      {minimized && (
        <FloatingBubble
          currentAgent={
            pipelineStats?.active_role === "PM" ? "PM" :
            pipelineStats?.active_role === "UIDesigner" ? "Designer" :
            pipelineStats?.active_role === "Coder" ? "Coder" :
            pipelineStats?.active_role === "Auditor" ? "Auditor" :
            pipelineStats?.active_role === "Verifier" ? "Verifier" :
            "Idle"
          }
          savedCost={savedCost}
          onMaximize={() => setMinimized(false)}
        />
      )}

      {/* 主界面 */}
      {!minimized && (
        <>
      {/* 2. Main — 双视图路由 */}
      <div className="flex-1 flex overflow-hidden">
        {activeView === "workbench" ? (
          /* 工作台：左 Dock + 中央画布 + 右安全面板 */
          <div className="flex flex-1 overflow-hidden animate-fadeIn">
            {/* 左侧垂直 Dock 导航 */}
            <nav className="w-12 border-r border-cs-border bg-cs-surface flex flex-col items-center py-3 space-y-1.5 shrink-0 overflow-y-auto">
              {/* 核心面板 */}
              <DockButton active={dockView === "chat"} tip="沉浸对话" onClick={() => setDockView("chat")}>
                <ChatIcon size={18} className={dockView === "chat" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "pipeline"} tip="调度流水线" onClick={() => setDockView("pipeline")}>
                <PipelineIcon size={18} className={dockView === "pipeline" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>

              {/* 分隔线 */}
              <div className="w-6 h-px bg-[#27272a] my-1" />

              {/* 智能引擎 */}
              <DockButton active={dockView === "skills"} tip="技能中枢" onClick={() => setDockView("skills")}>
                <McpIcon size={18} className={dockView === "skills" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "webintel"} tip="Web智能搜索" onClick={() => setDockView("webintel")}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={dockView === "webintel" ? "stroke-cyan-400" : "stroke-zinc-500"}>
                  <circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/><path d="M11 8a3 3 0 0 0-3 3"/>
                </svg>
              </DockButton>
              <DockButton active={dockView === "autoroute"} tip="自动路由中枢" onClick={() => setDockView("autoroute")}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={dockView === "autoroute" ? "stroke-purple-400" : "stroke-zinc-500"}>
                  <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
                </svg>
              </DockButton>
              <DockButton active={dockView === "glue"} tip="跨软件粘合" onClick={() => setDockView("glue")}>
                <GlueIcon size={18} className={dockView === "glue" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>

              {/* 分隔线 */}
              <div className="w-6 h-px bg-[#27272a] my-1" />

              {/* 基础设施 */}
              <DockButton active={dockView === "remote"} tip="远程服务器" onClick={() => setDockView("remote")}>
                <RemoteIcon size={18} className={dockView === "remote" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "explorer"} tip="项目时光机" onClick={() => setDockView("explorer")}>
                <ChronosFolderIcon size={18} className={dockView === "explorer" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "approval"} tip="审批门禁" onClick={() => setDockView("approval")}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={dockView === "approval" ? "stroke-red-400" : "stroke-zinc-500"}>
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                  <path d="M9 12l2 2 4-4"/>
                </svg>
              </DockButton>
            </nav>

            {/* 中央画布 — 根据 Dock 切换 */}
            <section className="flex-1 bg-cs-bg overflow-hidden flex flex-col">
              {dockView === "chat" && (
                <div className="flex-1 overflow-hidden"><ChatPanel selectedModel={selectedLLM} apiKey="" hasKeys={hasKeys} currentProject={currentProject} onProjectChange={setCurrentProject} /></div>
              )}
              {dockView === "pipeline" && (
                <div className="flex-1 overflow-hidden">
                  <SdlcPipelinePanel
                    routeMode={routeMode} activeLLM={selectedLLM} activeVLM={selectedVLM}
                    pipelineStats={pipelineStats}
                    onStart={handlePipeline.start} onPause={handlePipeline.pause}
                    onResume={handlePipeline.resume} onAdvance={handlePipeline.advance}
                    isRunning={isRunning}
                  />
                </div>
              )}
              {dockView === "glue" && (
                <div className="flex-1 overflow-hidden"><AppGlueBinder /></div>
              )}
              {dockView === "skills" && (
                <div className="flex-1 overflow-hidden"><SkillMcpHub /></div>
              )}
              {dockView === "webintel" && (
                <div className="flex-1 overflow-hidden"><WebIntelligencePanel /></div>
              )}
              {dockView === "autoroute" && (
                <div className="flex-1 overflow-hidden"><AutoRoutingPanel /></div>
              )}
              {dockView === "remote" && (
                <div className="flex-1 overflow-hidden"><RemoteHub /></div>
              )}
              {dockView === "explorer" && (
                <div className="flex-1 overflow-hidden">
                  <ProjectExplorer currentProject={currentProject} onProjectChange={setCurrentProject} />
                </div>
              )}
              {dockView === "approval" && (
                <div className="flex-1 overflow-hidden"><ApprovalPanel /></div>
              )}
            </section>

            {/* 右侧安全风控面板 */}
            <aside className="w-[280px] border-l border-cs-border bg-cs-surface flex flex-col overflow-hidden shrink-0">
              <div className="flex-1 border-b border-cs-border overflow-hidden">
                <RedlineGuardPanel redlineStatus={redlineStatus} />
              </div>
              <div className="flex-1 overflow-hidden">
                <SecurityShieldPanel redlineStatus={redlineStatus} />
              </div>
            </aside>
          </div>
        ) : activeView === "evolution" ? (
          /* 进化控制台 */
          <div className="flex-1 overflow-hidden animate-fadeIn">
            <EvolutionConsole />
          </div>
        ) : (
          /* 全局配置 */
          <div className="flex-1 overflow-hidden animate-fadeIn">
            <SettingsPanel hasKeys={hasKeys} onKeyChange={(provider, has) => setHasKeys(prev => ({ ...prev, [provider]: has }))} />
          </div>
        )}
      </div>

      {/* 3. FooterBar — 常驻成本对账 */}
      <FooterBar sessionCost={sessionCost} savedCost={savedCost} savingRate={savingRate} routeMode={routeMode} buddySaved={buddySaved} />
        </>
      )}

      {/* 命令面板 (Ctrl+K) */}
      <CommandPalette commands={paletteCommands} open={showPalette} onClose={() => setShowPalette(false)} />

      {/* 快捷键帮助浮层 */}
      <Modal open={showShortcuts} onClose={() => setShowShortcuts(false)} title="快捷键 (Keyboard Shortcuts)">
        <div className="grid grid-cols-1 gap-1.5">
          {SHORTCUTS.map((s) => (
            <div key={s.desc} className="flex items-center justify-between px-2 py-1.5 rounded bg-black/40 border border-cs-border/50">
              <span className="text-[11px] text-zinc-400">{s.desc}</span>
              <span className="flex items-center space-x-1">
                {s.keys.map((k) => (
                  <kbd key={k} className="text-[10px] font-mono text-zinc-300 bg-cs-surface border border-cs-border px-1.5 py-0.5 rounded">{k}</kbd>
                ))}
              </span>
            </div>
          ))}
        </div>
      </Modal>
    </div>
  );
}

export default function App() {
  return (
    <I18nProvider>
      <ToastProvider>
        <ErrorBoundary>
          <AppInner />
        </ErrorBoundary>
      </ToastProvider>
    </I18nProvider>
  );
}

// ─── Dock 按钮辅助组件 ──────────────────────────────────────

function DockButton({
  active,
  tip,
  onClick,
  children,
}: {
  active: boolean;
  tip: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      title={tip}
      className={`w-9 h-9 flex items-center justify-center rounded transition-all duration-150 active:scale-90 ${
        active
          ? "bg-[#27272a] text-white border border-zinc-700 shadow-sm"
          : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/40"
      }`}
    >
      {children}
    </button>
  );
}
