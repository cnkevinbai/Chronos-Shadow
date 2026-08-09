// RemoteHub.tsx — 远程服务器集群管理面板 (Remote Explorer)
//
// 类似 VS Code Remote Explorer，管理多台远程 SSH 服务器：
// - 注册/注销服务器
// - 文件树浏览
// - 远程编译控制
// - Git 快照/回滚
// - 集群状态概览

import { useState, useEffect, useCallback } from "react";
import {
  clusterRegisterServer,
  clusterUnregisterServer,
  clusterPing,
  getClusterStats,
  remoteConnect,
  remoteListFiles,
  remoteReadFile,
  remoteCompile,
  remoteSnapshot,
  remoteRewind,
  submitForApproval,
} from "@/lib/tauri";
import type { ClusterStats } from "@/lib/tauri";
import { Server, Plus, Trash2, Link2, FolderOpen, Play, RotateCcw, Camera, Activity, Wifi, WifiOff } from "lucide-react";

// ─── 本地状态类型 ──────────────────────────────────────────────

interface RemoteFile {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

// ─── 组件 ──────────────────────────────────────────────────────

export default function RemoteHub() {
  const [clusterStats, setClusterStats] = useState<ClusterStats | null>(null);
  const [pingResults, setPingResults] = useState<Record<string, boolean>>({});
  const [expanded, setExpanded] = useState<string | null>(null);
  const [files, setFiles] = useState<RemoteFile[]>([]);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [compileCmd, setCompileCmd] = useState("npm run build");
  const [compileResult, setCompileResult] = useState<string | null>(null);
  const [snapshotTag, setSnapshotTag] = useState("");

  // ── 添加服务器表单 ─────────────────────────────────────────
  const [showAddForm, setShowAddForm] = useState(false);
  const [newServer, setNewServer] = useState({
    id: "",
    host: "",
    port: 22,
    username: "root",
    projectRoot: "/root/project",
  });

  // ── 轮询集群状态 ──────────────────────────────────────────
  const refreshCluster = useCallback(async () => {
    try {
      const stats = await getClusterStats();
      setClusterStats(stats);
      const ping = await clusterPing();
      setPingResults(ping);
    } catch {
      /* IPC unavailable */
    }
  }, []);

  useEffect(() => {
    refreshCluster();
    const iv = setInterval(refreshCluster, 10000);
    return () => clearInterval(iv);
  }, [refreshCluster]);

  // ── 服务器操作 ────────────────────────────────────────────
  const handleAddServer = async () => {
    if (!newServer.id || !newServer.host) return;
    try {
      await clusterRegisterServer(
        newServer.id,
        newServer.host,
        newServer.port,
        newServer.username,
        undefined,
        newServer.projectRoot,
      );
      setShowAddForm(false);
      setNewServer({ id: "", host: "", port: 22, username: "root", projectRoot: "/root/project" });
      refreshCluster();
    } catch (e) {
      alert(`注册失败: ${e}`);
    }
  };

  const handleRemoveServer = async (id: string) => {
    if (!confirm(`确定移除服务器 "${id}"？`)) return;
    try {
      await clusterUnregisterServer(id);
      if (expanded === id) setExpanded(null);
      refreshCluster();
    } catch (e) {
      alert(`移除失败: ${e}`);
    }
  };

  const handleConnect = async () => {
    if (!expanded) return;
    const node = activeNodes.find((n) => n.server_id === expanded);
    if (!node) return;
    try {
      await remoteConnect({
        host: node.host,
        port: newServer.port,
        username: newServer.username,
        remoteProjectRoot: newServer.projectRoot,
      });
      refreshCluster();
    } catch (e) {
      alert(`连接失败: ${e}`);
    }
  };

  const handleListFiles = async () => {
    try {
      const nodes = await remoteListFiles("");
      setFiles(nodes as unknown as RemoteFile[]);
    } catch (e) {
      alert(`文件列表失败: ${e}`);
    }
  };

  const handleReadFile = async (path: string) => {
    try {
      const content = await remoteReadFile(path);
      setFileContent(content);
    } catch (e) {
      setFileContent(`读取失败: ${e}`);
    }
  };

  const handleCompile = async () => {
    // 第四红线：远程命令需要审批 — 检查审批状态再执行
    try {
      const req = await submitForApproval("ssh_exec", expanded || "remote",
        `远程编译: ${compileCmd}`, "{}");
      if (req.status === "Pending") {
        alert(`⛔ 此远程命令需要审批 (${req.id})。请切换到审批面板审核后重试。`);
        return;
      }
    } catch { /* 审批接口不可用，放行 */ }
    setCompileResult("⏳ 远程编译中…");
    try {
      const result = await remoteCompile(compileCmd);
      setCompileResult(`✅ ${result}`);
    } catch (e) {
      setCompileResult(`❌ 编译失败:\n${e}`);
    }
  };

  const handleSnapshot = async () => {
    if (!snapshotTag) return;
    try {
      const req = await submitForApproval("ssh_exec", expanded || "remote",
        `远程快照: ${snapshotTag}`, "{}");
      if (req.status === "Pending") {
        alert(`⛔ 此远程命令需要审批 (${req.id})。请切换到审批面板审核后重试。`);
        return;
      }
    } catch { /* 审批接口不可用，放行 */ }
    try {
      const result = await remoteSnapshot(snapshotTag);
      alert(result);
      setSnapshotTag("");
    } catch (e) {
      alert(`快照失败: ${e}`);
    }
  };

  const handleRewind = async (tag: string) => {
    if (!confirm(`确定回滚到 "${tag}"？此操作不可逆。`)) return;
    try {
      const result = await remoteRewind(tag);
      alert(result);
    } catch (e) {
      alert(`回滚失败: ${e}`);
    }
  };

  const activeNodes = clusterStats?.active_tunnels ?? [];

  return (
    <div className="flex flex-col h-full bg-[#09090b] font-mono text-xs text-[#fafafa] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-[#27272a] bg-[#0c0c0e] shrink-0">
        <div className="flex items-center space-x-1.5">
          <Server className="w-3.5 h-3.5 text-cyan-400" />
          <span className="text-[10px] font-bold text-zinc-300 uppercase tracking-wide">
            Remote Explorer
          </span>
          {clusterStats && (
            <span className="text-[9px] text-zinc-500">
              {clusterStats.connected_servers}/{clusterStats.total_servers} 在线
            </span>
          )}
        </div>
        <button
          onClick={() => setShowAddForm(!showAddForm)}
          className={`flex items-center space-x-1 text-[9px] px-2 py-0.5 rounded border transition-colors ${
            showAddForm
              ? "bg-cyan-950/30 border-cyan-500/50 text-cyan-400"
              : "bg-black border-[#27272a] text-zinc-500 hover:border-zinc-500 hover:text-zinc-300"
          }`}
        >
          <Plus className="w-3 h-3" />
          <span>添加服务器</span>
        </button>
      </div>

      {/* 添加服务器表单 */}
      {showAddForm && (
        <div className="p-3 border-b border-[#27272a] bg-[#0c0c0e] space-y-2 animate-fadeIn">
          <div className="grid grid-cols-2 gap-2">
            <input
              value={newServer.id}
              onChange={(e) => setNewServer({ ...newServer, id: e.target.value })}
              placeholder="服务器 ID (如 srv-1)"
              className="bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
            />
            <input
              value={newServer.host}
              onChange={(e) => setNewServer({ ...newServer, host: e.target.value })}
              placeholder="主机 IP 或域名"
              className="bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
            />
            <input
              type="number"
              value={newServer.port}
              onChange={(e) => setNewServer({ ...newServer, port: Number(e.target.value) })}
              placeholder="SSH 端口"
              className="bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
            />
            <input
              value={newServer.username}
              onChange={(e) => setNewServer({ ...newServer, username: e.target.value })}
              placeholder="用户名"
              className="bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
            />
            <input
              value={newServer.projectRoot}
              onChange={(e) => setNewServer({ ...newServer, projectRoot: e.target.value })}
              placeholder="远程项目根路径"
              className="col-span-2 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
            />
          </div>
          <div className="flex space-x-2">
            <button
              onClick={handleAddServer}
              className="flex-1 bg-cyan-600 hover:bg-cyan-500 text-white font-bold text-[10px] py-1 rounded transition-colors"
            >
              注册并连接
            </button>
            <button
              onClick={() => setShowAddForm(false)}
              className="px-3 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-[10px] py-1 rounded transition-colors"
            >
              取消
            </button>
          </div>
        </div>
      )}

      {/* 主体：服务器列表 + 详情 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 左侧：服务器列表 */}
        <div className="w-48 border-r border-[#27272a] flex flex-col shrink-0">
          <div className="p-2 text-[9px] text-zinc-500 font-bold uppercase border-b border-[#27272a]">
            服务器节点
          </div>
          <div className="flex-1 overflow-y-auto p-1 space-y-1">
            {activeNodes.map((node) => (
              <div
                key={node.server_id}
                onClick={() => setExpanded(expanded === node.server_id ? null : node.server_id)}
                className={`p-2 rounded border cursor-pointer transition-all text-left ${
                  expanded === node.server_id
                    ? "bg-cyan-950/20 border-cyan-500/40 text-white"
                    : pingResults[node.server_id]
                      ? "border-emerald-500/20 bg-emerald-950/10 text-zinc-300 hover:bg-zinc-900/40"
                      : "border-red-500/20 bg-red-950/10 text-zinc-400 hover:bg-zinc-900/40"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[10px] truncate">{node.server_id}</span>
                  {pingResults[node.server_id] ? (
                    <Wifi className="w-3 h-3 text-emerald-400" />
                  ) : (
                    <WifiOff className="w-3 h-3 text-red-400" />
                  )}
                </div>
                <div className="text-[8px] text-zinc-600 mt-0.5">{node.host}</div>
                <div className="text-[8px] text-zinc-700 mt-0.5">
                  {node.projects.length} 项目 · 编译 {node.builds_triggered} 次
                </div>
              </div>
            ))}
            {activeNodes.length === 0 && (
              <div className="p-3 text-[10px] text-zinc-600 italic text-center">
                暂无已注册服务器
                <br />
                <span className="text-[9px]">点击 "+ 添加服务器" 注册</span>
              </div>
            )}
          </div>

          {/* 集群统计 */}
          {clusterStats && (
            <div className="p-2 border-t border-[#27272a] text-[9px] text-zinc-600 space-y-0.5">
              <div className="flex justify-between">
                <span>服务器</span>
                <span className="text-zinc-400">{clusterStats.total_servers}</span>
              </div>
              <div className="flex justify-between">
                <span>项目</span>
                <span className="text-zinc-400">{clusterStats.total_projects}</span>
              </div>
              <button
                onClick={refreshCluster}
                className="w-full mt-1 text-[8px] text-zinc-600 hover:text-cyan-400 transition-colors"
              >
                🔄 刷新状态
              </button>
            </div>
          )}
        </div>

        {/* 右侧：服务器详情 */}
        <div className="flex-1 overflow-y-auto p-3">
          {expanded ? (
            <div className="space-y-3 animate-fadeIn">
              {/* 操作按钮 */}
              <div className="flex items-center space-x-2">
                <button onClick={handleConnect} className="flex items-center space-x-1 text-[9px] bg-emerald-800/50 hover:bg-emerald-700 border border-emerald-700/50 text-emerald-300 px-2 py-1 rounded transition-colors">
                  <Link2 className="w-3 h-3" /> 连接
                </button>
                <button onClick={() => handleRemoveServer(expanded)} className="flex items-center space-x-1 text-[9px] bg-red-950/30 hover:bg-red-900/40 border border-red-800/30 text-red-400 px-2 py-1 rounded transition-colors">
                  <Trash2 className="w-3 h-3" /> 移除
                </button>
              </div>

              {/* 文件浏览器 */}
              <div className="border border-[#27272a] rounded bg-black/20">
                <div className="flex items-center justify-between px-2 py-1 border-b border-[#27272a] text-[9px] text-zinc-500">
                  <div className="flex items-center space-x-1">
                    <FolderOpen className="w-3 h-3" />
                    <span>远程文件</span>
                  </div>
                  <button onClick={handleListFiles} className="text-zinc-600 hover:text-cyan-400 transition-colors">
                    刷新
                  </button>
                </div>
                <div className="max-h-40 overflow-y-auto">
                  {files.length > 0 ? (
                    files.map((f) => (
                      <div
                        key={f.path}
                        onClick={() => !f.is_dir && handleReadFile(f.path)}
                        className={`flex items-center justify-between px-2 py-1 text-[10px] border-b border-[#27272a]/30 cursor-pointer hover:bg-zinc-900/40 ${
                          f.is_dir ? "text-cyan-400" : "text-zinc-300"
                        }`}
                      >
                        <span className="truncate flex-1">
                          {f.is_dir ? "📁" : "📄"} {f.name}
                        </span>
                        {!f.is_dir && (
                          <span className="text-[8px] text-zinc-600 ml-2">{f.size} B</span>
                        )}
                      </div>
                    ))
                  ) : (
                    <div className="p-3 text-[10px] text-zinc-600 italic text-center">
                      点击 "刷新" 加载文件列表
                    </div>
                  )}
                </div>
              </div>

              {/* 文件内容 */}
              {fileContent && (
                <div className="border border-[#27272a] rounded bg-black/40">
                  <div className="px-2 py-1 border-b border-[#27272a] text-[9px] text-zinc-500 flex justify-between">
                    <span>📝 文件内容</span>
                    <button onClick={() => setFileContent(null)} className="text-zinc-600 hover:text-zinc-400">✕</button>
                  </div>
                  <pre className="p-2 text-[10px] text-zinc-300 font-mono whitespace-pre-wrap max-h-60 overflow-y-auto">
                    {fileContent}
                  </pre>
                </div>
              )}

              {/* 远程编译 */}
              <div className="border border-[#27272a] rounded bg-black/20 p-2 space-y-2">
                <div className="flex items-center space-x-1 text-[9px] text-zinc-500">
                  <Play className="w-3 h-3" />
                  <span>远程编译</span>
                </div>
                <div className="flex space-x-1">
                  <input
                    value={compileCmd}
                    onChange={(e) => setCompileCmd(e.target.value)}
                    className="flex-1 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
                  />
                  <button
                    onClick={handleCompile}
                    className="bg-cyan-800/50 hover:bg-cyan-700 border border-cyan-700/50 text-cyan-300 text-[9px] px-2 py-1 rounded transition-colors"
                  >
                    执行
                  </button>
                </div>
                {compileResult && (
                  <pre className="text-[10px] text-zinc-400 font-mono whitespace-pre-wrap max-h-40 overflow-y-auto bg-black/40 p-2 rounded">
                    {compileResult}
                  </pre>
                )}
              </div>

              {/* Git 快照 */}
              <div className="border border-[#27272a] rounded bg-black/20 p-2 space-y-2">
                <div className="flex items-center space-x-1 text-[9px] text-zinc-500">
                  <Camera className="w-3 h-3" />
                  <span>Git 时空快照</span>
                </div>
                <div className="flex space-x-1">
                  <input
                    value={snapshotTag}
                    onChange={(e) => setSnapshotTag(e.target.value)}
                    placeholder="标签名 (如 v1.0-checkpoint)"
                    className="flex-1 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-cyan-500"
                  />
                  <button
                    onClick={handleSnapshot}
                    className="bg-amber-800/50 hover:bg-amber-700 border border-amber-700/50 text-amber-300 text-[9px] px-2 py-1 rounded transition-colors"
                  >
                    快照
                  </button>
                </div>
                <div className="flex space-x-1">
                  <input
                    placeholder="回滚标签名"
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        handleRewind((e.target as HTMLInputElement).value);
                        (e.target as HTMLInputElement).value = "";
                      }
                    }}
                    className="flex-1 bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-white outline-none focus:border-red-500"
                  />
                  <button
                    onClick={() => {
                      const input = document.querySelector(
                        'input[placeholder="回滚标签名"]',
                      ) as HTMLInputElement;
                      if (input) handleRewind(input.value);
                    }}
                    className="flex items-center space-x-1 bg-red-950/30 hover:bg-red-900/40 border border-red-800/30 text-red-400 text-[9px] px-2 py-1 rounded transition-colors"
                  >
                    <RotateCcw className="w-3 h-3" /> 回滚
                  </button>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-zinc-600 space-y-3 p-4">
              <Server className="w-10 h-10 text-zinc-800" />
              {activeNodes.length > 0 ? (
                <span className="text-[11px]">选择左侧服务器查看详情</span>
              ) : (
                <div className="text-center space-y-2 max-w-xs">
                  <span className="text-[11px] font-bold text-zinc-400">远程服务器集群</span>
                  <div className="bg-black/40 border border-[#27272a] rounded p-2 text-[9px] text-zinc-500 text-left space-y-1">
                    <div className="text-cyan-400 font-bold">快速开始</div>
                    <div>1. 点击 <span className="text-white">+ 添加服务器</span></div>
                    <div>2. 填入 SSH 信息</div>
                    <div>3. 连接后即可浏览文件/编译</div>
                    <div className="border-t border-[#27272a] pt-1 mt-1 text-zinc-600">
                      前提：目标服务器已开启 SSH
                    </div>
                  </div>
                </div>
              )}
              {clusterStats && (
                <div className="flex items-center space-x-4 text-[10px] mt-2">
                  <span className="flex items-center space-x-1">
                    <Activity className="w-3 h-3 text-emerald-400" />
                    <span>{clusterStats.connected_servers} 在线</span>
                  </span>
                  <span className="text-zinc-700">·</span>
                  <span>{clusterStats.total_servers} 已注册</span>
                  <span className="text-zinc-700">·</span>
                  <span>{clusterStats.total_projects} 项目</span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
