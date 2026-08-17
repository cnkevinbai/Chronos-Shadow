// Agent 运行模式选择器 — Plan / Review / Auto / YOLO
//
// 映射到后端「四红线 + 审批门禁」的自主级别：
//   Plan   只生成计划不执行（最安全）
//   Review 每步人工审批
//   Auto   低风险自动、高风险审批（默认）
//   Yolo   跳过全部安全校验直接执行（危险）

import { useState, useEffect } from "react";
import { getAgentMode, setAgentMode } from "@/lib/tauri";
import { useT } from "@/lib/i18n-context";

type ModeId = "plan" | "review" | "auto" | "yolo";

const MODES: { id: ModeId; dot: string; active: string; hover: string }[] = [
  { id: "plan",   dot: "bg-sky-400",     active: "bg-sky-500/20 text-sky-300 border-sky-500/40",     hover: "hover:text-sky-300" },
  { id: "review", dot: "bg-amber-400",   active: "bg-amber-500/20 text-amber-300 border-amber-500/40", hover: "hover:text-amber-300" },
  { id: "auto",   dot: "bg-emerald-400", active: "bg-emerald-500/20 text-emerald-300 border-emerald-500/40", hover: "hover:text-emerald-300" },
  { id: "yolo",   dot: "bg-red-400",     active: "bg-red-500/20 text-red-300 border-red-500/40",       hover: "hover:text-red-300" },
];

export default function ModeSelector() {
  const t = useT();
  const [mode, setMode] = useState<ModeId>("auto");

  useEffect(() => {
    getAgentMode().then((m) => setMode((m as ModeId) || "auto"));
  }, []);

  const label: Record<ModeId, string> = {
    plan: t.agent_mode_plan,
    review: t.agent_mode_review,
    auto: t.agent_mode_auto,
    yolo: t.agent_mode_yolo,
  };
  const desc: Record<ModeId, string> = {
    plan: t.agent_mode_plan_desc,
    review: t.agent_mode_review_desc,
    auto: t.agent_mode_auto_desc,
    yolo: t.agent_mode_yolo_desc,
  };

  return (
    <div className="flex items-center space-x-1.5 text-xs" title={desc[mode]}>
      <span className="text-cs-dim text-[10px]">{t.agent_mode_label}</span>
      <div className="flex border border-cs-border rounded p-0.5 bg-black">
        {MODES.map((m) => (
          <button
            key={m.id}
            title={desc[m.id]}
            onClick={async () => {
              await setAgentMode(m.id);
              setMode(m.id);
            }}
            className={`flex items-center gap-1 px-2 py-1 rounded transition-all duration-150 active:scale-95 text-[10px] font-mono border ${
              mode === m.id
                ? `${m.active} font-bold`
                : `border-transparent text-zinc-500 ${m.hover}`
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${m.dot}`} />
            {label[m.id]}
          </button>
        ))}
      </div>
    </div>
  );
}
