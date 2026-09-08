// src/views/chat/Composer.tsx — 输入区（自 ChatPanel 拆分）
// 职责：斜杠宏/@特种兵 弹窗、多模态挂载看板、输入框、发送/停止按钮
// 弹窗开关状态由父组件持有（键盘 Escape effect 需要 setters），
// slashCommands/subAgents 数据与 selectCommand 闭环逻辑留在父组件。
import QuickMacros from "@/components/QuickMacros";
import { FileTextIcon, ImageIcon } from "@/components/SvgIcons";
import { useT } from "@/lib/i18n-context";
import { FileText, Lightbulb } from "lucide-react";
import type { Attachment } from "@/lib/types";

interface SlashCommand {
  cmd: string;
  desc: string;
}

interface SubAgent {
  name: string;
  desc: string;
  color: string;
}

interface ComposerProps {
  input: string;
  setInput: (v: string) => void;
  onInputChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  isThinking: boolean;
  stagedAttachments: Attachment[];
  setStagedAttachments: React.Dispatch<React.SetStateAction<Attachment[]>>;
  showSlashMenu: boolean;
  showAtMenu: boolean;
  macrosVisible: boolean;
  setMacrosVisible: (v: boolean) => void;
  slashCommands: SlashCommand[];
  subAgents: SubAgent[];
  selectCommand: (cmd: string, type: "slash" | "at") => void;
  onSend: (e: React.FormEvent) => void;
  onCancelStream: () => void;
  onAttachDoc: () => void;
  onAttachImage: () => void;
  apiKey: string;
  inputRef: React.RefObject<HTMLInputElement | null>;
}

export default function Composer({
  input, setInput, onInputChange, isThinking, stagedAttachments, setStagedAttachments,
  showSlashMenu, showAtMenu, macrosVisible, setMacrosVisible,
  slashCommands, subAgents, selectCommand, onSend, onCancelStream,
  onAttachDoc, onAttachImage, apiKey, inputRef,
}: ComposerProps) {
  const t = useT();

  return (
        <div className="relative shrink-0">
          <QuickMacros
            visible={macrosVisible}
            onSelect={(prompt) => {
              setInput(prompt);
              setMacrosVisible(false);
            }}
          />

          {/* 弹窗 A：快捷斜杠宏命令菜单 */}
          {showSlashMenu && (
            <div className="absolute bottom-full left-0 right-0 mb-2 mx-4 bg-cs-header/95 border border-cs-border rounded shadow-2xl z-30 backdrop-blur-md max-h-44 overflow-y-auto animate-slideLeft">
              <div className="px-3 py-1.5 text-[10px] text-zinc-500 font-bold uppercase tracking-wider border-b border-zinc-900">
                快捷斜杠宏命令 (Slash Macros)
              </div>
              {slashCommands.map((sc) => (
                <div
                  key={sc.cmd}
                  onClick={() => selectCommand(sc.cmd, "slash")}
                  className="flex items-center justify-between px-3 py-2 hover:bg-zinc-900/60 cursor-pointer text-xs text-zinc-300 transition-colors"
                >
                  <span className="font-bold text-white">
                    {sc.cmd}
                  </span>
                  <span className="text-[10px] text-zinc-500 font-light ml-2 truncate">
                    {sc.desc}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* 弹窗 B：@ 特种兵子智能体精准靶向选择菜单 */}
          {showAtMenu && (
            <div className="absolute bottom-full left-0 right-0 mb-2 mx-4 bg-cs-header/95 border border-cs-border rounded shadow-2xl z-30 backdrop-blur-md max-h-44 overflow-y-auto animate-slideLeft">
              <div className="px-3 py-1.5 text-[10px] text-zinc-500 font-bold uppercase tracking-wider border-b border-zinc-900">
                唤醒专业特种子智能体 (Target Subagent)
              </div>
              {subAgents.map((sa) => (
                <div
                  key={sa.name}
                  onClick={() => selectCommand(sa.name, "at")}
                  className="flex items-center justify-between px-3 py-2 hover:bg-zinc-900/60 cursor-pointer text-xs text-zinc-300 transition-colors"
                >
                  <span className={`font-bold ${sa.color}`}>
                    {sa.name}
                  </span>
                  <span className="text-[10px] text-zinc-500 font-light ml-2 truncate">
                    {sa.desc}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* 多模态临时挂载缓冲区看板 */}
          {stagedAttachments.length > 0 && (
            <div className="flex flex-wrap gap-1.5 p-2 mx-4 mb-1 bg-black/40 border border-zinc-900 rounded animate-fadeIn">
              {stagedAttachments.map((stg, i) => (
                <div
                  key={i}
                  className="flex items-center space-x-1 bg-zinc-900 border border-zinc-800 px-2 py-0.5 rounded text-[10px] text-zinc-400"
                >
                  <span>{stg.type === "doc" ? <FileText size={10} aria-hidden="true" /> : <ImageIcon size={10} aria-hidden="true" />}</span>
                  <span className="truncate max-w-[100px]">
                    {stg.name}
                  </span>
                  <button
                    type="button"
                    onClick={() =>
                      setStagedAttachments((prev) =>
                        prev.filter((_, idx) => idx !== i),
                      )
                    }
                    className="text-zinc-500 hover:text-zinc-400 ml-1 font-bold"
                  >
✕
                  </button>
                </div>
              ))}
            </div>
          )}

          <form
            data-chat-form
            onSubmit={onSend}
            className="p-3 border-t border-cs-border bg-cs-surface flex items-center space-x-2"
          >
            {/* 附件挂载按钮（真实文件对话框 + 浏览器降级 mock） */}
            <button
              type="button"
              onClick={onAttachDoc}
              title="挂载本地文档/知识库"
              className="w-7 h-7 flex items-center justify-center rounded bg-black border border-cs-border hover:border-zinc-500 text-xs transition-colors shrink-0"
            >
              <FileTextIcon size={14} className="stroke-zinc-400" />
            </button>
            <button
              type="button"
              onClick={onAttachImage}
              title="挂载多模态图片走查"
              className="w-7 h-7 flex items-center justify-center rounded bg-black border border-cs-border hover:border-zinc-500 text-xs transition-colors shrink-0"
            >
              <ImageIcon size={14} className="stroke-zinc-400" />
            </button>

            <div className="flex-1 relative flex items-center bg-black border border-cs-border rounded-lg px-3 py-2.5 focus-within:border-zinc-500 transition-colors">
              <button
                type="button"
                onClick={() => setMacrosVisible(!macrosVisible)}
                className="text-zinc-500 hover:text-emerald-400 text-sm mr-2 transition-colors"
                title="一键宏指令"
              >
                <Lightbulb size={12} aria-hidden="true" />
              </button>
              <span className="text-zinc-500 text-sm font-bold mr-2 select-none">
                $
              </span>
              <input
                ref={inputRef}
                type="text"
                value={input}
                onChange={onInputChange}
                placeholder={
                  apiKey
                    ? "键入 / 触发宏命令，键入 @ 唤醒特种兵…"
: " 请先在全局配置中填入 API Key…"
                }
                className="w-full bg-transparent text-sm text-cs-text placeholder-zinc-600 outline-none border-none p-0"
                disabled={isThinking}
              />
            </div>
            {isThinking ? (
              <button
                type="button"
                onClick={onCancelStream}
                aria-label={t.stream_stop}
                className="bg-red-500/90 hover:bg-red-500 active:scale-95 text-white font-bold text-sm px-4 py-2.5 rounded-lg transition-all duration-150 flex items-center space-x-1 outline-none shadow-sm shrink-0 animate-pulse"
              >
                <span className="inline-block w-2.5 h-2.5 bg-white rounded-[2px]" aria-hidden="true" />
                <span>{t.stream_stop}</span>
              </button>
            ) : (
              <button
                type="submit"
                className="bg-zinc-100 hover:bg-zinc-200 active:bg-zinc-300 active:scale-95 text-black font-bold text-sm px-4 py-2.5 rounded-lg transition-all duration-150 flex items-center space-x-1 outline-none shadow-sm shrink-0"
              >
                <span>{t.execute}</span>
                <span className="text-[10px] bg-zinc-300 px-1 rounded text-zinc-500 ml-0.5">
                  ↵
                </span>
              </button>
            )}
          </form>
        </div>
  );
}
