// Visual Diff 对比面板
// 红绿差异对比 — 连接沙盒快照系统，展示文件变更
import { useMemo } from "react";
import { useT } from "@/lib/i18n-context";
import { Diff, GitBranch, ArrowRight } from "lucide-react";

interface DiffLine {
  type: "add" | "remove" | "context";
  content: string;
  lineNum?: number;
}

interface DiffViewerProps {
  leftLabel?: string;
  rightLabel?: string;
  leftContent?: string;
  rightContent?: string;
  title?: string;
}

/** 计算两个文本之间的差异 */
function computeDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");
  const result: DiffLine[] = [];

  // Simple line-by-line diff (LCS would be better, but simplified for UI)
  let i = 0;
  let j = 0;

  while (i < oldLines.length || j < newLines.length) {
    if (i < oldLines.length && j < newLines.length && oldLines[i] === newLines[j]) {
      result.push({ type: "context", content: oldLines[i], lineNum: j + 1 });
      i++;
      j++;
    } else {
      // Find next matching line
      let found = false;
      for (let k = 0; k < 10 && i + k < oldLines.length; k++) {
        if (oldLines[i + k] === newLines[j]) {
          for (let x = 0; x < k; x++) {
            result.push({ type: "remove", content: oldLines[i + x], lineNum: i + x + 1 });
          }
          i += k;
          found = true;
          break;
        }
      }
      if (!found && j < newLines.length) {
        // Emit removal first, then addition — don't silently drop old lines
        if (i < oldLines.length) {
          result.push({ type: "remove", content: oldLines[i], lineNum: i + 1 });
          i++;
        }
        result.push({ type: "add", content: newLines[j], lineNum: j + 1 });
        j++;
      } else if (i < oldLines.length) {
        result.push({ type: "remove", content: oldLines[i], lineNum: i + 1 });
        i++;
      }
    }
  }

  return result;
}

export default function DiffViewer({
  leftLabel,
  rightLabel,
  leftContent = "",
  rightContent = "",
  title,
}: DiffViewerProps) {
  const t = useT();
  const diffLines = useMemo(
    () => computeDiff(leftContent, rightContent),
    [leftContent, rightContent],
  );

  const addedCount = diffLines.filter((l) => l.type === "add").length;
  const removedCount = diffLines.filter((l) => l.type === "remove").length;

  if (!leftContent && !rightContent) {
    return (
      <div className="flex items-center justify-center h-full text-cs-muted text-[10px]">
        {t.diff_select_hint}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-cs-bg">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-cs-border shrink-0">
        <div className="flex items-center space-x-2">
          <Diff className="w-3 h-3 text-cs-info" />
          <span className="text-[10px] font-bold text-cs-text">
            {title ?? t.diff_title}
          </span>
        </div>
        <div className="flex items-center space-x-2 text-[9px]">
          <span className="text-cs-accent">+{addedCount}</span>
          <span className="text-cs-danger">-{removedCount}</span>
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center space-x-4 px-3 py-1.5 border-b border-cs-border/50 text-[9px] shrink-0">
        <div className="flex items-center space-x-1">
          <div className="w-3 h-3 rounded bg-cs-accent/20 border border-cs-accent/30" />
          <span className="text-cs-dim">{leftLabel ?? t.before}</span>
        </div>
        <ArrowRight className="w-2.5 h-2.5 text-cs-muted" />
        <div className="flex items-center space-x-1">
          <div className="w-3 h-3 rounded bg-cs-danger/20 border border-cs-danger/30" />
          <span className="text-cs-dim">{rightLabel ?? t.after}</span>
        </div>
      </div>

      {/* Diff content */}
      <div className="flex-1 overflow-auto font-mono text-[10px] leading-relaxed">
        {diffLines.map((line, idx) => (
          <div
            key={idx}
            className={`flex ${
              line.type === "add"
                ? "bg-cs-accent/10 border-l-2 border-cs-accent"
                : line.type === "remove"
                  ? "bg-cs-danger/10 border-l-2 border-cs-danger"
                  : "border-l-2 border-transparent"
            }`}
          >
            {/* Line number */}
            <span className="w-8 text-right pr-2 text-cs-muted/50 shrink-0 select-none">
              {line.lineNum ?? ""}
            </span>
            {/* Prefix */}
            <span
              className={`w-4 text-center shrink-0 select-none ${
                line.type === "add"
                  ? "text-cs-accent"
                  : line.type === "remove"
                    ? "text-cs-danger"
                    : "text-cs-muted/30"
              }`}
            >
              {line.type === "add" ? "+" : line.type === "remove" ? "-" : " "}
            </span>
            {/* Content */}
            <span
              className={`flex-1 truncate ${
                line.type === "add"
                  ? "text-cs-accent/80"
                  : line.type === "remove"
                    ? "text-cs-danger/80"
                    : "text-cs-dim"
              }`}
            >
              {line.content || "\u00A0"}
            </span>
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="h-5 border-t border-cs-border bg-cs-surface px-3 flex items-center text-[9px] text-cs-muted shrink-0">
        <GitBranch className="w-2.5 h-2.5 mr-1" />
        <span>{diffLines.length} {t.lines_count}</span>
        <span className="ml-auto">
          <span className="text-cs-accent">{addedCount} {t.added}</span>
          {" · "}
          <span className="text-cs-danger">{removedCount} {t.removed}</span>
        </span>
      </div>
    </div>
  );
}

/** Compact inline diff for small views */
export function MiniDiff({ before, after }: { before: string; after: string }) {
  if (before === after) return <span className="text-cs-dim">{before}</span>;

  const diff = computeDiff(before, after);
  return (
    <span className="font-mono text-[9px]">
      {diff.map((line, i) => (
        <span
          key={i}
          className={
            line.type === "add"
              ? "text-cs-accent"
              : line.type === "remove"
                ? "text-cs-danger line-through"
                : "text-cs-dim"
          }
        >
          {line.content}
        </span>
      ))}
    </span>
  );
}
