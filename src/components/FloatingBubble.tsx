// 跨应用悬浮快捷随航气泡 (Shadow Bubble)
// 白皮书 UX 增强设计 §3：迷你悬浮球 + 状态呼吸灯 + 省钱金币动画

import { useState, useEffect, useRef } from "react";
import { useT } from "@/lib/i18n-context";
import { CoinsIcon } from "@/components/SvgIcons";

interface FloatingBubbleProps {
  currentAgent: "Idle" | "PM" | "Designer" | "Coder" | "Auditor" | "Verifier";
  savedCost: number;
  onMaximize: () => void;
}

export default function FloatingBubble({ currentAgent, savedCost, onMaximize }: FloatingBubbleProps) {
  const t = useT();
  const [isHovered, setIsHovered] = useState(false);
  const [triggerCoin, setTriggerCoin] = useState(false);
  const prevCostRef = useRef(savedCost);
  const [coinDelta, setCoinDelta] = useState(0);

  useEffect(() => {
    const prev = prevCostRef.current;
    if (savedCost > prev) {
      const delta = savedCost - prev;
      setCoinDelta(delta);
      setTriggerCoin(true);
      const timer = setTimeout(() => setTriggerCoin(false), 800);
      prevCostRef.current = savedCost;
      return () => clearTimeout(timer);
    }
    prevCostRef.current = savedCost;
  }, [savedCost]);

  const agentColor = () => {
    switch (currentAgent) {
      case "PM": return "bg-cyan-400 shadow-cyan-500/50";
      case "Designer": return "bg-purple-400 shadow-purple-500/50";
      case "Coder": return "bg-emerald-400 shadow-emerald-500/50";
      case "Auditor": return "bg-amber-400 shadow-amber-500/50";
      case "Verifier": return "bg-indigo-400 shadow-indigo-500/50";
      default: return "bg-zinc-600 shadow-zinc-700/50";
    }
  };

  const agentLabel = () => {
    switch (currentAgent) {
      case "Idle": return t.bubble_idle;
      default: return t.bubble_agent_working.replace("{agent}", currentAgent);
    }
  };

  return (
    <div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className="fixed bottom-12 right-6 z-50 flex items-center font-mono select-none"
    >
      {isHovered && (
        <div className="mr-2 bg-[#121214]/95 border border-[#27272a] rounded px-3 py-1.5 shadow-2xl backdrop-blur-md flex items-center space-x-3 text-[11px] text-zinc-300 animate-slideLeft">
          <div className="flex flex-col space-y-0.5">
            <span className="text-zinc-500 font-bold">{t.bubble_shadow_pilot}</span>
            <span className="text-white font-medium">{agentLabel()}</span>
          </div>
          <div className="h-6 w-[1px] bg-[#27272a]" />
          <div className="text-emerald-400 flex flex-col items-end">
            <span className="text-[9px] text-zinc-500">{t.bubble_saved}</span>
            <span className="font-bold">¥ {savedCost.toFixed(2)}</span>
          </div>
          <button
            onClick={onMaximize}
            className="bg-zinc-100 hover:bg-white text-black text-[10px] font-bold px-2 py-0.5 rounded transition-colors"
          >
            {t.bubble_restore}
          </button>
        </div>
      )}

      <div
        onClick={onMaximize}
        className="w-10 h-10 rounded-full bg-black border-2 border-[#27272a] flex items-center justify-center cursor-pointer hover:border-zinc-500 shadow-2xl transition-all relative group"
      >
        <span className={`absolute inset-0.5 rounded-full ${agentColor()} opacity-20 group-hover:opacity-40 animate-ping`} />
        <span className={`w-2.5 h-2.5 rounded-full ${agentColor()} shadow-md transition-all`} />

        {triggerCoin && (
          <span className="absolute -top-6 text-emerald-400 font-bold text-[10px] bg-emerald-950/80 border border-emerald-800/40 px-1 rounded animate-coinFloat select-none">
            <CoinsIcon size={10} className="stroke-emerald-400" /> Ching! +¥{coinDelta.toFixed(2)}
          </span>
        )}
      </div>
    </div>
  );
}
