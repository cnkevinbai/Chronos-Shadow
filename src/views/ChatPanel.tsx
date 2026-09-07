// src/views/ChatPanel.tsx — 沉浸式高级全局对话视窗 (Omni-Chat Console)
//
// v2: 接入真实 chatApi IPC → Rust 后端 → 云端 LLM
// v3: IDE 风格历史会话侧栏 → session_db 持久化 + DeepSeek Context Caching 一折命中
// v4: 流式分块会话数据库 (Chunked V2) → caching_marker_hash + 财务审计 + Cache-Aligned 徽章
// 若未配置 API Key 则降级为本地 mock 演示

import React, { useState, useEffect, useRef, useCallback } from "react";
import { MessageSquare, Coins, Upload, Save, Search, Lightbulb, RefreshCw, Link2, Zap, FolderOpen, FileText, Image as LucideImage, Copy } from "lucide-react";
import { useT } from "@/lib/i18n-context";
import ArtifactPanel from "@/views/chat/ArtifactPanel";
import Composer from "@/views/chat/Composer";
import SessionSidebar from "@/views/chat/SessionSidebar";
import { useToast } from "@/lib/use-toast";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getModelDisplay } from "@/lib/models";
import { APP_VERSION } from "@/lib/version";
import {
  chatApiStream,
  onChatStreamChunk,
  getModelEndpoint,
  saveChatSessionChunk,
  loadChatSessionChunk,
  listHistoricalMetaManifests,
  listSessionsByProject,
  deleteChatSession,
  exportChatSession,
  importChatSession,
  extractAndExecuteActions,
  cvfsListProjectFiles,
  cvfsGetProjects,
  cancelChatStream,
} from "@/lib/tauri";

// 真实文件对话框（Tauri 环境可用，浏览器降级为 mock）
let dialogOpen: ((options: {
  multiple?: boolean;
  filters?: { name: string; extensions: string[] }[];
}) => Promise<string | string[] | null>) | null = null;
(async () => {
  try {
    const mod = await import("@tauri-apps/plugin-dialog");
    dialogOpen = mod.open;
  } catch {
    /* browser dev — mock 模式 */
  }
})();
import type { SessionMetaManifest, Attachment } from "@/lib/types";
import { renderMarkdown } from "@/lib/utils";
import { createElement, type ReactNode } from "react";


interface Message {
  id: string;
  sender:
    | "User"
    | "PM"
    | "UI Designer"
    | "Coder"
    | "System"
    | "Explore"
    | "Auditor"
    | "Scout"
    | "Compaction";
  model: string;
  content: string;
  /** 挂载的多模态文档或图片附件 */
  attachments?: Attachment[];
  thinking?: string;
  costTokens?: number;
  isCached?: boolean;
  timestamp: string;
  /** Rust 端 SHA256 链式累积缓存特征哈希（加载历史会话时回传） */
  cachingMarkerHash?: string;
}

interface ChatPanelProps {
  selectedModel: string;
  apiKey?: string;
  hasKeys?: { deepseek: boolean; kimi: boolean; glm: boolean };
  currentProject?: string;
  onProjectChange?: (project: string) => void;
}

function modelDisplayName(model: string): string {
  return getModelDisplay(model);
}

// 动态文件扩展名颜色 — 支持任意类型, 哈希映射保证稳定
function deriveTitle(messages: Message[]): string {
  const firstUser = messages.find((m) => m.sender === "User");
  if (firstUser) {
    const txt = firstUser.content.replace(/\s+/g, " ").trim();
    return txt.length > 24 ? txt.slice(0, 24) + "…" : txt;
  }
  return "未命名研发 Quest";
}

// ─── Markdown 渲染辅助组件 ──────────────────────────────────────

type MdNode = string | { type: string; props: Record<string, unknown> };

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

// ─── 组件 ──────────────────────────────────────────────────────────

export default function ChatPanel({
  selectedModel,
  apiKey = "",
  hasKeys = { deepseek: false, kimi: false, glm: false },
  currentProject = "default",
  onProjectChange,
}: ChatPanelProps) {
  const keyForModel = (m: string) => m.startsWith("deepseek") ? hasKeys.deepseek : m.startsWith("kimi") ? hasKeys.kimi : m.startsWith("glm") ? hasKeys.glm : false;
  const currentHasKey = keyForModel(selectedModel);
  const anyKey = hasKeys.deepseek || hasKeys.kimi || hasKeys.glm;
  const availableProvider = hasKeys.deepseek ? "DeepSeek" : hasKeys.kimi ? "Kimi" : hasKeys.glm ? "GLM" : null;
  const t = useT();
  const toast = useToast();

  // ── 会话侧栏状态（Chunked V2）─────────────────────────────────
  const [manifests, setManifests] = useState<SessionMetaManifest[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string>(
    () => `sess-${Date.now()}`,
  );
  const [isSaving, setIsSaving] = useState(false);

  // 冷启动：极速清单流式 Lazy 加载——仅读取轻量 .meta 文件，杜绝卡顿
  const refreshManifests = useCallback(() => {
    listHistoricalMetaManifests()
      .then((res) => {
        if (res) setManifests(res);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    refreshManifests();
  }, [refreshManifests]);

  // 加载项目列表
  useEffect(() => {
    cvfsGetProjects().then(p => { if (p.length) setProjectList(p); }).catch(() => {});
  }, []);

  // 当项目切换时，自动刷新该项目关联的会话列表
  useEffect(() => {
    if (currentProject && currentProject !== "default") {
      listSessionsByProject(currentProject)
        .then((res) => {
          if (res && res.length > 0) setManifests(res);
          else refreshManifests();
        })
        .catch(() => refreshManifests());
      // 加载项目文件列表
      cvfsListProjectFiles(currentProject).then(files => setProjectFiles(files)).catch(() => {});
    } else {
      setProjectFiles([]);
    }
  }, [currentProject, refreshManifests]);

  // ── 聊天状态 ─────────────────────────────────────────────────
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<Message[]>([
    {
      id: "msg-0",
      sender: "System",
      model: "Local System",
      content: anyKey ? t.chat_welcome_connected : t.chat_welcome_demo,
      timestamp: new Date().toLocaleTimeString(),
    },
  ]);

  // React to key presence change after startup load or view switch
  useEffect(() => {
    setMessages((prev) => [
      {
        ...prev[0],
        content: anyKey
          ? `🦀 **Chronos-Shadow v${APP_VERSION}** 已就绪 ❤️\n\n${t.chat_welcome_connected}\n\n> *每一次交互，都让系统更懂你*\n> *所有数据端侧处理，你的隐私我们守护* 🔒`
          : t.chat_welcome_demo,
      },
      ...prev.slice(1),
    ]);
  }, [anyKey, t]);

  const [isThinking, setIsThinking] = useState(false);
  const [flowStage, setFlowStage] = useState<"idle"|"connecting"|"thinking"|"streaming"|"researching">("idle");
  const [flowStartMs, setFlowStartMs] = useState(0);
  const [flowTick, setFlowTick] = useState(0);
  const [macrosVisible, setMacrosVisible] = useState(false);

  // Flow stage animation timer
  useEffect(() => {
    if (!isThinking) return;
    const iv = setInterval(() => setFlowTick(t => t + 1), 200);
    return () => clearInterval(iv);
  }, [isThinking]);

  const flowDots = ".".repeat((flowTick % 3) + 1);
  const flowElapsed = isThinking ? ((Date.now() - flowStartMs) / 1000).toFixed(1) : "0.0";
  const stageLabel = { idle:"", connecting:"连接中", thinking:"推理中", streaming:"流式接收", researching:"研究中" }[flowStage];
  const [lastUserInput, setLastUserInput] = useState("");
  const [retryCount, setRetryCount] = useState(0);
  const [showFileExplorer, setShowFileExplorer] = useState(false);
  const [projectFiles, setProjectFiles] = useState<Array<{name:string;is_dir:boolean;relative_path:string}>>([]);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [projectList, setProjectList] = useState<Array<{id:string;name:string;path:string}>>([]);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const msgContainerRef = useRef<HTMLDivElement>(null);
  const isNearBottomRef = useRef(true);
  const messagesRef = useRef<Message[]>(messages);
  // Keep ref in sync for keyboard shortcuts that read stale closure
  useEffect(() => { messagesRef.current = messages; }, [messages]);

  // ── 多模态附件挂载状态 ─────────────────────────────────────
  const [stagedAttachments, setStagedAttachments] = useState<
    Attachment[]
  >([]);

  // ── 成品文件登记追踪 ───────────────────────────────────────
  const [artifacts, setArtifacts] = useState<Array<{path:string; type:string; createdAt:string; versions:number}>>([]);
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState("");

  // ── 键盘快捷键 ─────────────────────────────────────────────
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+Enter / Cmd+Enter → 发送
      if (
        (e.ctrlKey || e.metaKey) &&
        e.key === "Enter" &&
        !isThinking
      ) {
        e.preventDefault();
        const form = document.querySelector(
          'form[data-chat-form]',
        ) as HTMLFormElement | null;
        form?.requestSubmit();
      }
      // Ctrl+N → 新建会话
      if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        shortcutsRef.current.handleNewSession();
        return;
      }
      // Ctrl+S → 固化保存 (uses ref to avoid stale closure)
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === "s") {
        e.preventDefault();
        shortcutsRef.current.persistCurrentSession(messagesRef.current);
        setIsSaving(true);
        setTimeout(() => {
          setIsSaving(false);
          shortcutsRef.current.toast.showToast("success", "CHUNK COMMIT SUCCESS", "💾 时空分块与缓存特征点已安全写入物理磁盘档案库。");
          shortcutsRef.current.refreshManifests();
        }, 300);
        return;
      }
      // Ctrl+Shift+E → 导出
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "e") {
        e.preventDefault();
        shortcutsRef.current.handleExportSession();
        return;
      }
      // Ctrl+F / Cmd+F → 消息搜索
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        setSearchOpen(true);
        setTimeout(() => {
          const input = document.querySelector(
            'input[data-search-input]',
          ) as HTMLInputElement | null;
          input?.focus();
        }, 50);
        return;
      }
      // Escape → 关闭所有弹窗 + 取消附件
      if (e.key === "Escape") {
        setShowSlashMenu(false);
        setShowAtMenu(false);
        setMacrosVisible(false);
        setSearchOpen(false);
        if (stagedAttachments.length > 0) {
          setStagedAttachments([]);
        } else {
          inputRef.current?.blur();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isThinking, stagedAttachments.length]);

  // ── 斜杠 / 宏命令 与 @ 特种兵 弹窗状态 ──────────────────────
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [showAtMenu, setShowAtMenu] = useState(false);

  // ── 导入 JSON 会话（dialogOpen 为模块级，逻辑留在父组件） ────────
  const handleImportJson = async () => {
    if (dialogOpen) {
      const selected = await dialogOpen({
        multiple: false,
        filters: [{ name: "JSON 会话档案", extensions: ["json"] }],
      });
      if (selected && !Array.isArray(selected)) {
        try {
          const resp = await fetch(`file://${selected}`);
          const jsonStr = await resp.text();
          await importChatSession(jsonStr);
          refreshManifests();
          toast.showToast("success", "IMPORTED", "会话已从 JSON 文件导入。");
        } catch {
          toast.showToast("error", "IMPORT FAILED", "无法读取文件 — 请确认选择的是 .json 会话档案。");
        }
      }
    } else {
      toast.showToast("info", "BROWSER MODE", "导入功能需 Tauri 桌面环境。");
    }
  };

  // ── 清空所有会话 ──────────────────────────────────────────
  const handleClearAll = async () => {
    if (
      !confirm(
        `确定删除全部 ${manifests.length} 个会话档案？\n此操作不可撤销，所有对话记录将被永久移除。`,
      )
    )
      return;
    for (const m of manifests) {
      try {
        await deleteChatSession(m.session_id);
      } catch {
        /* continue */
      }
    }
    refreshManifests();
    handleNewSession();
    toast.showToast("success", "ALL CLEARED", "全部会话档案已清空。");
  };

  // ── 消息搜索 (Ctrl+F) ──────────────────────────────────────
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchMatches, setSearchMatches] = useState<number[]>([]);
  const [currentMatchIdx, setCurrentMatchIdx] = useState(0);

  // ── 消息搜索逻辑 ──────────────────────────────────────────
  const handleSearch = (query: string) => {
    setSearchQuery(query);
    if (!query.trim()) {
      setSearchMatches([]);
      return;
    }
    const indices: number[] = [];
    messages.forEach((msg, i) => {
      if (msg.content.toLowerCase().includes(query.toLowerCase())) {
        indices.push(i);
      }
    });
    setSearchMatches(indices);
    setCurrentMatchIdx(0);
    // 滚动到第一个匹配
    if (indices.length > 0) {
      const el = document.getElementById(
        `msg-${messages[indices[0]]?.id}`,
      );
      el?.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  };

  const navSearch = (dir: 1 | -1) => {
    if (searchMatches.length === 0) return;
    const next =
      (currentMatchIdx + dir + searchMatches.length) %
      searchMatches.length;
    setCurrentMatchIdx(next);
    const el = document.getElementById(
      `msg-${messages[searchMatches[next]]?.id}`,
    );
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
  };


  // ── 快捷斜杠宏指令白名单 ───────────────────────────────────
  const slashCommands = [
    {
      cmd: "/rewind",
      desc: "⏳ 逆转时空：一键触发系统级状态秒级双回滚",
      icon: "⏳",
    },
    {
      cmd: "/compact",
      desc: "✂️ 压缩上下文：蒸馏冗余历史日志，清空废 Token 占用",
      icon: "✂️",
    },
    {
      cmd: "/snapshot",
      desc: "📸 磁盘原子锁定：调用 Windows VSS 建立当前物理快照",
      icon: "📸",
    },
    {
      cmd: "/clean",
      desc: "🧹 清屏重置：擦除当前黑板，保留会话元数据",
      icon: "🧹",
    },
  ];

  // ── 专业子智能体特种兵集群 ─────────────────────────────────
  const subAgents = [
    {
      name: "@Explore",
      desc: "🦀 源码检索：专职抓取和分析本地/远程代码拓扑树",
      color: "text-emerald-400",
    },
    {
      name: "@Auditor",
      desc: "🛡️ 安全审计：增量静态 AST 白盒走查，拦截 Secrets 与 GPL 污染",
      color: "text-amber-400",
    },
    {
      name: "@Scout",
      desc: "🌐 多模态探路者：扫描外部 Web 文档与非标系统界面执行纠偏",
      color: "text-purple-400",
    },
    {
      name: "@Compaction",
      desc: "🥷 摘要刺客：端侧隐密激活，压缩上下文以极致降低云端资费",
      color: "text-pink-400",
    },
  ];

  // ── 输入框实时侦测 / 与 @ 触发字 ────────────────────────────
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setInput(val);

    // 格式防幻觉前置探测：判断最后一个字符
    const lastChar = val.charAt(val.length - 1);
    if (lastChar === "/") {
      setShowSlashMenu(true);
      setShowAtMenu(false);
    } else if (lastChar === "@") {
      setShowAtMenu(true);
      setShowSlashMenu(false);
    } else if (val === "" || lastChar === " ") {
      setShowSlashMenu(false);
      setShowAtMenu(false);
    }
  };

  // ── 选择快捷菜单指令后的闭环合并处理 ───────────────────────
  const selectCommand = (command: string, menuType: "slash" | "at") => {
    if (menuType === "slash") {
      // 特殊指令：/clean 直接清屏而非插入文本
      if (command === "/clean") {
        setMessages([
          {
            id: "msg-0",
            sender: "System",
            model: "Local System",
            content: "🧹 黑板已擦除。会话元数据与分块档案完整保留。",
            timestamp: new Date().toLocaleTimeString(),
          },
        ]);
        setInput("");
        setShowSlashMenu(false);
        toast.showToast(
          "info",
          "BOARD CLEANED",
          "黑板已擦除，会话元数据与分块档案完整保留。",
        );
        return;
      }
      setInput(command + " ");
      setShowSlashMenu(false);
      toast.showToast(
        "info",
        "MACRO COMMAND",
        `已挂载快捷指令: ${command}`,
      );
    } else {
      setInput(command + " ");
      setShowAtMenu(false);
      toast.showToast(
        "success",
        "SUBAGENT TARGETING",
        `已精准锁定特种随航体: ${command}`,
      );
    }
  };

  // ── 持久化：分块 Commit 到 Chronos Vault ─────────────────────
  const persistCurrentSession = useCallback(
    (msgs: Message[]) => {
      const accumulatedCost = msgs.reduce(
        (acc, m) => acc + (m.costTokens ?? 0) * 0.000001,
        0,
      );

      const payload = {
        meta: {
          session_id: activeSessionId,
          title: deriveTitle(msgs),
          bound_project: currentProject,
          last_updated: new Date().toISOString(),
          total_messages_count: msgs.length,
          total_accumulated_cost: accumulatedCost,
        },
        messages: msgs.map((m) => ({
          id: m.id,
          sender: m.sender,
          model: m.model,
          content: m.content,
          thinking: m.thinking ?? null,
          cost_tokens: m.costTokens ?? 0,
          timestamp: m.timestamp,
          caching_marker_hash: "", // Rust 端全自动计算，此处占位
        })),
      };
      saveChatSessionChunk(payload).catch(() => {});
    },
    [activeSessionId, currentProject],
  );

  // ── 切换历史航道：从物理磁盘反序列化分块消息体 ──────────────
  const handleSwitchSession = (sessionId: string) => {
    setActiveSessionId(sessionId);
    loadChatSessionChunk(sessionId)
      .then((payload) => {
        const restored: Message[] = payload.messages.map((m) => ({
          id: m.id,
          sender: m.sender as Message["sender"],
          model: m.model,
          content: m.content,
          thinking: m.thinking ?? undefined,
          costTokens: m.cost_tokens,
          isCached: true,
          timestamp: m.timestamp,
          cachingMarkerHash: m.caching_marker_hash,
        }));
        setMessages(restored);
        toast.showToast(
          "success",
          "CHRONOS VFS ALIGNED",
          `成功回溯历史航道。缓存哈希特征点 [${payload.meta.total_messages_count}] 锁定。`,
        );
      })
      .catch((err) =>
        toast.showToast(
          "error",
          "LOAD INTERRUPTED",
          `时空分块加载阻断: ${err}`,
        ),
      );
  };

  // ── 新建空白会话 ─────────────────────────────────────────────
  const handleNewSession = () => {
    const newId = `sess-${Date.now()}`;
    setActiveSessionId(newId);
    setMessages([
      {
        id: "msg-0",
        sender: "System",
        model: "Local System",
        content: "新智能研发航道已开启。",
        timestamp: new Date().toLocaleTimeString(),
      },
    ]);
    toast.showToast("info", "NEW LINE", "空白研发航道已就位。");
  };

  // ── 导出当前会话为 JSON 文件 ──────────────────────────────
  const handleExportSession = async () => {
    try {
      const jsonStr = await exportChatSession(activeSessionId);
      // 尝试通过 save dialog 保存（Tauri 环境）
      if (dialogOpen) {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const filePath = await save({
          defaultPath: `chronos-session-${activeSessionId}.json`,
          filters: [{ name: "JSON", extensions: ["json"] }],
        });
        if (filePath) {
          // 使用前端 Blob 下载（跨平台兼容）
          const blob = new Blob([jsonStr], {
            type: "application/json",
          });
          const url = URL.createObjectURL(blob);
          const a = document.createElement("a");
          a.href = url;
          a.download = filePath.split(/[/\\]/).pop() ?? filePath;
          a.click();
          URL.revokeObjectURL(url);
          toast.showToast(
            "success",
            "EXPORTED",
            `会话 JSON 已保存。`,
          );
        }
      } else {
        // 浏览器降级：直接下载
        const blob = new Blob([jsonStr], { type: "application/json" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `chronos-session-${activeSessionId}.json`;
        a.click();
        URL.revokeObjectURL(url);
        toast.showToast("success", "EXPORTED", "会话 JSON 已下载。");
      }
    } catch (err) {
      toast.showToast("error", "EXPORT FAILED", `导出失败: ${err}`);
    }
  };

  // ── 手动固化当前会话（分块 Commit）───────────────────────────
  const handlePersistSession = () => {
    setIsSaving(true);
    persistCurrentSession(messages);
    // 仿真一段微小延迟以展示按钮状态
    setTimeout(() => {
      setIsSaving(false);
      toast.showToast(
        "success",
        "CHUNK COMMIT SUCCESS",
        "💾 时空分块与缓存特征点已安全写入物理磁盘档案库。",
      );
      refreshManifests();
    }, 300);
  };

  // ── 发送消息 ─────────────────────────────────────────────────
  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if ((!input.trim() && stagedAttachments.length === 0) || isThinking)
      return;
    const userText = input.trim();
    setInput("");
    setLastUserInput(userText);
    setRetryCount(0);
    // 关闭所有弹窗
    setShowSlashMenu(false);
    setShowAtMenu(false);

    // 检测 @ 特种兵唤醒
    const targetedAgent = subAgents.find((sa) =>
      userText.startsWith(sa.name),
    );

    // 检测 / 宏命令
    const triggeredMacro = slashCommands.find((sc) =>
      userText.startsWith(sc.cmd),
    );

    // 映射特种兵名称到 sender 角色
    const agentSenderMap: Record<string, Message["sender"]> = {
      "@Explore": "Explore",
      "@Auditor": "Auditor",
      "@Scout": "Scout",
      "@Compaction": "Compaction",
    };

    const userMsg: Message = {
      id: `user-${Date.now()}`,
      sender: targetedAgent
        ? (agentSenderMap[targetedAgent.name] ?? "User")
        : "User",
      model: targetedAgent
        ? `${targetedAgent.name.slice(1)} (特种兵)`
        : triggeredMacro
          ? `Macro: ${triggeredMacro.cmd}`
          : "Human Operator",
      content: userText,
      attachments:
        stagedAttachments.length > 0 ? [...stagedAttachments] : undefined,
      timestamp: new Date().toLocaleTimeString(),
    };
    // 写入共享黑板后清空临时缓冲区
    setStagedAttachments([]);
    const updatedAfterUser = [...messages, userMsg];
    setMessages(updatedAfterUser);
    setIsThinking(true);
    setFlowStage("connecting");
    setFlowStartMs(Date.now());
    // 用户发送消息时强制滚到底部
    isNearBottomRef.current = true;

    // ── 真实 API 调用 — key resolved server-side from vault ──
    // Always attempt API call; Rust backend resolves key from Windows Credential Manager
    try {
      const chatMessages = messages
        .filter(
          (m) =>
            m.sender === "User" ||
            m.sender === "Coder" ||
            m.sender === "PM",
        )
        .map((m) => ({
          role: m.sender === "User" ? "user" : "assistant",
          content: m.content,
        }));
      chatMessages.push({ role: "user", content: userText });

      const endpoint = await getModelEndpoint(selectedModel);

      setFlowStage("thinking");
      // ── 流式调用：先插入占位消息，逐 chunk 更新 ──────────
      const streamMsgId = `stream-${Date.now()}`;
      const streamPlaceholder: Message = {
        id: streamMsgId,
        sender: "Coder",
        model: `${modelDisplayName(selectedModel)} (Stream)`,
        content: "",
        costTokens: 0,
        isCached: false,
        timestamp: new Date().toLocaleTimeString(),
      };
      const initialMsgs = [...updatedAfterUser, streamPlaceholder];
      setMessages(initialMsgs);
      let streamedContent = "";

      // 监听流式 chunk 事件
      const unlisten = await onChatStreamChunk((chunk) => {
        if (flowStage !== "streaming") setFlowStage("streaming");
        streamedContent += chunk;
        setMessages((prev) =>
          prev.map((m) =>
            m.id === streamMsgId
              ? { ...m, content: streamedContent }
              : m,
          ),
        );
      });

      // 发起流式请求 — finally 确保监听器一定被清理
      let response;
      try {
        response = await chatApiStream(
          endpoint,
          apiKey,
          selectedModel,
          chatMessages,
          4096,
        );
      } finally {
        unlisten();
      }

      if (response.success) {
        // Replace stream placeholder with final message
        setMessages((prev) =>
          prev.map((m) =>
            m.id === streamMsgId
              ? {
                  ...m,
                  content: response.content || streamedContent,
                  costTokens: response.tokens_used,
                  isCached: response.cached,
                  model: `${modelDisplayName(selectedModel)} (API)`,
                }
              : m,
          ),
        );

        // ── 行动调度引擎：检测 LLM 响应中的动作指令 ──
        const finalContent = response.content || streamedContent;
        let allMessages = [...updatedAfterUser, {
          ...streamPlaceholder,
          content: finalContent,
          costTokens: response.tokens_used,
          isCached: response.cached,
          model: `${modelDisplayName(selectedModel)} (API)`,
        }];

        // 扫描并执行动作 + 自动保存代码块 (始终执行, 不再依赖 "action" 关键词)
        {
          try {
            const execResult = await extractAndExecuteActions(finalContent);
            if (execResult.has_actions) {
              // Build action summary message
              const filesCreated = (execResult as any).files_created as string[] | undefined;
              const filesSummary = (execResult as any).files_summary as string | undefined;
              const actionResults = execResult.action_results || [];
              const actionCount = actionResults.filter((a: any) => a.success).length;
              const failCount = actionResults.filter((a: any) => !a.success).length;

              let summaryText = '';
              if (filesCreated && filesCreated.length > 0) {
                // 登记成品文件
                const newArtifacts = filesCreated.map(f => ({
                  path: f, type: f.split('.').pop() || 'file',
                  createdAt: new Date().toLocaleTimeString(), versions: 1,
                }));
                setArtifacts(prev => [...prev, ...newArtifacts]);
                summaryText = `📁 **文件已生成** (${filesCreated.length} files)\n\n${filesSummary || filesCreated.map((f: string) => `✅ ${f}`).join('\n')}`;
              } else if (execResult.combined_context) {
                // 显示动作执行结果 (搜索/抓取/环境检测等)
                summaryText = execResult.combined_context.slice(0, 2000);
              } else if (failCount > 0) {
                // 动作失败: 显示错误信息
                const errors = actionResults.filter((a: any) => !a.success)
                  .map((a: any, i: number) => `❌ ${i + 1}. ${(a.error || '未知错误')}`)
                  .join('\n');
                summaryText = `⚠️ **动作执行失败** (${failCount} 个)\n\n${errors}`;
              } else {
                summaryText = `⚡ **已执行 ${actionCount} 个操作**`;
              }

              const sysMsg: Message = {
                id: `sys-${Date.now()}`,
                sender: "System",
                model: "Action Engine",
                content: summaryText,
                timestamp: new Date().toLocaleTimeString(),
              };
              allMessages = [...allMessages, sysMsg];

              // Auto-continue: feed results back to LLM for a synthesized answer
              setMessages(allMessages);
              setIsThinking(true);
              setFlowStage("researching");

              const followUpMessages = [
                ...chatMessages,
                { role: "assistant", content: finalContent },
                { role: "user", content: `Based on the following research results, please synthesize a comprehensive answer. Cite sources.\n\n${execResult.combined_context}` },
              ];

              // Stream listener for follow-up
              let followUpContent = "";
              const fuUnlisten = await onChatStreamChunk((chunk) => {
                followUpContent += chunk;
                setMessages((prev) => {
                  const last = prev[prev.length - 1];
                  if (last?.id.startsWith("followup-")) {
                    setFlowStage("streaming");
                    return prev.map((m) => m.id === last.id ? { ...m, content: followUpContent } : m);
                  }
                  return prev;
                });
              });

              const followUpPlaceholderId = `followup-${Date.now()}`;
              setMessages((prev) => [...prev, {
                id: followUpPlaceholderId, sender: "Coder" as const,
                model: `${modelDisplayName(selectedModel)} (Research)`,
                content: "", costTokens: 0, isCached: false,
                timestamp: new Date().toLocaleTimeString(),
              }]);

              let followUp;
              try { followUp = await chatApiStream(endpoint, apiKey, selectedModel, followUpMessages, 4096); }
              finally { fuUnlisten(); }

              if (followUp.success) {
                const fuFinal = followUp.content || followUpContent;
                // 🔬 follow-up 结果也可能包含新动作 (如 web_search) — 递归执行
                let fuProcessed = fuFinal;
                try {
                  const fuExec = await extractAndExecuteActions(fuFinal);
                  if (fuExec.has_actions && fuExec.combined_context) {
                    fuProcessed = fuExec.combined_context.slice(0, 2000);
                    const fuFiles = (fuExec as any).files_created as string[] | undefined;
                    if (fuFiles && fuFiles.length > 0) {
                      const newArtifacts = fuFiles.map(f => ({
                        path: f, type: f.split('.').pop() || 'file',
                        createdAt: new Date().toLocaleTimeString(), versions: 1,
                      }));
                      setArtifacts(prev => [...prev, ...newArtifacts]);
                    }
                  }
                } catch { /* follow-up action failed, keep raw */ }
                setMessages((prev) => prev.map((m) =>
                  m.id === followUpPlaceholderId ? { ...m, content: fuProcessed, costTokens: followUp.tokens_used, isCached: followUp.cached } : m
                ));
                allMessages.push({
                  id: followUpPlaceholderId, sender: "Coder" as const,
                  model: `${modelDisplayName(selectedModel)} (Research)`,
                  content: fuProcessed, costTokens: followUp.tokens_used, isCached: followUp.cached,
                  timestamp: new Date().toLocaleTimeString(),
                });
                toast.showToast("success", "RESEARCH COMPLETE", "已自动搜索并整合信息到回复中。");
              } else {
                setMessages((prev) => prev.filter((m) => m.id !== followUpPlaceholderId));
              }
            }
          } catch (e) {
            // Action execution failed silently — response is still valid
            console.warn("[ChatPanel] Action dispatch failed:", e);
          }
        }

        setMessages(allMessages);

        // 自动分块持久化
        persistCurrentSession(allMessages);
        refreshManifests();
        if (response.cached) {
          toast.showToast(
            "success",
            "CACHE HIT",
            `DeepSeek 一折缓存命中，节省 ${response.tokens_used ?? 0} tokens。`,
          );
        }
      } else {
        setMessages((prev) =>
          prev.filter((m) => m.id !== streamMsgId),
        );
        const errMsg: Message = {
          id: `err-${Date.now()}`,
          sender: "System",
          model: "Error",
          content: `${t.chat_error_api}: ${response.error ?? "unknown"}\n请检查 API Key 和网络连接。`,
          timestamp: new Date().toLocaleTimeString(),
        };
        const finalMsgs = [...updatedAfterUser, errMsg];
        setMessages(finalMsgs);
        persistCurrentSession(finalMsgs);
        toast.showToast(
          "error",
          "API ERROR",
          response.error ?? "未知错误 — 请检查 API Key 和端点地址。",
        );
      }
    } catch (err) {
      const errMsg: Message = {
        id: `err-${Date.now()}`,
        sender: "System",
        model: "Error",
        content: `${t.chat_error_network}: ${err instanceof Error ? err.message : String(err)}`,
        timestamp: new Date().toLocaleTimeString(),
      };
      const finalMsgs = [...updatedAfterUser, errMsg];
      setMessages(finalMsgs);
      persistCurrentSession(finalMsgs);
      toast.showToast(
        "error",
        "NETWORK ERROR",
        "网络请求失败，请检查连接或切换至 LAN 离线模式。",
      );
    }

    setIsThinking(false);
    setFlowStage("idle");
    // 自动聚焦输入框
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  // ── 附件挂载（dialogOpen 为模块级，逻辑留在父组件） ───────────
  const handleAttachDoc = async () => {
    if (dialogOpen) {
      const selected = await dialogOpen({
        multiple: true,
        filters: [{ name: "文档", extensions: ["md", "pdf", "txt", "xlsx", "csv", "json"] }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        const newAttachments: Attachment[] = paths.map((p) => {
          const name = p.split(/[\\]/).pop() ?? p;
          return { type: "doc" as const, name, sizeOrPath: p };
        });
        setStagedAttachments((prev) => [...prev, ...newAttachments]);
      }
    } else {
      // 浏览器降级 mock
      setStagedAttachments((prev) => [...prev, { type: "doc", name: "PRD_Requirements.pdf", sizeOrPath: "124 KB" }]);
    }
  };

  const handleAttachImage = async () => {
    if (dialogOpen) {
      const selected = await dialogOpen({
        multiple: true,
        filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        const newAttachments: Attachment[] = paths.map((p) => {
          const name = p.split(/[\\]/).pop() ?? p;
          return { type: "image" as const, name, sizeOrPath: p };
        });
        setStagedAttachments((prev) => [...prev, ...newAttachments]);
      }
    } else {
      // 浏览器降级 mock
      setStagedAttachments((prev) => [...prev, { type: "image", name: "ERP_Error_Snapshot.png", sizeOrPath: "1080P" }]);
    }
  };

  // ── 拖拽上传：Tauri 原生 drag-drop → 多模态挂载缓冲区 ────────
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const paths = event.payload.paths ?? [];
      const docs: Attachment[] = [];
      const imgs: Attachment[] = [];
      for (const p of paths) {
        const name = p.split(/[\\/]/).pop() ?? p;
        if (/\.(png|jpe?g|gif|webp)$/i.test(p)) imgs.push({ type: "image", name, sizeOrPath: p });
        else docs.push({ type: "doc", name, sizeOrPath: p });
      }
      setStagedAttachments((prev) => [...prev, ...docs, ...imgs]);
      shortcutsRef.current.toast.showToast("success", "ATTACHED", `${docs.length + imgs.length} file(s) staged from drag & drop.`);
    }).then((fn) => { unlisten = fn; }).catch(() => {});
    return () => unlisten?.();
  }, []);

  // ── 重试上次消息 ──────────────────────────────────────────
  // 请求取消当前流式对话 — 后端流循环检查取消标志并提前返回，
  // handleSend 尾部的 setIsThinking(false) 随之自然复位
  const handleCancelStream = () => {
    cancelChatStream().catch(() => {});
    toast.showToast("info", "STREAM CANCEL", t.stream_stop);
  };

  const handleRetry = () => {
    if (!lastUserInput || retryCount >= 2) return;
    setRetryCount((c) => c + 1);
    // 移除最后的错误消息
    setMessages((prev) => {
      const last = prev[prev.length - 1];
      if (last?.sender === "System" && last?.model === "Error") {
        return prev.slice(0, -1);
      }
      return prev;
    });
    // 重新设置输入并触发发送
    setInput(lastUserInput);
    setTimeout(() => {
      const form = document.querySelector('form[data-chat-form]') as HTMLFormElement;
      form?.requestSubmit();
    }, 100);
  };

  // ── 复制消息内容到剪贴板 ───────────────────────────────────
  const handleCopyMessage = async (content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      toast.showToast("success", "COPIED", "已复制到系统剪贴板。");
    } catch {
      toast.showToast("error", "COPY FAILED", "剪贴板写入失败。");
    }
  };

  useEffect(() => {
    // 仅当用户在底部时才自动滚动（防止打断历史消息阅读）
    if (isNearBottomRef.current) {
      chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, isThinking]);

  // ── 窗口关闭前自动保存 ────────────────────────────────────
  useEffect(() => {
    const handleBeforeUnload = () => {
      if (messages.length > 1) {
        persistCurrentSession(messages);
      }
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [messages, persistCurrentSession]);

  // ── 快捷键/命令 handler 的稳定引用（避免每渲染重绑监听器 + 防 stale closure）──
  const shortcutsRef = useRef({
    handleNewSession, persistCurrentSession, toast, refreshManifests,
    handleExportSession, handlePersistSession, handleClearAll,
  });
  shortcutsRef.current = {
    handleNewSession, persistCurrentSession, toast, refreshManifests,
    handleExportSession, handlePersistSession, handleClearAll,
  };

  // ── 全局命令面板事件 (App.tsx Ctrl+K → CommandPalette) ─────
  useEffect(() => {
    const handleCommand = (e: Event) => {
      const cmd = (e as CustomEvent<string>).detail;
      const h = shortcutsRef.current;
      switch (cmd) {
        case "new-session": h.handleNewSession(); break;
        case "save-session": h.handlePersistSession(); break;
        case "export-session": h.handleExportSession(); break;
        case "clear-all": h.handleClearAll(); break;
        case "toggle-sidebar": setSidebarCollapsed(v => !v); break;
        case "focus-input": inputRef.current?.focus(); break;
        default: break;
      }
    };
    window.addEventListener("chronos:command", handleCommand);
    return () => window.removeEventListener("chronos:command", handleCommand);
  }, []);

  const getSenderStyle = (sender: string) => {
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
  };

  return (
    <div className="flex h-full bg-cs-bg font-mono text-xs text-cs-text overflow-hidden select-none">
      {/* ═══ 左侧栏：会话历史 (可折叠) ═══ */}
      {!sidebarCollapsed && (
        <SessionSidebar
          manifests={manifests}
          activeSessionId={activeSessionId}
          currentProject={currentProject}
          onSwitchSession={handleSwitchSession}
          onNewSession={handleNewSession}
          onClearAll={handleClearAll}
          onImport={handleImportJson}
          refreshManifests={refreshManifests}
        />
      )}

      {/* ═══ 右侧主栏：沉浸式对话 ═══ */}
      <div className="flex-1 flex flex-col min-w-0 bg-cs-bg relative h-full">
        {/* Header */}
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-cs-border bg-cs-surface shrink-0">
          <div className="flex items-center space-x-2 text-xs">
            {/* 侧栏切换 */}
            <button
              onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
              className="p-1 rounded hover:bg-zinc-800/50 text-zinc-500 hover:text-zinc-300 transition-colors"
              title={sidebarCollapsed ? "展开历史会话" : "收起历史会话"}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M3 12h18M3 6h18M3 18h18"/>
              </svg>
            </button>
            <span
              className={`inline-block w-2 h-2 rounded-full ${apiKey ? "bg-emerald-500 animate-ping" : "bg-amber-500"}`}
            />
            <span className="font-bold text-zinc-300 uppercase text-[10px] tracking-wider">
              {t.omni_chat}
            </span>
            {/* Tauri 连通性指示器 */}
            <span
              className={`text-[10px] px-1 rounded ${
                typeof window !== "undefined" &&
                "__TAURI_INTERNALS__" in window
                  ? "bg-emerald-950/30 text-emerald-500 border border-emerald-500/30"
                  : "bg-amber-950/30 text-amber-500 border border-amber-500/30"
              }`}
              title={
                typeof window !== "undefined" &&
                "__TAURI_INTERNALS__" in window
                  ? "已连接 Rust 引擎"
                  : "浏览器演示模式 — IPC 不可用"
              }
            >
              {typeof window !== "undefined" &&
              "__TAURI_INTERNALS__" in window
                ? <><Link2 size={9} className="inline text-emerald-400 -mt-0.5" aria-hidden="true" /> 在线</>
                : <><Zap size={9} className="inline text-amber-400 -mt-0.5" aria-hidden="true" /> 演示</>}
            </span>
            <span className="text-[10px] text-zinc-500">
              |{" "}
              {isThinking
                ? `${stageLabel}${flowDots} ${flowElapsed}s`
                : currentHasKey
                  ? modelDisplayName(selectedModel)
                  : anyKey
                    ? `${t.agent_listening.replace("...", "")} — ${availableProvider} 可用`
                    : t.agent_listening}
            </span>
            {isThinking ? (
              <span className="text-[10px] text-cyan-400 animate-pulse">● 心流激活</span>
            ) : !currentHasKey && (
              <span className={anyKey ? "text-[10px] text-cyan-400" : "text-[10px] text-amber-500"}>
                {anyKey ? `(切换至 ${availableProvider})` : "(Demo)"}
              </span>
            )}
            {/* 项目切换下拉框 */}
            <select
              value={currentProject || ""}
              onChange={e => { if (e.target.value) { onProjectChange?.(e.target.value); cvfsListProjectFiles(e.target.value).then(setProjectFiles).catch(()=>{}); } }}
              onFocus={() => { cvfsGetProjects().then(p => { if (p.length) setProjectList(p); }).catch(()=>{}); }}
              className="text-[10px] bg-cs-header border border-cs-border rounded px-1.5 py-0.5 text-zinc-300 outline-none cursor-pointer max-w-[140px]"
              title="切换项目"
            >
              <option value="">📁 选择项目</option>
              {projectList.length > 0 ? projectList.map(p => (
                <option key={p.id} value={p.name}>{p.name}</option>
              )) : (currentProject && <option value={currentProject}>{currentProject}</option>)}
            </select>
            {/* 文件浏览器切换 */}
            {currentProject && currentProject !== "default" && (
              <button
                onClick={() => { setShowFileExplorer(!showFileExplorer); if (!showFileExplorer) cvfsListProjectFiles(currentProject).then(setProjectFiles).catch(()=>{}); }}
                className={`text-[10px] border px-1 py-0.5 rounded transition-colors ${
                  showFileExplorer
                    ? "text-cyan-300 border-cyan-400/40 bg-cyan-950/30"
                    : "text-zinc-500 border-cs-border hover:border-zinc-600"
                }`}
                title="项目文件"
              >
                📂{projectFiles.length}
              </button>
            )}
          </div>
          <div className="flex items-center space-x-1.5">
            <button
              onClick={handleExportSession}
              className="text-[10px] bg-zinc-800/50 hover:bg-zinc-700 border border-zinc-700/50 text-zinc-400 hover:text-zinc-200 px-2 py-0.5 rounded transition-all"
              title="导出为 JSON 文件"
            >
              <Upload size={10} className="inline mr-0.5 -mt-0.5" aria-hidden="true" />
              导出
            </button>
            <button
              onClick={handlePersistSession}
              disabled={isSaving}
              className="text-[10px] bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 text-zinc-200 px-2 py-0.5 rounded font-bold transition-all disabled:opacity-40"
            >
              <Save size={10} className="inline mr-0.5 -mt-0.5" aria-hidden="true" />
              {isSaving ? "正在执行物理分块..." : "固化当前会话分块"}
            </button>
          </div>
        </div>

        {/* 消息搜索栏 (Ctrl+F) */}
        {searchOpen && (
          <div className="flex items-center space-x-2 px-4 py-1.5 border-b border-cs-border bg-cs-header shrink-0 animate-fadeIn">
            <span className="text-zinc-500" aria-hidden="true"><Search size={10} /></span>
            <input
              data-search-input
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") navSearch(e.shiftKey ? -1 : 1);
                if (e.key === "Escape") setSearchOpen(false);
              }}
              placeholder="搜索消息…"
              className="flex-1 bg-transparent text-xs text-zinc-200 placeholder-zinc-600 outline-none"
            />
            {searchMatches.length > 0 && (
              <span className="text-[10px] text-zinc-500">
                {currentMatchIdx + 1}/{searchMatches.length}
              </span>
            )}
            <button
              onClick={() => navSearch(-1)}
              className="text-[10px] text-zinc-500 hover:text-zinc-300 px-1"
            >
              ▲
            </button>
            <button
              onClick={() => navSearch(1)}
              className="text-[10px] text-zinc-500 hover:text-zinc-300 px-1"
            >
              ▼
            </button>
            <button
              onClick={() => setSearchOpen(false)}
              className="text-[10px] text-zinc-600 hover:text-zinc-400 px-1"
            >
              ✕
            </button>
          </div>
        )}

        {/* ── 消息区 + 文件面板 ── */}
        <div className="flex-1 flex overflow-hidden">
        {/* Messages */}
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
                searchMatches.includes(
                  messages.findIndex((m) => m.id === msg.id),
                )
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
                  <span className="text-zinc-600">
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
                {/* 🔥 显式呈现特征哈希对齐标记，赋予极客绝对的高能效掌控爽感 */}
                {msg.cachingMarkerHash && (
                  <span className="text-emerald-500 font-bold border border-emerald-950 bg-emerald-950/20 px-1 rounded scale-90 select-none">
                    [Cache-Aligned]
                  </span>
                )}
                {msg.cachingMarkerHash && (
                  <span className="hidden group-hover:inline text-[10px] text-zinc-600 font-light">
                    Hash: {msg.cachingMarkerHash.substring(0, 6)}
                  </span>
                )}
                <span className="text-[10px] text-zinc-600">
                  {msg.timestamp}
                </span>
              </div>
              <div
                className={`border p-3 rounded-lg text-xs leading-relaxed tracking-wide shadow-sm max-w-full relative ${getSenderStyle(msg.sender)}`}
              >
                {/* 复制按钮 */}
                <button
                  onClick={() => handleCopyMessage(msg.content)}
                  className="absolute top-1 right-1 w-5 h-5 flex items-center justify-center rounded text-[10px] text-zinc-600 hover:text-zinc-300 hover:bg-zinc-800/50 opacity-0 group-hover:opacity-100 transition-all"
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
                        <span className="text-[10px] text-zinc-600">
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
                    onClick={handleRetry}
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

        {/* 文件浏览器 (右侧面板) */}
        {showFileExplorer && currentProject !== "default" && (
          <div className="w-48 border-l border-cs-border bg-cs-surface flex flex-col shrink-0 overflow-y-auto">
            <div className="px-2 py-1.5 border-b border-cs-border text-[10px] text-zinc-500 flex items-center justify-between">
              <span className="flex items-center gap-0.5"><FolderOpen size={9} aria-hidden="true" />{currentProject}</span>
              <button onClick={() => setShowFileExplorer(false)} className="text-zinc-600 hover:text-zinc-400">✕</button>
            </div>
            <div className="p-1 space-y-0.5">
              {projectFiles.map((f) => {
                const isContract = (f as any).is_locked || f.name === 'CLAUDE.md';
                const isModified = (f as any).is_modified || false;
                return (
                <div key={f.relative_path}
                  className={`flex items-center space-x-1 px-1 py-0.5 rounded text-[10px] cursor-default transition-colors ${
                    isModified ? 'bg-emerald-950/20 border border-emerald-900/30 animate-pulse' : 'hover:bg-zinc-800/40'
                  }`}>
                  <span className="shrink-0">{f.is_dir ? '📁' : '📄'}</span>
                  <span className={`truncate ${
                    isContract ? 'text-amber-400 font-bold' :
                    isModified ? 'text-emerald-400 font-bold' :
                    f.is_dir ? 'text-cyan-400/70' : 'text-zinc-400'
                  }`} title={f.relative_path}>{f.name}</span>
                  {isContract && <span className="text-[10px] text-amber-500 border border-amber-500/30 bg-amber-950/20 px-0.5 rounded shrink-0">🔒</span>}
                  {isModified && <span className="text-[10px] text-emerald-400 shrink-0">●</span>}
                </div>
                );
              })}
              {projectFiles.length === 0 && (
                <div className="text-[10px] text-zinc-600 text-center py-2">
                  空项目 — AI创建文件后出现
                </div>
              )}
            </div>
          </div>
        )}
        </div>{/* end messages+files flex row */}

        {/* Input（拆分至 chat/Composer） */}
        <Composer
          input={input}
          setInput={setInput}
          onInputChange={handleInputChange}
          isThinking={isThinking}
          stagedAttachments={stagedAttachments}
          setStagedAttachments={setStagedAttachments}
          showSlashMenu={showSlashMenu}
          showAtMenu={showAtMenu}
          macrosVisible={macrosVisible}
          setMacrosVisible={setMacrosVisible}
          slashCommands={slashCommands}
          subAgents={subAgents}
          selectCommand={selectCommand}
          onSend={handleSend}
          onCancelStream={handleCancelStream}
          onAttachDoc={handleAttachDoc}
          onAttachImage={handleAttachImage}
          apiKey={apiKey}
          inputRef={inputRef}
        />

          {/* 成品文件面板 + 编辑模态（拆分至 chat/ArtifactPanel） */}
          <ArtifactPanel
            artifacts={artifacts}
            onClear={() => setArtifacts([])}
            onEditRequest={(path) => { setEditingFile(path); setFileContent(""); }}
            editingFile={editingFile}
            setEditingFile={setEditingFile}
            fileContent={fileContent}
            setFileContent={setFileContent}
            onEditWithAI={(path, content) => {
              setInput(`请修改以下文件内容，并返回完整修改后的文件:

文件: ${path}

${content}`);
              setEditingFile(null);
              inputRef.current?.focus();
            }}
            currentProject={currentProject}
          />

          {/* 状态栏：会话统计 + 审批指示 */}
          <div className="flex items-center justify-between px-4 py-1 border-t border-[#1a1a1e] bg-cs-surface text-[10px] text-zinc-600 select-none">
            <div className="flex items-center space-x-3">
              <span className="flex items-center gap-0.5"><MessageSquare size={9} aria-hidden="true" />{messages.length} 条</span>
              <span>|</span>
              <span className="flex items-center gap-0.5"><Coins size={9} aria-hidden="true" />¥{messages.reduce((a, m) => a + (m.costTokens ?? 0) * 0.000001, 0).toFixed(4)}</span>
              {currentProject && currentProject !== "default" && (
                <>
                  <span>|</span>
                  <span className="text-cyan-500 flex items-center gap-0.5"><FolderOpen size={9} aria-hidden="true" />{currentProject}</span>
                </>
              )}
            </div>
            {/* 审批门禁状态指示 */}
            <span className="text-red-400" title="审批门禁已激活，请通过左侧 Dock 的 🛡️ 图标访问审批面板">
              🛡️ 第四红线: 审批门禁已激活
            </span>
          </div>

          {/* 键盘快捷提示 + Token 计数器 */}
          <div className="flex items-center justify-between px-4 pb-2 text-[10px] text-zinc-700 select-none">
            <div className="flex items-center space-x-3">
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  Ctrl+Enter
                </kbd>
                发送
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  /
                </kbd>
                宏命令
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  @
                </kbd>
                特种兵
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  Ctrl+N
                </kbd>
                新建
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  Ctrl+S
                </kbd>
                保存
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-cs-header border border-cs-border rounded text-[10px] text-zinc-500 mr-1">
                  Esc
                </kbd>
                关闭
              </span>
            </div>
            <div className="text-zinc-600">
              {input.length > 0 && (
                <>
                  {input.length} 字符 ≈{" "}
                  {Math.max(1, Math.ceil(input.length / 4))} tokens
                </>
              )}
            </div>
          </div>
        </div>
      </div>
  );
}
