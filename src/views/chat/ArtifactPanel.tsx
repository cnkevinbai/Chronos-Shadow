// src/views/chat/ArtifactPanel.tsx — 成品文件面板 + 文件编辑模态（自 ChatPanel 拆分）
import { Package, Pencil } from "lucide-react";
import { cvfsReadFile } from "@/lib/tauri";

export interface ChatArtifact {
  path: string; type: string; createdAt: string; versions: number;
}

function extColor(ext: string): string {
  const palette = [
    'text-cyan-400', 'text-emerald-400', 'text-amber-400', 'text-purple-400',
    'text-rose-400', 'text-blue-400', 'text-lime-400', 'text-orange-400',
    'text-teal-400', 'text-pink-400', 'text-indigo-400', 'text-yellow-400',
  ];
  let hash = 0;
  for (let i = 0; i < ext.length; i++) {
    hash = (hash * 31 + ext.charCodeAt(i)) >>> 0;
  }
  return palette[hash % palette.length];
}

interface ArtifactPanelProps {
  artifacts: ChatArtifact[];
  onClear: () => void;
  onEditRequest: (path: string) => void;
  editingFile: string | null;
  setEditingFile: (v: string | null) => void;
  fileContent: string;
  setFileContent: (v: string) => void;
  onEditWithAI: (path: string, content: string) => void;
  currentProject: string;
}

export default function ArtifactPanel({
  artifacts, onClear, onEditRequest, editingFile, setEditingFile,
  fileContent, setFileContent, onEditWithAI, currentProject,
}: ArtifactPanelProps) {
  return (
    <>
          

            {artifacts.length > 0 && (

              <div className="border-t border-cs-border bg-cs-surface px-3 py-1.5 shrink-0 max-h-32 overflow-y-auto">

              <div className="flex items-center justify-between text-[10px] text-zinc-500 mb-1">

                <span className="font-bold text-zinc-400 flex items-center gap-1"><Package size={10} aria-hidden="true" />本会话成品 ({artifacts.length})</span>

                  <button onClick={onClear} className="text-zinc-500 hover:text-zinc-400">清空</button>

                </div>

                <div className="space-y-0.5">

                  {artifacts.map((a, i) => (

                    <div key={i} className="flex items-center justify-between text-[10px] bg-cs-header border border-cs-border rounded px-2 py-1">

                      <span className="text-zinc-300 truncate max-w-[200px] font-mono" title={a.path}>

                        {a.path.split(/[\\/]/).pop()}

                      </span>

                      <span className={`px-1 rounded text-[10px] ${extColor(a.type)}`}>

                        .{a.type}

                      </span>

                      <div className="flex items-center space-x-1">

                        <button

                          onClick={() => onEditRequest(a.path)}

                          className="text-[10px] text-cyan-400 hover:text-cyan-300 px-1 rounded border border-cyan-800/30 hover:border-cyan-500/40"

                          title="编辑文件"

                        ><Pencil size={9} aria-hidden="true" /></button>

                        <span className="text-zinc-500">v{a.versions}</span>

                      </div>

                    </div>

                  ))}

                </div>

              </div>

            )}



            {/* 文件编辑模态框 */}

            {editingFile && (

              <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/70 backdrop-blur-sm" onClick={e => { if(e.target===e.currentTarget) setEditingFile(null); }}>

                <div className="w-[600px] bg-cs-header border border-cs-border rounded-xl shadow-2xl overflow-hidden animate-fadeIn">

                  <div className="flex items-center justify-between px-4 py-2 border-b border-cs-border">

                    <span className="text-[11px] font-bold text-zinc-300 flex items-center gap-1"><Pencil size={11} aria-hidden="true" />编辑: {editingFile.split(/[\\/]/).pop()}</span>

 <button onClick={() => setEditingFile(null)} className="text-zinc-500 hover:text-zinc-300">✕</button>

                  </div>

                  <textarea

                    value={fileContent}

                    onChange={e => setFileContent(e.target.value)}

                    className="w-full h-64 bg-cs-bg text-zinc-200 text-[11px] font-mono p-3 outline-none resize-none"

                    placeholder="点击下方「加载文件」读取当前内容…"

                  />

                  <div className="flex items-center justify-end space-x-2 px-4 py-2 border-t border-cs-border">

                    <button onClick={() => { cvfsReadFile(currentProject, editingFile).then(content => setFileContent(content)).catch(() => {}); }}

className="text-[10px] text-zinc-400 hover:text-zinc-200 px-2 py-1 rounded border border-cs-border"> 加载文件</button>

                    <button onClick={() => setEditingFile(null)}

                      className="text-[10px] text-zinc-500 hover:text-zinc-300 px-2 py-1">取消</button>

                    <button onClick={() => { if (fileContent.trim()) onEditWithAI(editingFile, fileContent) }}

className="text-[10px] bg-cyan-800/50 hover:bg-cyan-700 text-cyan-300 px-2 py-1 rounded font-bold"> 让AI修改</button>

                  </div>

                </div>

              </div>

            )}
    </>
  );
}
