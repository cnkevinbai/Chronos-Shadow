import { useState, useEffect, useRef } from "react";
import { useT } from "@/lib/i18n-context";
import { CoinsIcon, ShieldIcon } from "@/components/SvgIcons";

interface FooterBarProps {
  sessionCost: number;
  savedCost: number;
  savingRate: number;
  routeMode: "auto" | "manual";
  buddySaved?: number;
}

function getFontScale(): number {
  try {
    return parseFloat(localStorage.getItem("cs-font-scale") ?? "1.0");
  } catch {
    return 1.0;
  }
}

function setFontScale(scale: number) {
  document.documentElement.style.fontSize = `${16 * scale}px`;
  localStorage.setItem("cs-font-scale", String(scale));
}

// Initialize on load
if (typeof document !== "undefined") {
  setFontScale(getFontScale());
}

export default function FooterBar({
  sessionCost,
  savedCost,
  savingRate,
  routeMode,
  buddySaved = 0.52,
}: FooterBarProps) {
  const t = useT();
  const [costLimit, setCostLimit] = useState<number>(5.0);
  const [isCapActive, setIsCapActive] = useState<boolean>(true);
  const [coinBounce, setCoinBounce] = useState(false);
  const prevSaved = useRef(savedCost);

  useEffect(() => {
    if (savedCost > prevSaved.current) {
      setCoinBounce(true);
      // 游戏化清脆金币声效
      try {
        const ctx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain); gain.connect(ctx.destination);
        osc.frequency.setValueAtTime(1200, ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(1800, ctx.currentTime + 0.08);
        gain.gain.setValueAtTime(0.08, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.15);
        osc.start(ctx.currentTime); osc.stop(ctx.currentTime + 0.15);
      } catch (_) { /* Audio not available */ }
      const t = setTimeout(() => setCoinBounce(false), 600);
      prevSaved.current = savedCost;
      return () => clearTimeout(t);
    }
    prevSaved.current = savedCost;
  }, [savedCost]);

  return (
    <footer className="h-8 border-t border-cs-border bg-cs-header px-4 flex items-center justify-between text-[11px] tracking-wide select-none shrink-0">
      {/* 左侧：当前会话开销 */}
      <div className="flex items-center space-x-2">
        <span className="text-cs-dim">{t.session_cost}</span>
        <span
          className={`font-bold transition-colors ${
            sessionCost > costLimit * 0.8 ? "text-cs-danger" : "text-cs-accent"
          }`}
        >
          ¥ {sessionCost.toFixed(3)}
        </span>
        <div className="w-[1px] h-3 bg-cs-border" />
        <span className="text-[10px] text-cs-muted font-light">
          {t.mode}{" "}
          {routeMode === "auto" ? t.auto_rule : t.manual_control}
        </span>
      </div>

      {/* 中间：省钱对账单 */}
      <div className="flex items-center space-x-2 border border-cs-accent-border bg-cs-accent-dim/30 px-3 py-0.5 rounded text-cs-accent font-medium">
        <CoinsIcon size={12} className={`stroke-emerald-400 mr-1 ${coinBounce ? "animate-coinFloat" : ""}`} />
        <span>{t.shield_saved}</span>
        <span className="underline decoration-dotted font-bold">
          ¥ {savedCost.toFixed(2)}
        </span>
        <span className="text-[10px] text-cs-accent/80">
          ({t.efficiency} {savingRate}%)
        </span>
        <div className="w-[1px] h-3 bg-cs-border" />
        <span className="text-[10px] text-amber-400/80">
          {t.saved_by_evolution}: ¥ {(buddySaved * 0.3).toFixed(2)}
        </span>
        <div className="w-[1px] h-3 bg-cs-border" />
        <span className="text-[10px] text-purple-400/80">
          <ShieldIcon size={10} className="stroke-purple-400 inline mr-0.5" />探测拦截: ¥ {(buddySaved * 0.42).toFixed(2)}
        </span>
        <div className="w-[1px] h-3 bg-cs-border" />
        <span className="text-[10px] text-cyan-400/80">
          <ShieldIcon size={10} className="stroke-cyan-400 inline mr-0.5" />{t.buddy_saved}: ¥ {buddySaved.toFixed(2)}
        </span>
      </div>

      {/* 右侧：安全通道 + 字体缩放 + 费用熔断器 */}
      <div className="flex items-center space-x-3">
        {/* AES-GCM 安全通道 */}
        <span className="text-[8px] text-amber-400/70 border border-amber-500/20 bg-amber-950/20 px-1.5 py-0.5 rounded flex items-center space-x-0.5">
          <ShieldIcon size={10} className="stroke-amber-400 inline mr-0.5" />AES-GCM
        </span>

        {/* 字体缩放 */}
        <div className="flex items-center space-x-0.5 text-[10px] text-zinc-500">
          <button
            onClick={() => {
              const s = Math.max(0.85, getFontScale() - 0.05);
              setFontScale(s);
            }}
            className="px-1 hover:text-zinc-300 transition-colors"
            title="缩小字体"
          >
            A⁻
          </button>
          <span className="text-zinc-600">
            {Math.round(getFontScale() * 100)}%
          </span>
          <button
            onClick={() => {
              const s = Math.min(1.2, getFontScale() + 0.05);
              setFontScale(s);
            }}
            className="px-1 hover:text-zinc-300 transition-colors"
            title="放大字体"
          >
            A⁺
          </button>
        </div>
        <div className="w-[1px] h-3 bg-cs-border" />
        <div className="flex items-center space-x-1.5">
          <span className="text-cs-muted">{t.cost_cap}</span>
          <span
            className="text-cs-text bg-black border border-cs-border px-1 py-0.5 rounded font-bold cursor-pointer hover:border-cs-dim transition-colors"
            onClick={() => {
              const newLimit = prompt(
                t.cost_cap_prompt,
                costLimit.toString(),
              );
              if (newLimit && !isNaN(Number(newLimit)))
                setCostLimit(Number(newLimit));
            }}
          >
            ¥ {costLimit.toFixed(2)}
          </span>
        </div>

        {/* 熔断开关 */}
        <div className="flex items-center space-x-1">
          <span className="text-[10px] text-cs-muted">{t.healing_fuse}</span>
          <button
            onClick={() => setIsCapActive(!isCapActive)}
            className={`w-7 h-4 rounded-full p-0.5 transition-colors relative outline-none ${
              isCapActive ? "bg-cs-accent" : "bg-cs-border"
            }`}
          >
            <div
              className={`w-3 h-3 rounded-full bg-white transition-transform ${
                isCapActive ? "translate-x-3" : "translate-x-0"
              }`}
            />
          </button>
        </div>
      </div>
    </footer>
  );
}
