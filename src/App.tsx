// Chronos-Shadow 主框架 — 工作台/全局配置 双视图路由
// 白皮书 6.1 + 6.7 最终联调版本

import { useState, useEffect, useCallback } from "react";
import { I18nProvider, useT } from "@/lib/i18n-context";
import SdlcPipelinePanel from "@/views/SdlcPipelinePanel";
import ProjectExplorer from "@/views/ProjectExplorer";
import ChatPanel from "@/views/ChatPanel";
import SettingsPanel from "@/views/SettingsPanel";
import EvolutionConsole from "@/views/EvolutionConsole";
import AppGlueBinder from "@/views/AppGlueBinder";
import SkillMcpHub from "@/views/SkillMcpHub";
import RemoteHub from "@/views/RemoteHub";
import RedlineGuardPanel from "@/views/RedlineGuardPanel";
import SecurityShieldPanel from "@/components/SecurityShieldPanel";
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
} from "@/lib/tauri";
import type { RedlineStatus, OrchestratorStats } from "@/lib/types";

function modelLabel(m: string): string {
  const labels: Record<string, string> = {
    "deepseek-v4-pro": "DeepSeek V4-Pro (深度推理)",
    "deepseek-v4-flash": "DeepSeek V4-Flash (代码生成)",
    "kimi-k3": "Kimi K3 (超长项目分析)",
    "kimi-k2.7-code": "Kimi K2.7-Code (代码专用)",
    "kimi-k2.7-code-highspeed": "Kimi K2.7-Code-HS (极速编程)",
    "glm-5.2": "GLM-5.2 (原生Agent规划)",
    "glm-5v-turbo": "GLM-5V-Turbo (高精视觉)",
    "glm-5.1": "GLM-5.1 (稳定推理)",
    "glm-4.7": "GLM-4.7 (高性价比)",
    "ollama-local": "Ollama Local (0资费)",
  };
  return labels[m] ?? m;
}

function AppInner() {
  const t = useT();
  const toast = useToast();
  // ── 全局视图路由 ────────────────────────────────────────────────
  const [activeView, setActiveView] = useState<"workbench" | "settings" | "evolution">("workbench");
  // Dock 导航
  const [dockView, setDockView] = useState<"chat" | "pipeline" | "glue" | "skills" | "remote" | "explorer">("chat");

  // ── 模型配置 ────────────────────────────────────────────────────
  const [routeMode, setRouteMode] = useState<"auto" | "manual">("auto");
  const [selectedLLM, setSelectedLLM] = useState("deepseek-v4-pro");
  const [selectedVLM, setSelectedVLM] = useState("glm-5v-turbo");
  const [availableLLMs, setAvailableLLMs] = useState<string[]>([]);
  const [availableVLMs, setAvailableVLMs] = useState<string[]>([]);

  // ── 项目状态 ────────────────────────────────────────────────────
  const [currentProject, setCurrentProject] = useState("Chronos-Core-Demo");
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
      setAvailableLLMs(models.filter((m) => !m.includes("vision")));
      setAvailableVLMs(models.filter((m) => m.includes("vision")));
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
    advance: async () => { await advancePipeline(); refreshStatus(); toast.showToast("info", "PIPELINE", t.toast_pipeline_advance); },
  };

  // ── 渲染 ────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-screen bg-[#09090b] text-[#fafafa] font-mono select-none">
      {/* 1. Header — 全局主控栏 */}
      <header className="flex items-center justify-between px-4 py-2.5 border-b border-[#27272a] bg-[#121214] shrink-0">
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <ChronosLogo size={20} className="stroke-cyan-400" />
            <span className="font-bold text-sm tracking-wider text-white">{t.app_title}</span>
          </div>
          <div className="h-4 w-[1px] bg-[#27272a]" />

          {/* 工作台 / 全局配置 路由切换 */}
          <div className="flex border border-[#27272a] rounded p-0.5 bg-black">
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
          <div className="flex border border-[#27272a] rounded p-0.5 bg-black">
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
              className="bg-black border border-[#27272a] rounded px-1.5 py-1 text-white disabled:opacity-40 disabled:cursor-not-allowed outline-none text-[11px]">
              {(availableLLMs.length > 0 ? availableLLMs : ["deepseek-v4-pro","deepseek-v4-flash","kimi-k3","kimi-k2.7-code","kimi-k2.7-code-highspeed","glm-5.2","glm-5.1","glm-4.7"]).map(m => (
                <option key={m} value={m}>{modelLabel(m)}</option>
              ))}
            </select>
          </div>

          <div className="flex items-center space-x-1">
            <span className="text-cs-muted">{t.vision_vlm}</span>
            <select value={selectedVLM} onChange={(e) => onVLMChange(e.target.value)}
              className="bg-black border border-[#27272a] rounded px-1.5 py-1 text-white outline-none text-[11px]">
              {(availableVLMs.length > 0 ? availableVLMs : ["glm-5v-turbo"]).map(m => (
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
            <nav className="w-12 border-r border-[#27272a] bg-[#0c0c0e] flex flex-col items-center py-3 space-y-2 shrink-0">
              <DockButton active={dockView === "chat"} tip="沉浸对话" onClick={() => setDockView("chat")}>
                <ChatIcon size={18} className={dockView === "chat" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "pipeline"} tip="调度流水线" onClick={() => setDockView("pipeline")}>
                <PipelineIcon size={18} className={dockView === "pipeline" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "glue"} tip="跨软件粘合" onClick={() => setDockView("glue")}>
                <GlueIcon size={18} className={dockView === "glue" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "skills"} tip="技能中枢" onClick={() => setDockView("skills")}>
                <McpIcon size={18} className={dockView === "skills" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "remote"} tip="远程服务器" onClick={() => setDockView("remote")}>
                <RemoteIcon size={18} className={dockView === "remote" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
              <DockButton active={dockView === "explorer"} tip="项目时光机" onClick={() => setDockView("explorer")}>
                <ChronosFolderIcon size={18} className={dockView === "explorer" ? "stroke-white" : "stroke-zinc-500"} />
              </DockButton>
            </nav>

            {/* 中央画布 — 根据 Dock 切换 */}
            <section className="flex-1 bg-[#09090b] overflow-hidden flex flex-col">
              {dockView === "chat" && (
                <div className="flex-1 overflow-hidden"><ChatPanel selectedModel={selectedLLM} apiKey="" hasKeys={hasKeys} /></div>
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
              {dockView === "remote" && (
                <div className="flex-1 overflow-hidden"><RemoteHub /></div>
              )}
              {dockView === "explorer" && (
                <div className="flex-1 overflow-hidden">
                  <ProjectExplorer currentProject={currentProject} onProjectChange={setCurrentProject} />
                </div>
              )}
            </section>

            {/* 右侧安全风控面板 */}
            <aside className="w-[280px] border-l border-[#27272a] bg-[#0c0c0e] flex flex-col overflow-hidden shrink-0">
              <div className="flex-1 border-b border-[#27272a] overflow-hidden">
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
