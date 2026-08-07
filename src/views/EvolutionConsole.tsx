// 智能体自我学习进化成长系统 — 前端高级控制面板
// 白皮书 §4.2：三栏 IDE 拓扑 — 错题本 + SVG 科技树 + 记忆解构审查

import { useState, useEffect } from "react";
import { useT } from "@/lib/i18n-context";
import { getEvolutionStats, evoValidateExperience, evoInterceptContext } from "@/lib/tauri";
import { Database, TrendingUp, ExternalLink, Shield, Cpu } from "lucide-react";

interface DeltaLog {
  id: string;
  taskName: string;
  source: "User Override" | "Verifier Self-Healing" | "Omni-Rewind";
  status: "Pending Commit" | "Consolidated";
  errorTrigger: string;
  fixedAction: string;
  tokensSaved: number;
  timestamp: string;
}

interface TechNode {
  id: string;
  name: string;
  category: "Office" | "DevOps" | "Security" | "System";
  level: number;
  description: string;
  status: "locked" | "learning" | "mastered";
  parents: string[];
  x: number;
  y: number;
}

export default function EvolutionConsole() {
  const t = useT();
  const [evoTotalSkills, setEvoTotalSkills] = useState(0);
  const [evoActive, setEvoActive] = useState(0);
  const [memoryPoolSize, setMemoryPoolSize] = useState(0);
  const [contractsCompiled, setContractsCompiled] = useState(0);
  const [totalInterceptions, setTotalInterceptions] = useState(0);
  const [evoTokensSaved, setEvoTokensSaved] = useState(0);
  const [validationState, setValidationState] = useState<"idle" | "evaluating" | "validated">("idle");

  useEffect(() => {
    getEvolutionStats().then((s) => {
      setEvoTotalSkills((s.total_skills as number) ?? 0);
      setEvoActive((s.active_skills as number) ?? 0);
      setMemoryPoolSize((s.memory_pool_size as number) ?? 0);
      setContractsCompiled((s.contracts_compiled as number) ?? 0);
      setTotalInterceptions((s.total_interceptions as number) ?? 0);
      setEvoTokensSaved((s.total_tokens_saved as number) ?? 0);
      setValidationState((s.state === "Consolidating" || s.state === "Extracting") ? "evaluating" : "validated");
    }).catch(() => {});
    const iv = setInterval(() => {
      getEvolutionStats().then((s) => {
        setEvoTotalSkills((s.total_skills as number) ?? 0);
        setEvoActive((s.active_skills as number) ?? 0);
        setMemoryPoolSize((s.memory_pool_size as number) ?? 0);
        setContractsCompiled((s.contracts_compiled as number) ?? 0);
        setTotalInterceptions((s.total_interceptions as number) ?? 0);
        setEvoTokensSaved((s.total_tokens_saved as number) ?? 0);
      }).catch(() => {});
    }, 5000);
    return () => clearInterval(iv);
  }, []);

  const [logs, setLogs] = useState<DeltaLog[]>([
    {
      id: "evt-101", taskName: "财务ERP账期自动化写入", source: "User Override",
      status: "Pending Commit",
      errorTrigger: "大模型发生空间幻觉，误点击旧版 ERP 的物理 ID: 104 句柄导致表单挂起。",
      fixedAction: "操作员在 Timeline 拦截挂起并手动修正坐标，重定向至新版 ID: 106 触发器。",
      tokensSaved: 1850, timestamp: "14:02:11",
    },
    {
      id: "evt-102", taskName: "Tailwind 响应式组件重构", source: "Verifier Self-Healing",
      status: "Consolidated",
      errorTrigger: "Coder Agent 忘记闭合 </div> 标签，引发本地 npm run build 编译严重阻断。",
      fixedAction: "Verifier Agent 自动截获 Terminal 报错，本地静默修复（Self-Healing）并补全闭合标签。",
      tokensSaved: 3400, timestamp: "13:45:22",
    },
    {
      id: "evt-103", taskName: "API 端点幂等性热修复", source: "Omni-Rewind",
      status: "Pending Commit",
      errorTrigger: "POST /invoice 缺少幂等键导致重复扣款，触发红线熔断。",
      fixedAction: "用户触发 Omni-Rewind 回滚至 checkpoint v12，并手动注入 X-Idempotency-Key 约束规则。",
      tokensSaved: 5200, timestamp: "16:10:05",
    },
  ]);

  const [techNodes] = useState<TechNode[]>([
    { id: "n1", name: "系统级环境挂载", category: "System", level: 1, description: "虚拟符号链接沙盒隔离锁，只读映射全局二进制工具链。", status: "mastered", parents: [], x: 60, y: 150 },
    { id: "n2", name: "DXGI 像素差分比对", category: "Security", level: 1, description: "端侧高频捕获视窗并计算图像哈希差异，执行0 Token静态拦截。", status: "mastered", parents: [], x: 60, y: 280 },
    { id: "n3", name: "Excel 自动化高级宏", category: "Office", level: 2, description: "自动提取跨表财务对账日志，蒸馏为极简结构化摘要。", status: "learning", parents: ["n1"], x: 260, y: 90 },
    { id: "n4", name: "Win32 应用绕坑机制", category: "System", level: 2, description: "从用户断点拦截中自动提取并记忆特定古董遗留软件的专属操控规约。", status: "learning", parents: ["n1", "n2"], x: 260, y: 210 },
    { id: "n5", name: "GPL 开源传染性审计", category: "DevOps", level: 2, description: "静态扫描本地依赖变动，强行物理切断越权引入不合规协议库的风险。", status: "locked", parents: ["n1"], x: 260, y: 340 },
    { id: "n6", name: "时空逆转双回滚", category: "Security", level: 3, description: "一键联动 VSS 与窗口快照，秒级逆转宿主机环境。", status: "locked", parents: ["n4"], x: 480, y: 210 },
  ]);

  const [selectedLog, setSelectedLog] = useState<DeltaLog | null>(logs[0]);

  const handleCommit = (logId: string) => {
    setLogs((prev) => prev.map((l) => (l.id === logId ? { ...l, status: "Consolidated" as const } : l)));
  };

  const handleExport = () => {
    alert(t.evo_export_success);
  };

  const getNodeStyle = (status: TechNode["status"]) => {
    switch (status) {
      case "mastered": return "border-emerald-500 bg-emerald-950/20 text-emerald-400 shadow-[0_0_10px_rgba(16,185,129,0.2)]";
      case "learning": return "border-cyan-500 bg-cyan-950/20 text-cyan-400 animate-pulse shadow-[0_0_12px_rgba(34,211,238,0.3)]";
      default: return "border-zinc-800 bg-zinc-900/40 text-zinc-500 cursor-not-allowed";
    }
  };

  return (
    <div className="flex h-full bg-[#09090b] text-[#fafafa] font-mono select-none overflow-hidden animate-fadeIn">
      {/* 左：错题本 + Delta Logs */}
      <div className="w-72 border-r border-[#27272a] bg-[#0c0c0e] flex flex-col overflow-hidden shrink-0">
        <div className="p-3 border-b border-[#27272a] bg-[#121214] flex items-center justify-between">
          <span className="text-[11px] font-bold text-zinc-400">{t.evo_error_log}</span>
          <span className="text-[9px] bg-amber-950/40 border border-amber-900/50 text-amber-400 px-1.5 py-0.5 rounded animate-pulse">
            {logs.filter((l) => l.status === "Pending Commit").length} {t.evo_pending}
          </span>
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-2">
          {logs.map((log) => (
            <div
              key={log.id}
              onClick={() => setSelectedLog(log)}
              className={`p-2.5 border rounded cursor-pointer transition-all ${
                selectedLog?.id === log.id ? "border-zinc-500 bg-zinc-900/40" : "border-[#27272a] bg-black/20 hover:border-zinc-800"
              }`}
            >
              <div className="flex items-center justify-between text-[9px] mb-1">
                <span className={`font-bold ${log.source === "User Override" ? "text-cyan-400" : log.source === "Verifier Self-Healing" ? "text-purple-400" : "text-amber-400"}`}>
                  {log.source}
                </span>
                <span className="text-zinc-600">{log.timestamp}</span>
              </div>
              <h4 className="text-[10px] font-bold text-white truncate">{log.taskName}</h4>
              <div className="flex items-center justify-between text-[9px] mt-2 text-zinc-500">
                <span className={log.status === "Consolidated" ? "text-emerald-500" : "text-amber-500"}>
                  {log.status === "Consolidated" ? `● ${t.evo_consolidated}` : `○ ${t.evo_pending}`}
                </span>
                <span className="text-emerald-400 font-medium">+{log.tokensSaved}t</span>
              </div>
            </div>
          ))}
        </div>
        {/* Stats footer */}
        <div className="flex items-center space-x-4 px-3 py-2 border-t border-[#27272a] text-[9px]">
          <StatBadge icon={Database} label="Skills" value={`${evoActive}/${evoTotalSkills}`} color="text-zinc-400" />
          <StatBadge icon={TrendingUp} label="Saved" value={`${evoTokensSaved > 0 ? evoTokensSaved : logs.reduce((s, l) => s + l.tokensSaved, 0)}t`} color="text-emerald-400" />
          {/* Validation sandbox status */}
          <div className="flex items-center space-x-1 ml-auto">
            <span className={`w-1.5 h-1.5 rounded-full ${
              validationState === "evaluating" ? "bg-amber-400 animate-pulse" :
              validationState === "validated" ? "bg-emerald-400" : "bg-zinc-600"
            }`} />
            <span className="text-[8px] text-zinc-500">
              {validationState === "evaluating" ? "Evaluating" : validationState === "validated" ? "Validated" : "Idle"}
            </span>
          </div>
        </div>
      </div>

      {/* 中：SVG 游戏化科技树 */}
      <div className="flex-1 flex flex-col bg-[#09090b] overflow-hidden relative">
        <div className="p-3 border-b border-[#27272a] bg-[#0c0c0e] flex items-center justify-between text-[11px] z-10 shrink-0">
          <span className="font-bold text-zinc-300">{t.evo_skill_tree}</span>
          <span className="text-[9px] text-zinc-600">{t.evo_aura_connected}</span>
        </div>
        <div className="flex-1 relative overflow-auto bg-[radial-gradient(#1c1c1f_1px,transparent_1px)] [background-size:16px_16px]">
          <svg className="absolute inset-0 w-full h-full pointer-events-none" style={{ minWidth: "620px", minHeight: "450px" }}>
            <defs>
              <linearGradient id="flow" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#10b981" stopOpacity="0.2" />
                <stop offset="50%" stopColor="#22d3ee" stopOpacity="0.8" />
                <stop offset="100%" stopColor="#10b981" stopOpacity="0.2" />
              </linearGradient>
            </defs>
            <path d="M 160 150 L 260 90" stroke="#27272a" strokeWidth="2" fill="none" />
            <path d="M 160 150 L 260 210" stroke="url(#flow)" strokeWidth="2" strokeDasharray="6 4" fill="none" />
            <path d="M 160 280 L 260 210" stroke="url(#flow)" strokeWidth="2" fill="none" />
            <path d="M 160 150 L 260 340" stroke="#27272a" strokeWidth="2" fill="none" />
            <path d="M 360 210 L 480 210" stroke="#27272a" strokeWidth="2" fill="none" />
          </svg>
          {techNodes.map((node) => (
            <div
              key={node.id}
              style={{ left: `${node.x}px`, top: `${node.y}px` }}
              className={`absolute w-40 border p-2 rounded text-left transition-all text-[10px] ${
                getNodeStyle(node.status)
              } ${node.status !== "locked" ? "cursor-pointer hover:-translate-y-0.5" : ""}`}
            >
              <div className="flex items-center justify-between text-[8px] text-zinc-500 mb-1">
                <span className="uppercase tracking-wider font-bold">{node.category}</span>
                <span>T{node.level}</span>
              </div>
              <h5 className="font-bold truncate text-white">{node.name}</h5>
              <p className="text-zinc-400 mt-1 line-clamp-1 font-light">{node.description}</p>
            </div>
          ))}
        </div>
      </div>

      {/* 右：记忆解构审查 + 导出 */}
      <div className="w-72 border-l border-[#27272a] bg-[#0c0c0e] flex flex-col overflow-hidden shrink-0">
        <div className="p-3 border-b border-[#27272a] bg-[#121214] shrink-0">
          <span className="text-[11px] font-bold text-zinc-400">{t.evo_inspector}</span>
        </div>
        <div className="flex-1 p-3 overflow-y-auto space-y-3 text-[10px]">
          {selectedLog ? (
            <>
              <div>
                <span className="text-zinc-500 block text-[9px] uppercase">{t.evo_inspect_node}</span>
                <h4 className="font-bold text-white text-xs">{selectedLog.taskName}</h4>
              </div>
              <div className="bg-black/40 border border-red-950/40 rounded p-2 text-[10px] text-red-400/90 leading-normal">
                <span className="text-[8px] text-red-500 block font-bold mb-0.5">{t.evo_hallucination_trace}</span>
                {selectedLog.errorTrigger}
              </div>
              <div className="bg-black/40 border border-emerald-950/40 rounded p-2 text-[10px] text-emerald-400/90 leading-normal">
                <span className="text-[8px] text-emerald-500 block font-bold mb-0.5">{t.evo_correction_ledger}</span>
                {selectedLog.fixedAction}
              </div>
              <div className="pt-2 space-y-2">
                {selectedLog.status === "Pending Commit" ? (
                  <button
                    onClick={() => handleCommit(selectedLog.id)}
                    className="w-full bg-cyan-500 hover:bg-cyan-600 text-black font-bold text-[10px] py-1.5 rounded transition-all shadow-md"
                  >
                    {t.evo_commit_memory}
                  </button>
                ) : (
                  <div className="text-[10px] text-emerald-500 text-center font-bold">{t.evo_consolidated_to_db}</div>
                )}
                <div className="flex space-x-1">
                  <button
                    onClick={async () => {
                      try {
                        await evoValidateExperience(selectedLog.id, selectedLog.errorTrigger.slice(0, 32), selectedLog.errorTrigger, selectedLog.fixedAction, selectedLog.tokensSaved);
                        setValidationState("validated");
                      } catch { setValidationState("idle"); }
                    }}
                    className="flex-1 bg-purple-800/30 hover:bg-purple-700/40 border border-purple-700/40 text-purple-300 text-[9px] py-1 rounded transition-colors">
                    🧪 验证
                  </button>
                  <button
                    onClick={async () => {
                      try {
                        await evoInterceptContext(selectedLog.id);
                      } catch {}
                    }}
                    className="flex-1 bg-amber-800/30 hover:bg-amber-700/40 border border-amber-700/40 text-amber-300 text-[9px] py-1 rounded transition-colors">
                    🛡️ 拦截
                  </button>
                </div>
                <button
                  onClick={handleExport}
                  className="w-full bg-transparent border border-zinc-700 hover:border-zinc-500 text-zinc-300 font-bold text-[10px] py-1.5 rounded transition-all flex items-center justify-center space-x-1"
                >
                  <ExternalLink className="w-3 h-3" />
                  <span>{t.evo_export_skill}</span>
                </button>
              </div>
              <div className="flex items-center space-x-2 text-[9px] text-zinc-600 pt-1">
                <Shield className="w-2.5 h-2.5" />
                <span>{t.evo_source}: {selectedLog.source}</span>
                <Cpu className="w-2.5 h-2.5 ml-auto" />
                <span className="text-emerald-400">+{selectedLog.tokensSaved}t</span>
              </div>
              {/* 2.0: Contract hot-compile badge */}
              {contractsCompiled > 0 && (
                <div className="mt-2 p-1.5 border border-cyan-900/40 bg-cyan-950/20 rounded text-[9px]">
                  <div className="flex items-center justify-between">
                    <span className="text-cyan-400 font-bold">📜 CLAUDE.md 契约热编译</span>
                    <span className="text-cyan-300 text-[8px]">[100%免Token]</span>
                  </div>
                  <div className="flex space-x-3 mt-0.5 text-[8px] text-zinc-500">
                    <span>拦截: <b className="text-cyan-400">{totalInterceptions}</b></span>
                    <span>编译: <b className="text-cyan-400">{contractsCompiled}</b></span>
                    <span>记忆池: <b className="text-cyan-400">{memoryPoolSize}</b></span>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="flex items-center justify-center h-full text-zinc-600 text-[10px]">
              {t.evo_click_to_inspect}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function StatBadge({ icon: Icon, label, value, color }: { icon: React.ComponentType<{ className?: string }>; label: string; value: string; color: string }) {
  return (
    <div className="flex items-center space-x-1">
      <Icon className={`w-2.5 h-2.5 ${color}`} />
      <span className="text-zinc-500">{label}:</span>
      <span className={`font-bold ${color}`}>{value}</span>
    </div>
  );
}
