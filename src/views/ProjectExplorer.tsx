// 智能沙盒与多维时空管理器面板 (Project & Chronos Explorer)
// C-VFS 升级版：项目矩阵 + VFS虚拟文件树 + Chronos Rail 时光机轨道
// v2: 接入 C-VFS 后端命令（创建项目、手动快照、实时列表）

import { useState, useEffect } from "react";
import { useT } from "@/lib/i18n-context";
import {
  cvfsGetCheckpoints,
  cvfsGetProjects,
  cvfsCreateProject,
  cvfsCaptureCheckpoint,
  getSandboxStatus,
} from "@/lib/tauri";
import type { SystemSnapshot } from "@/lib/types";
import { Plus, Shield, Clock, HardDrive, Camera } from "lucide-react";

interface VfsNode {
  name: string;
  isDir: boolean;
  isLocked: boolean;
  path: string;
}

const VFS_TREE: VfsNode[] = [
  { name: "CLAUDE.md", isDir: false, isLocked: true, path: "./CLAUDE.md" },
  { name: "src-tauri/", isDir: true, isLocked: false, path: "./src-tauri" },
  { name: "src/", isDir: true, isLocked: false, path: "./src" },
  { name: "package.json", isDir: false, isLocked: false, path: "./package.json" },
  { name: "vite.config.ts", isDir: false, isLocked: false, path: "./vite.config.ts" },
  { name: "skills/", isDir: true, isLocked: false, path: "./skills" },
];

interface CheckpointEntry {
  id: string;
  title: string;
  desc: string;
  time: string;
}

function mapSnapshot(s: SystemSnapshot): CheckpointEntry {
  return {
    id: s.id,
    title: s.label,
    desc: `${s.files_changed} files changed · ${s.snapshot_type}`,
    time: s.timestamp.split("T")[1]?.slice(0, 8) ?? s.timestamp.slice(0, 8),
  };
}

interface ProjectEntry {
  id: string;
  name: string;
  path: string;
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
  const [showNewProject, setShowNewProject] = useState(false);
  const [newProjId, setNewProjId] = useState("");
  const [newProjPath, setNewProjPath] = useState("C:\\Chronos-Workspace");
  const [snapshotLabel, setSnapshotLabel] = useState("");
  const [showSnapshot, setShowSnapshot] = useState(false);

  const refresh = async () => {
    try {
      const [cps, projs, status] = await Promise.all([
        cvfsGetCheckpoints(),
        cvfsGetProjects(),
        getSandboxStatus(),
      ]);
      if (cps.length > 0) setCheckpoints(cps.map(mapSnapshot));
      if (projs.length > 0) setProjects(projs);
      setSandboxStatus(status);
    } catch { /* IPC unavailable */ }
  };

  useEffect(() => { refresh(); const iv = setInterval(refresh, 8000); return () => clearInterval(iv); }, []);

  const handleCreateProject = async () => {
    if (!newProjId) return;
    try {
      await cvfsCreateProject(newProjId, newProjPath);
      setShowNewProject(false);
      setNewProjId("");
      onProjectChange(newProjId);
      refresh();
    } catch (e) { alert(`创建失败: ${e}`); }
  };

  const handleCaptureCheckpoint = async () => {
    if (!snapshotLabel) return;
    try {
      await cvfsCaptureCheckpoint(currentProject, `cp-${Date.now()}`, snapshotLabel);
      setSnapshotLabel("");
      setShowSnapshot(false);
      refresh();
    } catch (e) { alert(`快照失败: ${e}`); }
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
              className="text-[9px] bg-black border border-amber-500/30 hover:border-amber-400 text-amber-400 px-1.5 py-0.5 rounded transition-all"
              title="手动快照">
              <Camera className="w-2.5 h-2.5 inline mr-0.5" />快照
            </button>
            <button onClick={() => setShowNewProject(!showNewProject)}
              className="text-[10px] bg-black border border-[#27272a] hover:border-zinc-500 text-white px-1.5 py-0.5 rounded font-bold transition-all active:scale-95">
              <Plus className="w-2.5 h-2.5 inline mr-0.5" />{t.new_button}
            </button>
          </div>
        </div>

        {/* 快照输入 */}
        {showSnapshot && (
          <div className="flex space-x-1 animate-fadeIn">
            <input value={snapshotLabel} onChange={(e) => setSnapshotLabel(e.target.value)}
              placeholder="快照标签 (如 Before-Refactor)"
              className="flex-1 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-amber-500" />
            <button onClick={handleCaptureCheckpoint}
              className="bg-amber-800/50 hover:bg-amber-700 border border-amber-700/50 text-amber-300 text-[9px] px-2 py-1 rounded">捕获</button>
          </div>
        )}

        {/* 新建项目表单 */}
        {showNewProject && (
          <div className="space-y-1 animate-fadeIn">
            <input value={newProjId} onChange={(e) => setNewProjId(e.target.value)}
              placeholder="项目 ID (如 my-project)" className="w-full bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500" />
            <input value={newProjPath} onChange={(e) => setNewProjPath(e.target.value)}
              placeholder="物理路径" className="w-full bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500" />
            <button onClick={handleCreateProject}
              className="w-full bg-cyan-800/50 hover:bg-cyan-700 text-cyan-300 text-[9px] py-1 rounded font-bold">创建项目并锁定 Scope</button>
          </div>
        )}

        <select
          value={displayProjects.find((p) => p.name === currentProject)?.id || ""}
          onChange={(e) => {
            const selected = displayProjects.find((p) => p.id === e.target.value);
            if (selected) onProjectChange(selected.name);
          }}
          className="bg-black border border-[#27272a] rounded px-2 py-1 text-xs text-white outline-none cursor-pointer w-full"
        >
          {displayProjects.length > 0 ? displayProjects.map((p) => (
            <option key={p.id} value={p.id}>{p.name} [{p.path.slice(0, 30)}]</option>
          )) : (
            <option value="">暂无项目 — 点击 + 新建</option>
          )}
        </select>
        <div className="flex items-center space-x-2 text-[9px] text-zinc-500">
          <Shield className="w-2.5 h-2.5 text-cs-accent" />
          <span>{currentProject}</span>
          <span className="text-cs-accent ml-auto">{sandboxStatus}</span>
        </div>
      </div>

      {/* 2. C-VFS 虚拟目录浏览器 */}
      <div className="flex-1 flex flex-col min-h-0 border-b border-[#27272a]">
        <div className="px-3 py-1.5 text-[10px] font-bold text-zinc-600 bg-black/40 uppercase tracking-wider">
          📂 {t.files}
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
          {VFS_TREE.map((node, idx) => (
            <div key={idx}
              className="flex items-center justify-between px-2 py-1 rounded hover:bg-zinc-900/60 cursor-pointer text-zinc-300 transition-colors">
              <div className="flex items-center space-x-2 truncate">
                <span className="text-[11px]">{node.isDir ? "📁" : "📄"}</span>
                <span className={node.isLocked ? "text-amber-400 font-bold" : ""}>{node.name}</span>
              </div>
              {node.isLocked && (
                <span className="text-[7px] bg-amber-950 text-amber-400 border border-amber-900/40 px-1 rounded shrink-0">{t.lock_badge}</span>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* 3. 时光机树轴 */}
      <div className="h-52 flex flex-col min-h-0 bg-black/20 shrink-0">
        <div className="px-3 py-1.5 text-[10px] font-bold text-zinc-600 bg-black/40 uppercase tracking-wider flex items-center space-x-1.5">
          <Clock className="w-3 h-3" />
          <span>{t.chrono_trigger}</span>
          <span className="text-zinc-500 ml-auto">{checkpoints.length}</span>
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-2">
          {checkpoints.map((ckpt) => (
            <div key={ckpt.id}
              className="group relative pl-4 border-l border-zinc-800 hover:border-cyan-500 pb-2 cursor-pointer transition-all last:pb-0">
              <span className="absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-zinc-800 group-hover:bg-cyan-400 shadow-sm transition-all" />
              <div className="flex items-center justify-between text-[10px]">
                <span className="font-bold text-zinc-300 group-hover:text-cyan-400 transition-colors">{ckpt.title}</span>
                <span className="text-zinc-600 group-hover:text-cyan-500 font-light">{ckpt.time}</span>
              </div>
              <p className="text-[10px] text-zinc-500 line-clamp-1 mt-0.5 group-hover:text-zinc-400 font-light">{ckpt.desc}</p>
            </div>
          ))}
          {checkpoints.length === 0 && (
            <div className="text-[10px] text-zinc-600 italic p-2 text-center">暂无快照 — 点击 📸 手动创建</div>
          )}
        </div>
        <div className="h-5 border-t border-[#27272a] px-2.5 flex items-center text-[8px] text-zinc-500">
          <HardDrive className="w-2 h-2 mr-1" />
          {sandboxStatus} · {checkpoints.length} {t.snapshots}
        </div>
      </div>
    </div>
  );
}
