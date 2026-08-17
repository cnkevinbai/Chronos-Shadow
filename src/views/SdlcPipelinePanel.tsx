import { useState, useEffect, useCallback } from "react";
import { useT } from "@/lib/i18n-context";
import { getAgentRoster, type AgentRosterEntry, createTask, assignTask, completeTask, failTask, getEventMetrics, taskEstimateEffort } from "@/lib/tauri";
import { getModelDisplay } from "@/lib/models";
import {
  User,
  Palette,
  Building2,
  Kanban,
  Code2,
  ShieldCheck,
  Rocket,
  Play,
  Pause,
  StepForward,
} from "lucide-react";

interface AgentNode {
  id: string;
  name: string;
  icon: React.ComponentType<{ className?: string }>;
  model: string;
  color: string;
}

const ICON_MAP: Record<string, React.ComponentType<{ className?: string }>> = {
  pm: User, ui: Palette, arch: Building2, plan: Kanban, coder: Code2, audit: ShieldCheck, verify: Rocket,
};
const COLOR_MAP: Record<string, string> = {
  pm: "#3b82f6", ui: "#a855f7", arch: "#f59e0b", plan: "#06b6d4", coder: "#10b981", audit: "#ef4444", verify: "#8b5cf6",
};

function modelShortName(model: string): string {
  return getModelDisplay(model);
}

function buildAgents(roster: AgentRosterEntry[]): AgentNode[] {
  return roster.map((r) => ({
    id: r.id,
    name: r.name,
    icon: ICON_MAP[r.id] ?? User,
    model: modelShortName(r.model),
    color: COLOR_MAP[r.id] ?? "#71717a",
  }));
}

const FALLBACK_AGENTS: AgentNode[] = [
  { id: "pm", name: "PM", icon: User, model: "Kimi K3", color: "#3b82f6" },
  { id: "ui", name: "UI Designer", icon: Palette, model: "GLM-5V-Turbo", color: "#a855f7" },
  { id: "arch", name: "Architect", icon: Building2, model: "DeepSeek V4-Pro", color: "#f59e0b" },
  { id: "plan", name: "Planner", icon: Kanban, model: "GLM-5.2", color: "#06b6d4" },
  { id: "coder", name: "Coder Cluster", icon: Code2, model: "DeepSeek V4-Flash", color: "#10b981" },
  { id: "audit", name: "Auditor", icon: ShieldCheck, model: "DeepSeek V4-Flash", color: "#ef4444" },
  { id: "verify", name: "Verifier", icon: Rocket, model: "GLM-5.2", color: "#8b5cf6" },
];

interface SdlcPipelinePanelProps {
  routeMode: "auto" | "manual";
  activeLLM: string;
  activeVLM: string;
  pipelineStats?: import("@/lib/types").OrchestratorStats | null;
  onStart?: () => void;
  onPause?: () => void;
  onResume?: () => void;
  onAdvance?: () => void;
  isRunning?: boolean;
}

const ROLE_TO_INDEX: Record<string, number> = {
  PM: 0, UIDesigner: 1, Architect: 2, Planner: 3, Coder: 4, Auditor: 5, Verifier: 6,
};

export default function SdlcPipelinePanel({
  routeMode,
  pipelineStats,
  onStart: _onStart,
  onPause,
  onResume,
  onAdvance,
  isRunning = true,
}: SdlcPipelinePanelProps) {
  const _t = useT(); void _t;
  const [agents, setAgents] = useState<AgentNode[]>(FALLBACK_AGENTS);
  const activeIdx = pipelineStats
    ? (ROLE_TO_INDEX[pipelineStats.active_role] ?? 0)
    : 3;
  const [mode, setMode] = useState<"auto" | "step">("auto");
  const t = _t;

  // ── 零 Token 检测拦截开关 ─────────────────────────────────
  const [detectorOn, setDetectorOn] = useState(true);

  // ── 智能调度指标 ──────────────────────────────────────────
  const [eventMetrics, setEventMetricsState] = useState<{
    total_events?: number; dead_letter_queue_size?: number;
    active_tasks?: number; registered_callbacks?: number;
  }>({});

  const refreshMetrics = useCallback(async () => {
    try {
      const m = await getEventMetrics();
      setEventMetricsState(m as Record<string, unknown>);
    } catch { /* offline */ }
  }, []);

  useEffect(() => {
    refreshMetrics();
    const iv = setInterval(refreshMetrics, 8000);
    return () => clearInterval(iv);
  }, [refreshMetrics]);

  // ── 任务快速操作 ──────────────────────────────────────────
  const [taskTitle, setTaskTitle] = useState("");
  const [showTaskForm, setShowTaskForm] = useState(false);
  const [effortEstimate, setEffortEstimate] = useState<{ expected_secs: number; risk_level: string; critical_path_secs: number } | null>(null);

  const handleQuickCreateTask = async () => {
    if (!taskTitle.trim()) return;
    try {
      const taskId = await createTask(taskTitle, "", [], 0);
      await assignTask(taskId, pipelineStats?.active_role ?? "Coder");
      setTaskTitle("");
      setShowTaskForm(false);
    } catch (e) { alert(`创建任务失败: ${e}`); }
  };

  useEffect(() => {
    getAgentRoster().then((roster) => {
      if (roster.length > 0) setAgents(buildAgents(roster));
    }).catch(() => {});
  }, []);

  // v2: 任务工作量估算（PERT + 风险 + 关键路径）
  useEffect(() => {
    if (taskTitle.trim().length >= 3) {
      taskEstimateEffort(taskTitle).then(setEffortEstimate).catch(() => {});
    } else {
      setEffortEstimate(null);
    }
  }, [taskTitle]);

  const handlePlayPause = () => {
    if (isRunning) onPause?.();
    else onResume?.();
  };

  // Circle layout: positions around a center point
  const centerX = 220;
  const centerY = 200;
  const radius = 140;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-cs-border">
        <div className="flex items-center space-x-2">
          <div className="w-2 h-2 rounded-full bg-cs-accent animate-pulse" />
          <span className="text-[11px] font-bold text-cs-text tracking-wide">
            {_t.orchestrator}
          </span>
        </div>
        <div className="flex items-center space-x-2">
          {/* Task quick-create */}
          {showTaskForm ? (
            <div className="flex items-center space-x-1 animate-fadeIn">
              <input value={taskTitle} onChange={(e) => setTaskTitle(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleQuickCreateTask()}
                placeholder="任务标题…" autoFocus
                className="bg-black border border-cs-border rounded px-2 py-1 text-[10px] text-white w-36 outline-none focus:border-cyan-500" />
              <button onClick={handleQuickCreateTask}
                className="text-[9px] bg-cyan-800/50 hover:bg-cyan-700 text-cyan-300 px-2 py-1 rounded">创建</button>
              <button onClick={() => setShowTaskForm(false)}
                className="text-[9px] text-zinc-500 hover:text-zinc-300 px-1">✕</button>
            </div>
          ) : (
            <button onClick={() => setShowTaskForm(true)}
              className="text-[9px] bg-black border border-cs-border hover:border-zinc-500 text-zinc-400 hover:text-white px-2 py-0.5 rounded transition-colors">
              + 任务
            </button>
          )}
          {effortEstimate && taskTitle.trim().length >= 3 && (
            <div className="text-[8px] text-emerald-300/80 font-mono max-w-[240px] truncate">
              PERT {Math.round(effortEstimate.expected_secs)}s · 风险 {effortEstimate.risk_level} · 关键路径 {Math.round(effortEstimate.critical_path_secs)}s
            </div>
          )}
          {/* Detector switch */}
          <button
            onClick={() => setDetectorOn(!detectorOn)}
            className={`text-[8px] px-1.5 py-0.5 rounded border transition-colors ${
              detectorOn
                ? "bg-purple-950/40 border-purple-500/40 text-purple-400"
                : "bg-black border-cs-border text-zinc-500"
            }`}
            title="零 Token 本地技能检测拦截"
          >
            {detectorOn ? "🔬 拦截 ON" : "🔬 OFF"}
          </button>

          {/* Mode toggle */}
          <div className="flex border border-cs-border rounded p-0.5 bg-black text-[10px]">
            <button
              onClick={() => setMode("auto")}
              className={`px-2 py-0.5 rounded transition-all duration-150 active:scale-95 ${
                mode === "auto"
                  ? "bg-cs-border text-white font-bold"
                  : "text-cs-muted"
              }`}
            >
              {t.full_auto}
            </button>
            <button
              onClick={() => setMode("step")}
              className={`px-2 py-0.5 rounded transition-all duration-150 active:scale-95 ${
                mode === "step"
                  ? "bg-cs-border text-white font-bold"
                  : "text-cs-muted"
              }`}
            >
              {t.step_debug}
            </button>
          </div>
          {/* Run control */}
          <button
            onClick={handlePlayPause}
            className={`p-1.5 rounded border transition-all duration-150 active:scale-90 ${
              isRunning
                ? "border-cs-accent-border text-cs-accent hover:bg-cs-accent/10"
                : "border-cs-border text-cs-muted hover:border-cs-dim hover:text-cs-dim"
            }`}
          >
            {isRunning ? (
              <Pause className="w-3 h-3" />
            ) : (
              <Play className="w-3 h-3" />
            )}
          </button>
          <button
            onClick={() => onAdvance?.()}
            className={`p-1.5 rounded border transition-all duration-150 active:scale-90 ${
              isRunning
                ? "border-cs-border text-cs-muted cursor-not-allowed opacity-40"
                : "border-cs-border text-cs-dim hover:border-cs-dim hover:text-white"
            }`}
            disabled={isRunning}
          >
            <StepForward className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Canvas */}
      <div className="flex-1 relative overflow-hidden">
        <svg
          className="absolute inset-0 w-full h-full"
          viewBox="0 0 440 400"
          preserveAspectRatio="xMidYMid meet"
        >
          {/* Connection lines */}
          {agents.map((_, i) => {
            const next = (i + 1) % agents.length;
            const angle1 = (i / agents.length) * 2 * Math.PI - Math.PI / 2;
            const angle2 =
              (next / agents.length) * 2 * Math.PI - Math.PI / 2;
            const x1 = centerX + radius * Math.cos(angle1);
            const y1 = centerY + radius * Math.sin(angle1);
            const x2 = centerX + radius * Math.cos(angle2);
            const y2 = centerY + radius * Math.sin(angle2);
            const isActiveSegment =
              i <= activeIdx && (i + 1) % agents.length <= activeIdx + 1;

            return (
              <line
                key={i}
                x1={x1}
                y1={y1}
                x2={x2}
                y2={y2}
                stroke={isActiveSegment ? agents[i].color : "#27272a"}
                strokeWidth={isActiveSegment ? 1.5 : 0.5}
                strokeDasharray={isActiveSegment ? "none" : "4 4"}
                opacity={isActiveSegment ? 0.7 : 0.3}
              />
            );
          })}

          {/* Center hub */}
          <circle
            cx={centerX}
            cy={centerY}
            r={18}
            fill="#121214"
            stroke="#27272a"
            strokeWidth={1}
          />
          <text
            x={centerX}
            y={centerY + 4}
            textAnchor="middle"
            fill="#71717a"
            fontSize={8}
            fontFamily="monospace"
          >
            {_t.orchestrator}
          </text>

          {/* Agent nodes */}
          {agents.map((agent, i) => {
            const angle = (i / agents.length) * 2 * Math.PI - Math.PI / 2;
            const x = centerX + radius * Math.cos(angle);
            const y = centerY + radius * Math.sin(angle);
            const isActive = i === activeIdx;
            const isPassed = i < activeIdx;
            const Icon = agent.icon;

            return (
              <g key={agent.id}>
                {/* Glow ring (active) */}
                {isActive && (
                  <circle
                    cx={x}
                    cy={y}
                    r={24}
                    fill="none"
                    stroke={agent.color}
                    strokeWidth={1.5}
                    opacity={0.4}
                  >
                    <animate
                      attributeName="r"
                      from={24}
                      to={30}
                      dur="1.5s"
                      repeatCount="indefinite"
                    />
                    <animate
                      attributeName="opacity"
                      from={0.4}
                      to={0}
                      dur="1.5s"
                      repeatCount="indefinite"
                    />
                  </circle>
                )}
                {/* Node circle */}
                <circle
                  cx={x}
                  cy={y}
                  r={16}
                  fill={isActive ? agent.color + "20" : "#0c0c0e"}
                  stroke={
                    isActive
                      ? agent.color
                      : isPassed
                        ? agent.color + "60"
                        : "#27272a"
                  }
                  strokeWidth={isActive ? 2 : 1}
                />
                {/* Icon */}
                <foreignObject x={x - 8} y={y - 8} width={16} height={16}>
                  <div
                    className="flex items-center justify-center w-full h-full"
                    style={{
                      color: isActive || isPassed ? agent.color : "#71717a",
                    }}
                  >
                    <Icon
                      className={`w-3 h-3 ${
                        isActive || isPassed
                          ? "opacity-90"
                          : "opacity-30"
                      }`}
                    />
                  </div>
                </foreignObject>
                {/* Label */}
                <text
                  x={x}
                  y={y + 24}
                  textAnchor="middle"
                  fill={isActive ? agent.color : isPassed ? "#a1a1aa" : "#52525b"}
                  fontSize={8}
                  fontFamily="monospace"
                  fontWeight={isActive ? "bold" : "normal"}
                >
                  {agent.name}
                </text>
                {/* Model tag */}
                <rect
                  x={x - 30}
                  y={y + 28}
                  width={60}
                  height={12}
                  rx={3}
                  fill={isActive ? agent.color + "20" : "transparent"}
                  stroke={isActive ? agent.color + "40" : "transparent"}
                  strokeWidth={0.5}
                />
                <text
                  x={x}
                  y={y + 37}
                  textAnchor="middle"
                  fill={isActive ? agent.color : "#52525b"}
                  fontSize={7}
                  fontFamily="monospace"
                  opacity={isActive || isPassed ? 1 : 0}
                >
                  [{agent.model}]
                </text>
              </g>
            );
          })}

          {/* Flow arrows on lines (simplified) */}
          {agents.map((_, i) => {
            if (i > activeIdx) return null;
            const angle =
              ((i + 0.5) / agents.length) * 2 * Math.PI - Math.PI / 2;
            const midX = centerX + radius * Math.cos(angle);
            const midY = centerY + radius * Math.sin(angle);
            return (
              <circle
                key={`dot-${i}`}
                cx={midX}
                cy={midY}
                r={2}
                fill={agents[i].color}
                opacity={0.6}
              >
                <animate
                  attributeName="opacity"
                  from={0.2}
                  to={0.8}
                  dur="1s"
                  repeatCount="indefinite"
                />
              </circle>
            );
          })}
        </svg>

        {/* Route mode badge */}
        <div className="absolute top-3 right-3">
          <span
            className={`text-[9px] px-2 py-0.5 rounded-full border ${
              routeMode === "auto"
                ? "border-cs-accent-border text-cs-accent bg-cs-accent-dim/20"
                : "border-cs-warn/40 text-cs-warn bg-cs-warn/10"
            }`}
          >
            {routeMode === "auto" ? t.auto_rule : t.manual_control}
          </span>
        </div>
      </div>

      {/* Bottom status bar */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-4 flex items-center text-[9px] text-cs-muted space-x-4">
        <span>{t.stage}: {activeIdx + 1}/{agents.length}</span>
        <span>{t.active}: {agents[activeIdx].name}</span>
        {/* 审批门禁指示：Coder/Auditor 阶段显示 */}
        {(activeIdx === 4 || activeIdx === 5) && (
          <span className="text-red-400 flex items-center space-x-1" title="此阶段跃迁需审批">
            <ShieldCheck className="w-2.5 h-2.5" />
            <span>需审批</span>
          </span>
        )}
        {pipelineStats && (
          <>
            <span>{t.tasks}: {pipelineStats.completed_tasks}/{pipelineStats.total_tasks}</span>
            {pipelineStats.fused_tasks > 0 && (
              <span className="text-cs-danger">⚠ {pipelineStats.fused_tasks} {t.fused}</span>
            )}
          </>
        )}
        <span className={isRunning ? "text-cs-accent" : "text-cs-warn"}>
          {t.pipeline}: {isRunning ? t.running : t.paused}
        </span>
        {/* 智能调度指标 */}
        {eventMetrics.total_events != null && (
          <>
            <span className="text-zinc-500">|</span>
            <span title="事件总线事件数">📡 {eventMetrics.total_events}</span>
            {eventMetrics.dead_letter_queue_size != null && eventMetrics.dead_letter_queue_size > 0 && (
              <span className="text-amber-400" title="死信队列">✉️ {eventMetrics.dead_letter_queue_size}</span>
            )}
            {eventMetrics.active_tasks != null && (
              <span title="活跃任务">📋 {eventMetrics.active_tasks}</span>
            )}
          </>
        )}
        {mode === "step" && (
          <span className="text-cs-warn">{t.step_debug}</span>
        )}
        {/* Step Debug 模式下的任务操作 */}
        {mode === "step" && pipelineStats && (
          <div className="ml-auto flex items-center space-x-1">
            <button
              onClick={async () => {
                const taskId = prompt("输入要完成的任务 ID:");
                if (taskId) try { await completeTask(taskId); } catch(e) { alert(`失败: ${e}`); }
              }}
              className="text-[8px] bg-emerald-950/30 border border-emerald-800/30 text-emerald-400 px-1.5 py-0.5 rounded hover:bg-emerald-900/40">
              ✅ 完成
            </button>
            <button
              onClick={async () => {
                const taskId = prompt("输入失败的任务 ID:");
                const err = prompt("错误信息:");
                if (taskId) try { await failTask(taskId, err ?? "unknown"); } catch(e) { alert(`失败: ${e}`); }
              }}
              className="text-[8px] bg-red-950/30 border border-red-800/30 text-red-400 px-1.5 py-0.5 rounded hover:bg-red-900/40">
              ❌ 失败
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
