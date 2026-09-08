// src/views/chat/SessionSidebar.tsx — 会话历史侧栏（自 ChatPanel 拆分）
// 职责：会话清单（按项目分组）/ 搜索过滤 / 内联重命名 / 删除 / 统计摘要 / 清空
// 导入 JSON 会话的入口按钮也在此（dialogOpen 依赖留在父组件，经 onImport 回调）
import { useState } from "react";
import { MessageSquare, BarChart3, Coins, Trash2 } from "lucide-react";
import { useToast } from "@/lib/use-toast";
import { deleteChatSession, renameChatSession } from "@/lib/tauri";
import { appConfirm } from "@/lib/dialogs";
import type { SessionMetaManifest } from "@/lib/types";

interface SessionSidebarProps {
  manifests: SessionMetaManifest[];
  activeSessionId: string;
  currentProject: string;
  onSwitchSession: (id: string) => void;
  onNewSession: () => void;
  onClearAll: () => void;
  onImport: () => void;
  refreshManifests: () => void;
}

export default function SessionSidebar({
  manifests, activeSessionId, currentProject,
  onSwitchSession, onNewSession, onClearAll, onImport, refreshManifests,
}: SessionSidebarProps) {
  const toast = useToast();
  const [sessionFilter, setSessionFilter] = useState("");
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");

  const filteredManifests = sessionFilter.trim()
    ? manifests.filter(
        (m) =>
          m.title.toLowerCase().includes(sessionFilter.toLowerCase()) ||
          (m.last_message_preview ?? "")
            .toLowerCase()
            .includes(sessionFilter.toLowerCase()),
      )
    : manifests;

  const handleStartRename = (sessionId: string, currentTitle: string) => {
    setEditingSessionId(sessionId);
    setEditTitle(currentTitle);
  };

  const handleCommitRename = async (sessionId: string) => {
    if (editTitle.trim()) {
      try {
        await renameChatSession(sessionId, editTitle.trim());
        refreshManifests();
        toast.showToast("success", "RENAMED", "会话已重命名。");
      } catch (err) {
        toast.showToast("error", "RENAME FAILED", `重命名失败: ${err}`);
      }
    }
    setEditingSessionId(null);
    setEditTitle("");
  };

  return (
      <div className="w-56 border-r border-cs-border bg-cs-surface flex flex-col shrink-0">
        <div className="p-2.5 border-b border-cs-border bg-cs-header flex items-center justify-between">
          <span className="font-bold text-zinc-500 uppercase tracking-wider text-[10px]">
            🗂️ 项目会话矩阵
            {manifests.length > 0 && (
              <span className="ml-1.5 bg-zinc-800 text-zinc-400 text-[10px] px-1.5 py-0.5 rounded-full">
                {manifests.length}
              </span>
            )}
          </span>
          <div className="flex items-center space-x-1">
            <button
              onClick={onImport}
              className="text-[10px] bg-black border border-cs-border px-1.5 py-0.5 rounded hover:border-zinc-500 text-zinc-400 hover:text-white transition-colors"
              title="导入 JSON 会话"
            >
              📥
            </button>
            <button
              onClick={onNewSession}
              className="text-[10px] bg-black border border-cs-border px-1.5 py-0.5 rounded hover:border-zinc-500 text-white font-bold transition-colors"
            >
              + NEW
            </button>
          </div>
        </div>

        {/* 会话搜索过滤 */}
        {manifests.length > 0 && (
          <div className="px-2 py-1.5 border-b border-cs-border">
            <input
              value={sessionFilter}
              onChange={(e) => setSessionFilter(e.target.value)}
              placeholder="搜索会话…"
              className="w-full bg-black border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-300 placeholder-zinc-600 outline-none focus:border-zinc-500 transition-colors"
            />
          </div>
        )}

        {/* 清单列表流（仅渲染轻量 .meta，毫秒级撑起成百上千条树轴） */}
        <div className="flex-1 overflow-y-auto p-1.5 space-y-2 scrollbar-thin">
          {/* 项目分组显示 */}
          {(() => {
            const groups = new Map<string, typeof filteredManifests>();
            for (const m of filteredManifests) {
              const proj = (m as any).bound_project || currentProject || "default";
              if (!groups.has(proj)) groups.set(proj, []);
              groups.get(proj)!.push(m);
            }
            return Array.from(groups.entries()).map(([project, sessions]) => (
              <div key={project} className="space-y-0.5">
                <div className="px-1.5 py-0.5 text-[10px] bg-black border border-zinc-900 rounded font-bold text-zinc-400 flex items-center justify-between">
                  <span className="truncate">📁 {project}</span>
                  <span className="text-zinc-500 font-light text-[10px] shrink-0 ml-1">
                    {sessions.length}会话
                  </span>
                </div>
                <div className="pl-1 space-y-0.5">
                  {sessions.map((m) => (
            <div
              key={m.session_id}
              className={`group/session p-2 rounded border text-left transition-all cursor-pointer relative ${
                activeSessionId === m.session_id
                  ? "bg-[#27272a]/70 border-zinc-700 text-white"
                  : "border-transparent text-zinc-400 hover:bg-zinc-900/40"
              }`}
            >
              <div onClick={() => onSwitchSession(m.session_id)}>
                {editingSessionId === m.session_id ? (
                  <input
                    value={editTitle}
                    onChange={(e) => setEditTitle(e.target.value)}
                    onBlur={() => handleCommitRename(m.session_id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter")
                        handleCommitRename(m.session_id);
                      if (e.key === "Escape") {
                        setEditingSessionId(null);
                        setEditTitle("");
                      }
                    }}
                    className="font-bold text-[11px] text-zinc-200 bg-[#1a1a1e] border border-zinc-600 rounded px-1 py-0.5 w-full outline-none"
                    autoFocus
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <div
                    className="font-bold truncate text-[11px] text-zinc-200 pr-5"
                    onDoubleClick={() =>
                      handleStartRename(m.session_id, m.title)
                    }
                    title="双击重命名"
                  >
                    {m.title}
                  </div>
                )}
                <div className="text-[10px] text-zinc-500 mt-1 flex items-center justify-between font-light">
                  <span>🗂️ {m.bound_project}</span>
                  <span className="text-emerald-500 font-medium">
                    ¥{m.total_accumulated_cost.toFixed(3)}
                  </span>
                </div>
                {m.last_message_preview && (
                  <div className="text-[10px] text-zinc-500 mt-1 truncate font-light italic">
                    {m.last_message_preview}
                  </div>
                )}
                <div className="text-[10px] text-zinc-500 mt-0.5 font-light text-right">
                  条数: {m.total_messages_count} |{" "}
                  {m.last_updated.substring(11, 19)}
                </div>
              </div>
              {/* 删除按钮 */}
              <button
                onClick={async (e) => {
                  e.stopPropagation();
                  if (
                    await appConfirm(`确定删除会话「${m.title}」？\n此操作不可撤销。`)
                  ) {
                    try {
                      await deleteChatSession(m.session_id);
                      toast.showToast(
                        "success",
                        "SESSION DELETED",
                        `会话「${m.title}」已从物理磁盘移除。`,
                      );
                      refreshManifests();
                      if (activeSessionId === m.session_id) {
                        onNewSession();
                      }
                    } catch (err) {
                      toast.showToast(
                        "error",
                        "DELETE FAILED",
                        `删除失败: ${err}`,
                      );
                    }
                  }
                }}
                className="absolute top-1.5 right-1.5 w-4 h-4 flex items-center justify-center rounded text-[10px] text-zinc-500 hover:text-red-400 hover:bg-red-950/30 opacity-0 group-hover/session:opacity-100 transition-all"
                title="删除会话"
              >
                ✕
              </button>
            </div>
          ))}
                </div>
              </div>
            ));
          })()}
          {filteredManifests.length === 0 && manifests.length > 0 && (
            <div className="p-3 text-[10px] text-zinc-500 italic text-center">
              无匹配会话
            </div>
          )}
          {manifests.length === 0 && (
            <div className="p-3 text-[10px] text-zinc-500 italic text-center">
              尚无历史会话轨道 —
              <br />
              发送第一条消息后自动建档
            </div>
          )}
        </div>

        {/* 侧栏统计摘要 */}
        {manifests.length > 0 && (
          <div className="p-2 border-t border-cs-border text-[10px] text-zinc-500 space-y-0.5 shrink-0">
            <div className="flex justify-between">
              <span className="flex items-center gap-1"><MessageSquare size={9} aria-hidden="true" />会话</span>
              <span className="text-zinc-500">{manifests.length}</span>
            </div>
            <div className="flex justify-between">
              <span className="flex items-center gap-1"><BarChart3 size={9} aria-hidden="true" />总消息</span>
              <span className="text-zinc-500">
                {manifests.reduce((a, m) => a + m.total_messages_count, 0)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="flex items-center gap-1"><Coins size={9} aria-hidden="true" />累计节省</span>
              <span className="text-emerald-500 font-medium">
                ¥
                {manifests
                  .reduce((a, m) => a + m.total_accumulated_cost, 0)
                  .toFixed(3)}
              </span>
            </div>
            <button
              onClick={onClearAll}
              className="w-full mt-1 flex items-center justify-center gap-1 text-[10px] text-zinc-500 hover:text-red-400 transition-colors text-center"
            >
              <Trash2 size={9} aria-hidden="true" />
              清空全部会话
            </button>
          </div>
        )}
      </div>
  );
}
