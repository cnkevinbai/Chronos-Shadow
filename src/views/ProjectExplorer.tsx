// 智能沙盒与多维时空管理器 — v3 升级版
// 真实文件树 + 检查点捕获/恢复/删除 + 项目健康 + Worktree 状态

import { useState, useEffect, useCallback } from "react";
import { useT } from "@/lib/i18n-context";
import {
  cvfsGetCheckpoints,
  cvfsGetProjects,
  cvfsCreateProject,
  cvfsCaptureCheckpointV2,
  cvfsRestoreCheckpoint,
  cvfsDeleteCheckpoint,
  cvfsDeleteProject,
  cvfsListProjectFiles,
  cvfsGetProjectHealth,
  getSandboxStatus,
  listWorktrees,
  getWorktreeStats,
  mergeWorktree,
  submitForApproval,
} from "@/lib/tauri";
import type { SystemSnapshot, WorktreeInstance, WorktreeStats } from "@/lib/types";
import { Plus, Shield, Clock, HardDrive, Camera, RotateCcw, Trash2, GitMerge, Activity, FolderTree } from "lucide-react";

interface VfsNode { name: string; is_dir: boolean; relative_path: string; is_locked: boolean; }
interface CheckpointEntry { id: string; title: string; desc: string; time: string; }
interface ProjectEntry { id: string; name: string; path: string; }
interface ProjectHealth { project_id: string; path: string; file_count: number; total_size_bytes: number; has_git: boolean; checkpoint_count: number; last_checkpoint: string | null; status: string; }

function mapSnapshot(s: SystemSnapshot): CheckpointEntry {
  return {
    id: s.id,
    title: s.label,
    desc: `${s.files_changed} files · ${s.snapshot_type}`,
    time: s.timestamp.split("T")[1]?.slice(0, 8) ?? s.timestamp.slice(0, 8),
  };
}

function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

interface ProjectExplorerProps {
  currentProject: string;
  onProjectChange: (name: string) => void;
}

export default function ProjectExplorer({ currentProject, onProjectChange }: ProjectExplorerProps) {
  const t = useT();
  const [checkpoints, setCheckpoints] = useState<CheckpointEntry[]>([]);
  const [sandboxStatus, setSandboxStatus] = useState("Active");
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [vfsTree, setVfsTree] = useState<VfsNode[]>([]);
  const [health, setHealth] = useState<ProjectHealth | null>(null);
  const [worktrees, setWorktrees] = useState<WorktreeInstance[]>([]);
  const [wtStats, setWtStats] = useState<WorktreeStats>({ total: 0, active: 0, completed: 0, merged: 0, errors: 0 });
  const [showNewProject, setShowNewProject] = useState(false);
  const [newProjId, setNewProjId] = useState("");
  const [newProjPath, setNewProjPath] = useState("C:\\Chronos-Workspace");
  const [snapshotLabel, setSnapshotLabel] = useState("");
  const [showSnapshot, setShowSnapshot] = useState(false);
  const [activeTab, setActiveTab] = useState<"files" | "checkpoints" | "health">("files");

  const refresh = useCallback(async () => {
    try {
      const [cps, projs, status, files, h, wts, ws] = await Promise.all([
        cvfsGetCheckpoints(),
        cvfsGetProjects(),
        getSandboxStatus(),
        currentProject ? cvfsListProjectFiles(currentProject) : Promise.resolve([]),
        currentProject ? cvfsGetProjectHealth(currentProject) : Promise.resolve(null),
        listWorktrees(),
        getWorktreeStats(),
      ]);
      if (cps.length > 0) setCheckpoints(cps.map(mapSnapshot));
      if (projs.length > 0) setProjects(projs);
      setSandboxStatus(status);
      if (files.length > 0) setVfsTree(files);
      if (h) setHealth(h);
      setWorktrees(wts);
      setWtStats(ws);
    } catch { /* offline */ }
  }, [currentProject]);

  useEffect(() => { refresh(); const iv = setInterval(refresh, 8000); return () => clearInterval(iv); }, [refresh]);

  const handleCreateProject = async () => {
    if (!newProjId) return;
    try {
      await cvfsCreateProject(newProjId, newProjPath);
      setShowNewProject(false); setNewProjId("");
      onProjectChange(newProjId);
      refresh();
    } catch (e) { alert(`创建失败: ${e}`); }
  };

  const handleCaptureCheckpoint = async () => {
    if (!snapshotLabel || !currentProject) return;
    try {
      await cvfsCaptureCheckpointV2(currentProject, snapshotLabel, "手动快照");
      setSnapshotLabel(""); setShowSnapshot(false);
      refresh();
    } catch (e) { alert(`快照失败: ${e}`); }
  };

  const handleRestore = async (cpId: string) => {
    if (!currentProject || !confirm(`确定恢复到检查点 ${cpId}？此操作不可逆。`)) return;
    try { await cvfsRestoreCheckpoint(currentProject, cpId); refresh(); }
    catch (e) { alert(`恢复失败: ${e}`); }
  };

  const handleDeleteCp = async (cpId: string) => {
    if (!currentProject || !confirm(`删除检查点 ${cpId}？`)) return;
    try { await cvfsDeleteCheckpoint(currentProject, cpId); refresh(); }
    catch (e) { alert(`删除失败: ${e}`); }
  };

  const handleDeleteProject = async () => {
    if (!currentProject || currentProject === "default" || !confirm(`确认删除项目 ${currentProject}？此操作不可恢复。`)) return;
    try { await cvfsDeleteProject(currentProject); onProjectChange("default"); refresh(); }
    catch (e) { alert(`删除失败: ${e}`); }
  };

  const handleMergeWorktree = async (wtId: string) => {
    if (!confirm(`确认合并 Worktree ${wtId} 到主分支？`)) return;
    try {
      await mergeWorktree(wtId);
      alert(`Worktree ${wtId} 合并成功`);
      refresh();
    } catch (e: unknown) {
      const msg = String(e);
      if (msg.includes("第四红线")) {
        // 从错误消息提取 target_id 并自动提交审批
        const targetMatch = msg.match(/target=([^)]+)/);
        const targetId = targetMatch ? targetMatch[1] : wtId;
        try {
          await submitForApproval("worktree_merge", targetId,
            `合并 Worktree ${targetId} 到主分支`, "{}");
          alert(`已自动提交审批请求 (${targetId})。请切换到审批面板 (🛡️ 第四红线) 审核后重试合并。`);
        } catch { alert(`审批提交失败: ${msg}`); }
      } else {
        alert(`合并失败: ${msg}`);
      }
    }
  };

  const displayProjects = projects.length > 0 ? projects : [];

  return (
    <div className="h-full flex flex-col bg-[#0c0c0e] font-mono text-xs text-zinc-400 select-none overflow-hidden">
      {/* 1. 项目主切换枢纽 */}
      <div className="p-3 border-b border-[#27272a] bg-[#121214] flex flex-col space-y-2 shrink-0">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-bold text-zinc-500 uppercase tracking-wider">📁 {t.workspace}</span>
          <div className="flex items-center space-x-1">
            <button onClick={() => setShowSnapshot(!showSnapshot)}
              className="text-[9px] bg-black border border-amber-500/30 hover:border-amber-400 text-amber-400 px-1.5 py-0.5 rounded transition-all" title="手动快照">
              <Camera className="w-2.5 h-2.5 inline mr-0.5" />快照</button>
            <button onClick={() => setShowNewProject(!showNewProject)}
              className="text-[10px] bg-black border border-[#27272a] hover:border-zinc-500 text-white px-1.5 py-0.5 rounded font-bold transition-all active:scale-95">
              <Plus className="w-2.5 h-2.5 inline mr-0.5" />{t.new_button}</button>
          </div>
        </div>

        {showSnapshot && (
          <div className="flex space-x-1 animate-fadeIn">
            <input value={snapshotLabel} onChange={(e) => setSnapshotLabel(e.target.value)}
              placeholder="快照标签" className="flex-1 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-amber-500" />
            <button onClick={handleCaptureCheckpoint}
              className="bg-amber-800/50 hover:bg-amber-700 border border-amber-700/50 text-amber-300 text-[9px] px-2 py-1 rounded">捕获</button>
          </div>
        )}

        {showNewProject && (
          <div className="space-y-1 animate-fadeIn">
            <input value={newProjId} onChange={(e) => setNewProjId(e.target.value)}
              placeholder="项目 ID" className="w-full bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500" />
            <input value={newProjPath} onChange={(e) => setNewProjPath(e.target.value)}
              placeholder="物理路径" className="w-full bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500" />
            <button onClick={handleCreateProject}
              className="w-full bg-cyan-800/50 hover:bg-cyan-700 text-cyan-300 text-[9px] py-1 rounded font-bold">创建项目并锁定 Scope</button>
          </div>
        )}

        <select value={displayProjects.find(p => p.name === currentProject)?.id || ""}
          onChange={e => { const s = displayProjects.find(p => p.id === e.target.value); if (s) onProjectChange(s.name); }}
          className="bg-black border border-[#27272a] rounded px-2 py-1 text-xs text-white outline-none cursor-pointer w-full">
          {displayProjects.length > 0 ? displayProjects.map(p => (
            <option key={p.id} value={p.id}>{p.name} [{p.path.slice(0, 30)}]</option>
          )) : <option value="">暂无项目 — 点击 + 新建</option>}
        </select>

        <div className="flex items-center justify-between text-[9px] text-zinc-500">
          <div className="flex items-center space-x-2">
            <Shield className="w-2.5 h-2.5 text-cs-accent" />
            <span>{currentProject}</span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="text-cs-accent">{sandboxStatus}</span>
            {currentProject !== "default" && (
              <button onClick={handleDeleteProject} className="text-red-600 hover:text-red-400" title="删除项目">
                <Trash2 className="w-2.5 h-2.5" /></button>
            )}
          </div>
        </div>
      </div>

      {/* 2. Tab 切换：文件 / 检查点 / 健康 */}
      <div className="flex border-b border-[#27272a] bg-[#0c0c0e] shrink-0">
        {([
          { id: "files" as const, icon: FolderTree, label: "文件" },
          { id: "checkpoints" as const, icon: Clock, label: `检查点(${checkpoints.length})` },
          { id: "health" as const, icon: Activity, label: "健康" },
        ]).map(tab => {
          const Icon = tab.icon;
          const active = activeTab === tab.id;
          return (
            <button key={tab.id} onClick={() => setActiveTab(tab.id)}
              className={`flex-1 flex items-center justify-center space-x-1 py-1 text-[10px] transition-colors ${
                active ? "text-white bg-[#121214] border-b border-cyan-400" : "text-zinc-500 hover:text-zinc-300"}`}>
              <Icon className={`w-2.5 h-2.5 ${active ? "text-cyan-400" : ""}`} />
              <span>{tab.label}</span>
            </button>
          );
        })}
      </div>

      {/* 3. 内容区 */}
      <div className="flex-1 overflow-y-auto min-h-0">
        {/* ── 真实文件树 ── */}
        {activeTab === "files" && (
          <div className="p-2 space-y-0.5">
            {vfsTree.length === 0 ? (
              <div className="text-center text-zinc-600 py-4 text-[10px]">
                <FolderTree className="w-4 h-4 mx-auto mb-1 opacity-50" />
                {currentProject === "default" ? "创建项目后显示文件树" : "加载中..."}
              </div>
            ) : (
              vfsTree.map((node, idx) => (
                <div key={idx} className="flex items-center justify-between px-2 py-1 rounded hover:bg-zinc-900/60 text-zinc-300 transition-colors text-[10px]">
                  <div className="flex items-center space-x-1.5 truncate">
                    <span>{node.is_dir ? "📁" : "📄"}</span>
                    <span className={node.is_locked ? "text-amber-400" : ""}>{node.name}</span>
                  </div>
                  <div className="flex items-center space-x-1 shrink-0">
                    {node.is_locked && <span className="text-[7px] bg-amber-950 text-amber-400 border border-amber-900/40 px-1 rounded">LOCKED</span>}
                    <span className="text-zinc-600 text-[8px]">{node.relative_path}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {/* ── 检查点时间线 ── */}
        {activeTab === "checkpoints" && (
          <div className="p-2 space-y-2">
            {checkpoints.length === 0 ? (
              <div className="text-center text-zinc-600 py-4 text-[10px]">
                <Camera className="w-4 h-4 mx-auto mb-1 opacity-50" />
                暂无检查点 — 点击 📸 创建
              </div>
            ) : (
              checkpoints.map(cp => (
                <div key={cp.id} className="group relative pl-4 border-l border-zinc-800 hover:border-cyan-500 pb-2 transition-all last:pb-0">
                  <span className="absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-zinc-800 group-hover:bg-cyan-400 transition-all" />
                  <div className="flex items-center justify-between text-[10px]">
                    <span className="font-bold text-zinc-300 group-hover:text-cyan-400">{cp.title}</span>
                    <span className="text-zinc-600 text-[9px]">{cp.time}</span>
                  </div>
                  <p className="text-[10px] text-zinc-500 mt-0.5">{cp.desc}</p>
                  <div className="flex space-x-1 mt-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button onClick={() => handleRestore(cp.id)}
                      className="text-[8px] bg-amber-950/50 border border-amber-700/40 text-amber-400 px-1 py-0.5 rounded hover:bg-amber-900/50">
                      <RotateCcw className="w-2 h-2 inline mr-0.5" />恢复</button>
                    <button onClick={() => handleDeleteCp(cp.id)}
                      className="text-[8px] bg-red-950/50 border border-red-700/40 text-red-400 px-1 py-0.5 rounded hover:bg-red-900/50">
                      <Trash2 className="w-2 h-2 inline mr-0.5" />删除</button>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {/* ── 项目健康 + Worktree 状态 ── */}
        {activeTab === "health" && (
          <div className="p-2 space-y-2">
            {/* 健康面板 */}
            {health ? (
              <div className="p-2 border border-[#27272a] rounded bg-[#121214] space-y-1.5 text-[10px]">
                <div className="flex items-center justify-between">
                  <span className="text-zinc-400 font-bold">📊 项目健康</span>
                  <span className={`px-1 py-0.5 rounded text-[9px] ${
                    health.status === "healthy" ? "text-emerald-400 bg-emerald-950/40" : "text-red-400 bg-red-950/40"
                  }`}>{health.status}</span>
                </div>
                <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-zinc-500">
                  <span>文件: <b className="text-zinc-300">{health.file_count}</b></span>
                  <span>大小: <b className="text-zinc-300">{fmtSize(health.total_size_bytes)}</b></span>
                  <span>Git: <b className={health.has_git ? "text-emerald-400" : "text-zinc-600"}>{health.has_git ? "✅" : "❌"}</b></span>
                  <span>检查点: <b className="text-zinc-300">{health.checkpoint_count}</b></span>
                </div>
                {health.last_checkpoint && (
                  <div className="text-[9px] text-zinc-600">最近检查点: {health.last_checkpoint}</div>
                )}
              </div>
            ) : (
              <div className="text-center text-zinc-600 py-4 text-[10px]">创建项目后显示健康状态</div>
            )}

            {/* Worktree 面板 */}
            <div className="p-2 border border-[#27272a] rounded bg-[#121214] space-y-1.5 text-[10px]">
              <div className="flex items-center justify-between">
                <span className="text-zinc-400 font-bold">🌿 Worktrees</span>
                <span className="text-zinc-600 text-[9px]">{wtStats.total} 总计</span>
              </div>
              <div className="flex space-x-2 text-[9px] text-zinc-500">
                <span className="text-cyan-400">{wtStats.active} 活跃</span>
                <span className="text-emerald-400">{wtStats.completed} 完成</span>
                <span className="text-purple-400">{wtStats.merged} 已合并</span>
                {wtStats.errors > 0 && <span className="text-red-400">{wtStats.errors} 错误</span>}
              </div>
              {worktrees.length > 0 ? (
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {worktrees.map(wt => (
                    <div key={wt.id} className="flex items-center justify-between text-[9px] px-1.5 py-0.5 rounded bg-[#0a0a0c] group">
                      <span className="text-zinc-400 truncate">{wt.id}</span>
                      <div className="flex items-center space-x-1">
                        <span className="text-zinc-600">{wt.branch}</span>
                        <button onClick={() => handleMergeWorktree(wt.id)}
                          className="opacity-0 group-hover:opacity-100 text-[8px] bg-purple-950/50 border border-purple-700/40 text-purple-400 px-1 py-0.5 rounded hover:bg-purple-900/50 transition-all"
                          title="合并到主分支">
                          <GitMerge className="w-2 h-2 inline mr-0.5" />合并</button>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-zinc-600 text-[9px]">无活跃 Worktree</div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="h-5 border-t border-[#27272a] px-2.5 flex items-center text-[8px] text-zinc-500 shrink-0">
        <HardDrive className="w-2 h-2 mr-1" />
        {sandboxStatus} · {checkpoints.length} 检查点 · {vfsTree.length} 文件 · {wtStats.total} WT
      </div>
    </div>
  );
}
