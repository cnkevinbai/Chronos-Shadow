import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// ─── 轻量级 Markdown → React JSX 渲染器 ────────────────────────

interface MdBlock {
  type: "text" | "code" | "inlineCode" | "bold" | "list";
  content: string;
  lang?: string;
  items?: string[];
}

/**
 * 将 Markdown 文本解析为结构化块数组
 * 支持：```代码块```、`内联代码`、**粗体**、- 列表
 */
export function parseMarkdown(text: string): MdBlock[] {
  const blocks: MdBlock[] = [];
  let remaining = text;

  while (remaining.length > 0) {
    // 代码块 ```
    const codeBlockMatch = remaining.match(
      /^```(\w*)\n([\s\S]*?)```/,
    );
    if (codeBlockMatch && codeBlockMatch.index === 0) {
      blocks.push({
        type: "code",
        content: codeBlockMatch[2].trimEnd(),
        lang: codeBlockMatch[1] || undefined,
      });
      remaining = remaining.slice(codeBlockMatch[0].length);
      continue;
    }

    // 列表项（以 - 或 * 开头）
    const listMatch = remaining.match(/^(?:[-*] .+(?:\n|$))+/);
    if (listMatch && listMatch.index === 0) {
      const items = listMatch[0]
        .split("\n")
        .filter((l) => l.trim())
        .map((l) => l.replace(/^[-*] /, "").trim());
      blocks.push({ type: "list", content: "", items });
      remaining = remaining.slice(listMatch[0].length);
      continue;
    }

    // 取到下一个特殊标记之前
    const nextSpecial = remaining.search(/```|(?<!\w)\*\*|`|^[-*] /m);
    if (nextSpecial === -1) {
      blocks.push({ type: "text", content: remaining });
      break;
    }
    if (nextSpecial > 0) {
      blocks.push({
        type: "text",
        content: remaining.slice(0, nextSpecial),
      });
      remaining = remaining.slice(nextSpecial);
      continue;
    }

    // 内联代码 `
    if (remaining.startsWith("`")) {
      const end = remaining.indexOf("`", 1);
      if (end !== -1) {
        blocks.push({
          type: "inlineCode",
          content: remaining.slice(1, end),
        });
        remaining = remaining.slice(end + 1);
        continue;
      }
    }

    // 粗体 **
    if (remaining.startsWith("**")) {
      const end = remaining.indexOf("**", 2);
      if (end !== -1) {
        blocks.push({
          type: "bold",
          content: remaining.slice(2, end),
        });
        remaining = remaining.slice(end + 2);
        continue;
      }
    }

    // Fallback: consume one char
    const textEnd = Math.min(
      ...[...remaining.matchAll(/[`*]|^[-*] /gm)].map((m) => m.index!).filter((i) => i > 0),
      remaining.length,
    );
    blocks.push({
      type: "text",
      content: remaining.slice(0, textEnd || 1),
    });
    remaining = remaining.slice(textEnd || 1);
  }

  return blocks;
}

/**
 * 将 Markdown 文本渲染为 React JSX 元素数组
 */
export function renderMarkdown(
  text: string,
): (string | { type: string; props: Record<string, unknown> })[] {
  const blocks = parseMarkdown(text);
  return blocks.map((block, i) => {
    switch (block.type) {
      case "code":
        return {
          type: "pre",
          props: {
            key: i,
            className:
              "bg-black/60 border border-zinc-800 rounded p-2 my-1.5 overflow-x-auto text-[11px] leading-relaxed",
            children: [
              block.lang && {
                type: "div",
                props: {
                  key: "lang",
                  className:
                    "text-[9px] text-zinc-600 mb-1 uppercase tracking-wider",
                  children: block.lang,
                },
              },
              {
                type: "code",
                props: {
                  key: "code",
                  className: "text-zinc-300 font-mono whitespace-pre-wrap",
                  children: block.content,
                },
              },
            ].filter(Boolean),
          },
        };
      case "inlineCode":
        return {
          type: "code",
          props: {
            key: i,
            className:
              "bg-cs-header border border-cs-border rounded px-1 py-0.5 text-[11px] text-emerald-400 font-mono",
            children: block.content,
          },
        };
      case "bold":
        return {
          type: "strong",
          props: {
            key: i,
            className: "font-bold text-zinc-200",
            children: block.content,
          },
        };
      case "list":
        return {
          type: "ul",
          props: {
            key: i,
            className: "list-disc list-inside space-y-0.5 my-1",
            children: block.items!.map((item, j) => ({
              type: "li",
              props: { key: j, children: item },
            })),
          },
        };
      default:
        return block.content;
    }
  });
}
