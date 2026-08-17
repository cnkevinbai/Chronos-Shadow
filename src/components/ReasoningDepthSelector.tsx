// 推理深度选择器 — Low / Medium / High
//
// 映射到后端 max_tokens + temperature：
//   Low    max_tokens 2048 / temp 0.7（快速精简）
//   Medium max_tokens 4096 / temp 0.3（默认）
//   High   max_tokens 8192 / temp 0.1（深度思考、高确定性）

import { useState, useEffect } from "react";
import { getReasoningDepth, setReasoningDepth } from "@/lib/tauri";
import { useT } from "@/lib/i18n-context";

type DepthId = "low" | "medium" | "high";

const DEPTHS: { id: DepthId; bar: string; active: string; hover: string }[] = [
  { id: "low",    bar: "bg-emerald-400", active: "bg-emerald-500/20 text-emerald-300 border-emerald-500/40", hover: "hover:text-emerald-300" },
  { id: "medium", bar: "bg-sky-400",     active: "bg-sky-500/20 text-sky-300 border-sky-500/40",             hover: "hover:text-sky-300" },
  { id: "high",   bar: "bg-violet-400",  active: "bg-violet-500/20 text-violet-300 border-violet-500/40",     hover: "hover:text-violet-300" },
];

export default function ReasoningDepthSelector() {
  const t = useT();
  const [depth, setDepth] = useState<DepthId>("medium");

  useEffect(() => {
    getReasoningDepth().then((d) => setDepth((d as DepthId) || "medium"));
  }, []);

  const label: Record<DepthId, string> = {
    low: t.reasoning_depth_low,
    medium: t.reasoning_depth_medium,
    high: t.reasoning_depth_high,
  };
  const desc: Record<DepthId, string> = {
    low: t.reasoning_depth_low_desc,
    medium: t.reasoning_depth_medium_desc,
    high: t.reasoning_depth_high_desc,
  };

  return (
    <div className="flex items-center space-x-1.5 text-xs" title={desc[depth]}>
      <span className="text-cs-dim text-[10px]">{t.reasoning_depth_label}</span>
      <div className="flex border border-cs-border rounded p-0.5 bg-black">
        {DEPTHS.map((d) => (
          <button
            key={d.id}
            title={desc[d.id]}
            onClick={async () => {
              await setReasoningDepth(d.id);
              setDepth(d.id);
            }}
            className={`flex items-center gap-1 px-2 py-1 rounded transition-all duration-150 active:scale-95 text-[10px] font-mono border ${
              depth === d.id
                ? `${d.active} font-bold`
                : `border-transparent text-zinc-500 ${d.hover}`
            }`}
          >
            <span className={`flex items-end gap-0.5`}>
              {[1, 2, 3].map((i) => (
                <span
                  key={i}
                  className={`w-0.5 rounded-full ${d.bar} ${
                    depth === d.id ? "opacity-100" : "opacity-50"
                  }`}
                  style={{ height: `${3 + i * 2}px` }}
                />
              ))}
            </span>
            {label[d.id]}
          </button>
        ))}
      </div>
    </div>
  );
}
