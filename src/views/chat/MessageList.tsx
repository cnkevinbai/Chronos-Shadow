// src/views/chat/MessageList.tsx — 消息流渲染（自 ChatPanel 拆分）
import React, { createElement } from "react";
import type { ReactNode } from "react";
import { Copy, Lightbulb, RefreshCw, FileText, Image as LucideImage } from "lucide-react";
import { useT } from "@/lib/i18n-context";
import { renderMarkdown } from "@/lib/utils";
import type { Message } from "@/lib/types";

// ─── Markdown 渲染辅助组件 ──────────────────────────────────────

type MdNode = string | { type: string; props: Record<string, unknown> };

// oxlint-disable-next-line react/only-export-components -- 仅内部递归使用，无需导出
function renderMdNode(node: MdNode): ReactNode {
  if (typeof node === "string") return node;
  if (!node || typeof node !== "object") return null;
  const { type, props } = node;
  const { children, ...rest } = props as Record<string, unknown>;
  const childNodes = Array.isArray(children)
    ? children.map((c, i) => (
        <React.Fragment key={i}>{renderMdNode(c as MdNode)}</React.Fragment>
      ))
    : (children as ReactNode);
  return createElement(type as string, rest, childNodes);
}

function MarkdownContent({ text }: { text: string }) {
  const nodes = renderMarkdown(text);
  return (
    <div className="whitespace-pre-wrap font-medium">
      {nodes.map((node, i) => (
        <React.Fragment key={i}>{renderMdNode(node)}</React.Fragment>
      ))}
    </div>
  );
}

function getSenderStyle(sender: string): string {
  switch (sender) {
    case "PM":
      return "border-cyan-500/30 bg-cyan-950/10 text-cyan-400";
    case "UI Designer":
      return "border-purple-500/30 bg-purple-950/10 text-purple-400";
    case "Coder":
      return "border-emerald-500/30 bg-emerald-950/10 text-emerald-400";
    case "System":
      return "border-zinc-800 bg-zinc-900/40 text-zinc-400 text-xs";
    default:
      return "border-zinc-700 bg-black text-cs-text";
  }
}

interface MessageListProps {
  messages: Message[];
  searchMatches: number[];
  isThinking: boolean;
  retryCount: number;
  onCopyMessage: (content: string) => void;
  onRetry: () => void;
  msgContainerRef: React.RefObject<HTMLDivElement | null>;
  chatEndRef: React.RefObject<HTMLDivElement | null>;
  isNearBottomRef: React.MutableRefObject<boolean>;
  showFileExplorer: boolean;
}

export default function MessageList({
  messages, searchMatches, isThinking, retryCount, onCopyMessage, onRetry,
  msgContainerRef, chatEndRef, isNearBottomRef, showFileExplorer,
}: MessageListProps) {
  const t = useT();
  // 搜索命中索引查找表：避免在 map 内反复 findIndex（O(n²) → O(n)）
  const matchIdxSet = new Set(searchMatches);
  const idxOfId = new Map(messages.map((m, i) => [m.id, i] as const));
  return (
        <div
          ref={msgContainerRef}
          onScroll={() => {
            const el = msgContainerRef.current;
            if (el) {
              isNearBottomRef.current =
                el.scrollHeight - el.scrollTop - el.clientHeight < 80;
            }
          }}
          className={`${showFileExplorer ? 'flex-[3]' : 'flex-1'} p-4 space-y-4 overflow-y-auto scrollbar-thin`}
        >
          {messages.map((msg) => (
            <div
              key={msg.id}
              id={`msg-${msg.id}`}
              className={`flex flex-col space-y-1 max-w-[85%] group ${
                messages.length > 50 ? "chat-msg-virtual" : ""
              } ${
                // 最后一条消息淡入动画
                msg.id === messages[messages.length - 1]?.id &&
                msg.sender !== "System"
                  ? "animate-msg-in"
                  : ""
              } ${
                (idxOfId.has(msg.id) && matchIdxSet.has(idxOfId.get(msg.id)!))
                  ? "ring-1 ring-amber-500/30 rounded-lg"
                  : ""
              } ${
                msg.sender === "User"
                  ? "ml-auto items-end"
                  : "mr-auto items-start"
              }`}
            >
              <div className="flex items-center space-x-1.5 text-[10px] text-zinc-500 px-1">
                <span className="font-bold text-zinc-400">
                  {msg.sender}
                </span>
                <span>•</span>
                <span className="bg-cs-header border border-cs-border px-1 rounded text-[10px] text-zinc-300">
                  {msg.model}
                </span>
                {msg.costTokens != null && msg.costTokens > 0 && (
                  <span className="text-zinc-500">
                    ({msg.costTokens}t
                    {msg.isCached && (
                      <span className="ml-1 text-[10px] text-emerald-400 font-bold border border-emerald-500/30 bg-emerald-950/20 px-1 rounded animate-pulse">
                        [Cache-Aligned]
                      </span>
                    )}
                    <span className="text-emerald-600 ml-0.5">
                      ¥{(msg.costTokens * 0.000001).toFixed(4)}
                    </span>
                    )
                  </span>
                )}
 {/* 显式呈现特征哈希对齐标记，赋予极客绝对的高能效掌控爽感 */}
                {msg.cachingMarkerHash && (
                  <span className="text-emerald-500 font-bold border border-emerald-950 bg-emerald-950/20 px-1 rounded scale-90 select-none">
                    [Cache-Aligned]
                  </span>
                )}
                {msg.cachingMarkerHash && (
                  <span className="hidden group-hover:inline text-[10px] text-zinc-500 font-light">
                    Hash: {msg.cachingMarkerHash.substring(0, 6)}
                  </span>
                )}
                <span className="text-[10px] text-zinc-500">
                  {msg.timestamp}
                </span>
              </div>
              <div
                className={`border p-3 rounded-lg text-xs leading-relaxed tracking-wide shadow-sm max-w-full relative ${getSenderStyle(msg.sender)}`}
              >
                {/* 复制按钮 */}
                <button
                  onClick={() => onCopyMessage(msg.content)}
                  className="absolute top-1 right-1 w-5 h-5 flex items-center justify-center rounded text-[10px] text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50 opacity-0 group-hover:opacity-100 transition-all"
                  title="复制内容"
                >
                  <Copy size={10} aria-hidden="true" />
                </button>
                {/* 多模态附件胶囊标签 */}
                {msg.attachments && msg.attachments.length > 0 && (
                  <div className="mb-2.5 flex flex-wrap gap-1.5 border-b border-zinc-900 pb-2">
                    {msg.attachments.map((att, i) => (
                      <div
                        key={i}
                        className="flex items-center space-x-1.5 bg-black/60 border border-zinc-800/80 px-2 py-1 rounded text-[10px]"
                      >
                        <span>
                          {att.type === "doc" ? <FileText size={10} aria-hidden="true" /> : <LucideImage size={10} aria-hidden="true" />}
                        </span>
                        <span className="text-zinc-300 truncate max-w-[120px] font-medium">
                          {att.name}
                        </span>
                        <span className="text-[10px] text-zinc-500">
                          ({att.sizeOrPath})
                        </span>
                      </div>
                    ))}
                  </div>
                )}
                {msg.thinking && (
                  <details className="mb-2.5 border-l-2 border-zinc-700 pl-2 text-zinc-500 bg-black/30 p-1.5 rounded transition-all group">
                    <summary className="cursor-pointer text-[10px] text-zinc-400 select-none outline-none font-bold hover:text-zinc-300">
                      <Lightbulb size={10} className="inline mr-0.5 -mt-0.5" aria-hidden="true" />
                      {t.view_thinking}
                    </summary>
                    <p className="mt-1.5 text-[11px] font-light leading-normal text-zinc-500 italic whitespace-pre-line animate-fadeIn">
                      {msg.thinking}
                    </p>
                  </details>
                )}
                <MarkdownContent text={msg.content} />
                {msg.sender === "System" && msg.model === "Error" && retryCount < 2 && (
                  <button
                    onClick={onRetry}
                    className="mt-2 flex items-center space-x-1 text-[10px] bg-amber-800/30 hover:bg-amber-700/40 border border-amber-700/40 text-amber-300 px-2 py-0.5 rounded transition-colors"
                  >
                    <RefreshCw size={9} className="inline mr-0.5 -mt-0.5" aria-hidden="true" />
                    重试 ({2 - retryCount} 次)
                  </button>
                )}
              </div>
            </div>
          ))}

          {isThinking && (
            <div className="flex flex-col space-y-1.5 mr-auto items-start animate-pulse">
              <div className="text-[10px] text-zinc-500">
                {t.pipeline_dispatching}
              </div>
              <div className="border border-zinc-800 bg-zinc-900/20 px-4 py-2.5 rounded-lg text-xs text-zinc-500 italic flex items-center space-x-2">
                <div className="w-1.5 h-1.5 rounded-full bg-zinc-500 animate-bounce [animation-delay:-0.3s]" />
                <div className="w-1.5 h-1.5 rounded-full bg-zinc-500 animate-bounce [animation-delay:-0.15s]" />
                <div className="w-1.5 h-1.5 rounded-full bg-zinc-500 animate-bounce" />
                <span>{t.syncing_blackboard}</span>
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>
  );
}
