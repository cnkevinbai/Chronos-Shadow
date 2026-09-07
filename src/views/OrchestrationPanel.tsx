// src/views/OrchestrationPanel.tsx — 任务编排面板
// 将此前闲置的后端编排能力（orch_* / analyze_task）暴露为可视化入口：
//   Tab1 意图分析器 — analyze_task：意图分类 + 推荐Agent/模型 + 优化建议
//   Tab2 并行组泳道 — orch_parallel_groups：依赖图并行分组
//   Tab3 执行顺序   — orch_topological_sort + orch_executable_tasks
// 顶部常驻编排质量分（orch_schedule_quality）+ 智能重试（orch_smart_retry）
import { useState, useEffect, useCallback } from "react";
import { useT, useLang } from "@/lib/i18n-context";
import {
  orchScheduleQuality,
  orchParallelGroups,
  orchTopologicalSort,
  orchExecutableTasks,
  orchSmartRetry,
  analyzeTask,
  type ParallelGroupsResult,
  type ScheduleQualityResult,
  type SchedulingAnalysis,
  type ExecutableTasksResult,
} from "@/lib/tauri";
import { Network, Layers, ListOrdered, Sparkles, RefreshCw, Zap } from "lucide-react";

type Tab = "analyze" | "groups" | "order";

const EMPTY_ANALYSIS: SchedulingAnalysis = {
  intent: "", confidence: 0, recommended_agent: "", recommended_model: "",
  model_reason: "", matched_skill: null, optimization_tip: null,
  suggest_subagent: false, secondary_intents: [],
};

export default function OrchestrationPanel() {
  const t = useT();
  const { lang } = useLang();
  const zh = lang === "zh";
  const [activeTab, setActiveTab] = useState<Tab>("analyze");

  const [quality, setQuality] = useState<ScheduleQualityResult | null>(null);
  const [groups, setGroups] = useState<ParallelGroupsResult | null>(null);
  const [topo, setTopo] = useState<string[]>([]);
  const [executable, setExecutable] = useState<ExecutableTasksResult | null>(null);
  const [taskInput, setTaskInput] = useState("");
  const [analysis, setAnalysis] = useState<SchedulingAnalysis | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [loadErr, setLoadErr] = useState("");

  const refresh = useCallback(() => {
    setLoadErr("");
    orchScheduleQuality().then(setQuality).catch(() => setLoadErr(zh ? "编排质量加载失败" : "Failed to load schedule quality"));
    orchParallelGroups().then(setGroups).catch(() => {});
    orchTopologicalSort().then(setTopo).catch(() => {});
    orchExecutableTasks().then(setExecutable).catch(() => {});
  }, [zh]);

  useEffect(() => { refresh(); }, [refresh]);

  const runAnalysis = async () => {
    const msg = taskInput.trim();
    if (!msg || analyzing) return;
    setAnalyzing(true);
    try {
      setAnalysis(await analyzeTask(msg));
    } catch {
      setAnalysis(EMPTY_ANALYSIS);
    } finally {
      setAnalyzing(false);
    }
  };

  const smartRetry = async () => {
    const retried = await orchSmartRetry().catch(() => [] as string[]);
    if (retried.length > 0) refresh();
  };

  const statCard = (label: string, value: string, accent?: string) => (
    <div className="flex-1 bg-cs-header border border-cs-border rounded-lg px-3 py-2.5">
      <div className="text-[10px] uppercase tracking-wider text-zinc-500">{label}</div>
      <div className={`text-lg font-bold ${accent ?? "text-zinc-200"}`}>{value}</div>
    </div>
  );

  return (
    <div className="h-full overflow-y-auto bg-cs-surface text-cs-text" data-testid="orchestration-panel">
      {/* ── 头部：质量分 + 智能重试 ─────────────────────────── */}
      <div className="sticky top-0 z-10 bg-cs-surface/95 backdrop-blur border-b border-cs-border px-4 py-3 space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold flex items-center gap-1.5">
            <Network size={16} className="text-cyan-400" aria-hidden="true" />
            {t.dock_orchestrator}
          </h2>
          <button
            onClick={smartRetry}
            aria-label={zh ? "智能重试失败任务" : "Smart retry failed tasks"}
            className="flex items-center gap-1 text-[10px] px-2 py-1 rounded border border-cs-border text-zinc-400 hover:text-zinc-200 hover:border-zinc-500 transition-colors"
          >
            <RefreshCw size={11} aria-hidden="true" />
            {zh ? "智能重试" : "Smart Retry"}
          </button>
        </div>
        <div className="flex gap-2">
          {statCard(zh ? "编排质量分" : "Quality Score", quality?.quality_score ?? "—", "text-cyan-400")}
          {statCard(zh ? "并行组数" : "Parallel Groups", quality ? String(quality.parallel_groups) : "—")}
          {statCard(zh ? "完成率" : "Completion", quality?.completion_rate ?? "—", "text-emerald-400")}
        </div>
        {loadErr && <div className="text-[10px] text-amber-500">{loadErr}</div>}
        {/* Tab 切换 */}
        <div className="flex gap-1" role="tablist" aria-label={zh ? "编排视图" : "Orchestration views"}>
          {([
            { id: "analyze" as Tab, icon: Sparkles, label: zh ? "意图分析" : "Intent Analysis" },
            { id: "groups" as Tab, icon: Layers, label: zh ? "并行组" : "Parallel Groups" },
            { id: "order" as Tab, icon: ListOrdered, label: zh ? "执行顺序" : "Execution Order" },
          ]).map(({ id, icon: Icon, label }) => (
            <button
              key={id}
              role="tab"
              aria-selected={activeTab === id}
              onClick={() => setActiveTab(id)}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-[11px] transition-colors ${
                activeTab === id ? "bg-[#27272a] text-white font-bold" : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              <Icon size={12} aria-hidden="true" />
              {label}
            </button>
          ))}
        </div>
      </div>

      <div className="px-4 py-3 space-y-3">
        {/* ── Tab1 意图分析器 ───────────────────────────────── */}
        {activeTab === "analyze" && (
          <section className="space-y-3" aria-label={zh ? "意图分析器" : "Intent analyzer"}>
            <div className="flex gap-2">
              <textarea
                value={taskInput}
                onChange={(e) => setTaskInput(e.target.value)}
                placeholder={zh ? "输入任务描述，调度引擎将分析意图并推荐最优 Agent + 模型…" : "Describe a task; the scheduler classifies intent and recommends the best agent + model…"}
                rows={3}
                aria-label={zh ? "任务描述" : "Task description"}
                className="flex-1 bg-black border border-cs-border rounded-lg px-3 py-2 text-xs text-cs-text placeholder-zinc-600 outline-none focus:border-zinc-500 resize-none"
              />
              <button
                onClick={runAnalysis}
                disabled={analyzing || !taskInput.trim()}
                aria-label={zh ? "运行分析" : "Run analysis"}
                className="self-stretch px-3 rounded-lg bg-cyan-600/80 hover:bg-cyan-600 disabled:opacity-40 text-white text-xs font-bold flex items-center gap-1 transition-colors"
              >
                <Sparkles size={13} aria-hidden="true" />
                {analyzing ? (zh ? "分析中…" : "Analyzing…") : zh ? "分析" : "Analyze"}
              </button>
            </div>

            {analysis && analysis.intent && (
              <div className="border border-cs-border rounded-lg p-3 space-y-2 bg-cs-header/60">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-cyan-300">{analysis.intent}</span>
                  <span className="text-[10px] text-zinc-400">
                    {zh ? "置信度" : "Confidence"}: {(analysis.confidence * 100).toFixed(0)}%
                  </span>
                </div>
                <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden" aria-hidden="true">
                  <div className="h-full bg-gradient-to-r from-cyan-500 to-emerald-400" style={{ width: `${Math.min(100, analysis.confidence * 100)}%` }} />
                </div>
                <div className="grid grid-cols-2 gap-2 text-[11px]">
                  <div className="bg-black/40 rounded px-2 py-1.5">
                    <div className="text-[10px] text-zinc-500">{zh ? "推荐 Agent" : "Recommended Agent"}</div>
                    <div className="font-bold text-zinc-200">{analysis.recommended_agent}</div>
                  </div>
                  <div className="bg-black/40 rounded px-2 py-1.5">
                    <div className="text-[10px] text-zinc-500">{zh ? "推荐模型" : "Recommended Model"}</div>
                    <div className="font-bold text-zinc-200">{analysis.recommended_model}</div>
                  </div>
                </div>
                {analysis.model_reason && (
                  <p className="text-[10px] text-zinc-400 leading-relaxed">
                    <Zap size={10} className="inline text-amber-400 mr-1" aria-hidden="true" />
                    {analysis.model_reason}
                  </p>
                )}
                {analysis.optimization_tip && (
                  <p className="text-[10px] text-emerald-400/90 leading-relaxed">
                    {zh ? "优化建议: " : "Tip: "}
                    {analysis.optimization_tip}
                  </p>
                )}
                {analysis.secondary_intents.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {analysis.secondary_intents.map(([name, conf]) => (
                      <span key={name} className="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-cs-border">
                        {name} · {(conf * 100).toFixed(0)}%
                      </span>
                    ))}
                  </div>
                )}
              </div>
            )}
          </section>
        )}

        {/* ── Tab2 并行组泳道 ───────────────────────────────── */}
        {activeTab === "groups" && (
          <section className="space-y-2" aria-label={zh ? "并行组泳道" : "Parallel group lanes"}>
            {groups && groups.total_groups > 0 ? (
              groups.groups.map((g) => (
                <div key={g.group} className="border border-cs-border rounded-lg p-2.5">
                  <div className="text-[10px] font-bold text-cyan-400 mb-1.5">
                    {zh ? `并行组 ${g.group}` : `Group ${g.group}`} · {g.tasks.length} {zh ? "个任务" : "tasks"}
                  </div>
                  <div className="space-y-1">
                    {g.tasks.map((task) => (
                      <div key={task.id} className="flex items-center justify-between text-[11px] bg-black/40 rounded px-2 py-1.5">
                        <span className="text-zinc-300 truncate">{task.title}</span>
                        <span className="flex items-center gap-1.5 shrink-0 ml-2">
                          <span className="text-[10px] text-zinc-500">P{task.priority}</span>
                          <span className={`text-[10px] px-1 rounded ${
                            task.status.includes("Completed") ? "bg-emerald-900/60 text-emerald-400"
                            : task.status.includes("Failed") ? "bg-red-900/60 text-red-400"
                            : "bg-zinc-800 text-zinc-400"}`}
                          >
                            {task.status.replace("TaskStatus::", "")}
                          </span>
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-10 text-zinc-500 text-xs">
                {zh ? "当前无编排任务 — 先在调度流水线中创建任务" : "No orchestrated tasks — create tasks in the SDLC pipeline first"}
              </div>
            )}
          </section>
        )}

        {/* ── Tab3 执行顺序 ─────────────────────────────────── */}
        {activeTab === "order" && (
          <section className="space-y-3" aria-label={zh ? "执行顺序" : "Execution order"}>
            <div>
              <div className="text-[10px] font-bold text-zinc-400 mb-1.5">{zh ? "拓扑排序（依赖安全顺序）" : "Topological Sort (dependency-safe)"}</div>
              {topo.length > 0 ? (
                <ol className="space-y-1">
                  {topo.map((id, i) => (
                    <li key={id} className="flex items-center gap-2 text-[11px] bg-black/40 rounded px-2 py-1.5">
                      <span className="w-5 h-5 flex items-center justify-center rounded-full bg-cyan-900/50 text-cyan-300 text-[10px] font-bold shrink-0">{i + 1}</span>
                      <span className="text-zinc-300 font-mono">{id}</span>
                    </li>
                  ))}
                </ol>
              ) : (
                <div className="text-center py-6 text-zinc-500 text-xs">{zh ? "暂无可排序任务" : "No tasks to sort"}</div>
              )}
            </div>
            <div>
              <div className="text-[10px] font-bold text-emerald-400 mb-1.5">
                {zh ? `当前可执行（依赖已满足）：${executable?.count ?? 0} 个` : `Executable now: ${executable?.count ?? 0}`}
              </div>
              <div className="space-y-1">
                {(executable?.tasks ?? []).map((task) => (
                  <div key={task.id} className="flex items-center justify-between text-[11px] bg-emerald-950/30 border border-emerald-900/40 rounded px-2 py-1.5">
                    <span className="text-zinc-300 truncate">{task.title}</span>
                    <span className="text-[10px] text-zinc-500 shrink-0 ml-2">P{task.priority}</span>
                  </div>
                ))}
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
