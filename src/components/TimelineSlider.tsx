// 可视化时空节点拖拽回滚 — Timeline Slider
// 白皮书 UX 增强设计 §4：视频进度条式滑块

import { useState } from "react";
import { useT } from "@/lib/i18n-context";

interface TimelineSliderProps {
  checkpoints: { id: string; label: string }[];
  currentIndex: number;
  onSeek: (index: number) => void;
}

export default function TimelineSlider({ checkpoints, currentIndex, onSeek }: TimelineSliderProps) {
  const t = useT();
  const [dragging, setDragging] = useState(false);
  const [previewIdx, setPreviewIdx] = useState(currentIndex);

  if (checkpoints.length < 2) return null;

  const pct = checkpoints.length > 1
    ? (previewIdx / (checkpoints.length - 1)) * 100
    : 0;

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const idx = Number(e.target.value);
    setPreviewIdx(idx);
  };

  return (
    <div className="px-3 py-2 border-t border-[#27272a] bg-[#0c0c0e] shrink-0">
      <div className="flex items-center justify-between text-[9px] text-zinc-500 mb-1">
        <span>{t.timeline_title}</span>
        <span className="text-emerald-400">
          {dragging ? `${t.timeline_preview}: ${checkpoints[previewIdx]?.label ?? ""}` : checkpoints[currentIndex]?.label ?? ""}
        </span>
      </div>
      <input
        type="range"
        min={0}
        max={checkpoints.length - 1}
        value={dragging ? previewIdx : currentIndex}
        onChange={handleChange}
        onMouseDown={() => setDragging(true)}
        onMouseUp={() => { onSeek(previewIdx); setDragging(false); }}
        onTouchStart={() => setDragging(true)}
        onTouchEnd={() => { onSeek(previewIdx); setDragging(false); }}
        className="w-full h-1.5 rounded-full appearance-none bg-[#27272a] cursor-pointer
          [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:h-3.5
          [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-emerald-400
          [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:cursor-grab
          [&::-webkit-slider-thumb]:transition-shadow hover:[&::-webkit-slider-thumb]:shadow-emerald-500/50"
        style={{
          background: `linear-gradient(to right, #10b981 ${pct}%, #27272a ${pct}%)`,
        }}
      />
      <div className="flex justify-between text-[8px] text-zinc-600 mt-0.5">
        {checkpoints.map((cp, i) => (
          <span key={cp.id} className={i === currentIndex ? "text-emerald-400 font-bold" : ""}>
            {cp.id}
          </span>
        ))}
      </div>
    </div>
  );
}
