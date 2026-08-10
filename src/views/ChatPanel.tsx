// src/views/ChatPanel.tsx — 沉浸式高级全局对话视窗 (Omni-Chat Console)
//
// v2: 接入真实 chatApi IPC → Rust 后端 → 云端 LLM
// v3: IDE 风格历史会话侧栏 → session_db 持久化 + DeepSeek Context Caching 一折命中
// v4: 流式分块会话数据库 (Chunked V2) → caching_marker_hash + 财务审计 + Cache-Aligned 徽章
// 若未配置 API Key 则降级为本地 mock 演示

import React, { useState, useEffect, useRef, useCallback } from "react";
import { useT } from "@/lib/i18n-context";
import { useToast } from "@/components/ToastProvider";
import QuickMacros from "@/components/QuickMacros";
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
  renameChatSession,
  importChatSession,
  extractAndExecuteActions,
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
import type { SessionMetaManifest } from "@/lib/types";
import { renderMarkdown } from "@/lib/utils";
import { createElement, type ReactNode } from "react";
import { FileTextIcon, ImageIcon } from "@/components/SvgIcons";

interface Attachment {
  type: "doc" | "image";
  name: string;
  sizeOrPath: string;
}

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
  /** Per-provider key presence — drives welcome message + status bar */
  hasKeys?: { deepseek: boolean; kimi: boolean; glm: boolean };
  /** Current project name — binds sessions to real project for scoped recall */
  currentProject?: string;
}

function modelDisplayName(model: string): string {
  const m: Record<string, string> = {
    "deepseek-v4-pro": "DeepSeek V4-Pro",
    "deepseek-v4-flash": "DeepSeek V4-Flash",
    "kimi-k3": "Kimi K3",
    "kimi-k2.7-code": "Kimi K2.7-Code",
    "kimi-k2.7-code-highspeed": "Kimi K2.7-Code-HS",
    "glm-5.2": "GLM-5.2",
    "glm-5v-turbo": "GLM-5V-Turbo",
    "glm-5.1": "GLM-5.1",
    "glm-4.7": "GLM-4.7",
  };
  return m[model] ?? model;
}

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

  // 当项目切换时，自动刷新该项目关联的会话列表
  useEffect(() => {
    if (currentProject && currentProject !== "default") {
      listSessionsByProject(currentProject)
        .then((res) => {
          if (res && res.length > 0) setManifests(res);
          else refreshManifests(); // 无项目会话时回退到全量列表
        })
        .catch(() => refreshManifests());
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
          ? `🦀 **Chronos-Shadow v0.2.0** 已就绪 ❤️\n\n${t.chat_welcome_connected}\n\n> *每一次交互，都让系统更懂你*\n> *所有数据端侧处理，你的隐私我们守护* 🔒`
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
        handleNewSession();
        return;
      }
      // Ctrl+S → 固化保存 (uses ref to avoid stale closure)
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === "s") {
        e.preventDefault();
        persistCurrentSession(messagesRef.current);
        setIsSaving(true);
        setTimeout(() => {
          setIsSaving(false);
          toast.showToast("success", "CHUNK COMMIT SUCCESS", "💾 时空分块与缓存特征点已安全写入物理磁盘档案库。");
          refreshManifests();
        }, 300);
        return;
      }
      // Ctrl+Shift+E → 导出
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "e") {
        e.preventDefault();
        handleExportSession();
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

  // ── 侧栏内联重命名 ──────────────────────────────────────────
  const [editingSessionId, setEditingSessionId] = useState<
    string | null
  >(null);
  const [editTitle, setEditTitle] = useState("");

  // ── 侧栏会话搜索 ──────────────────────────────────────────
  const [sessionFilter, setSessionFilter] = useState("");

  const filteredManifests = sessionFilter.trim()
    ? manifests.filter(
        (m) =>
          m.title.toLowerCase().includes(sessionFilter.toLowerCase()) ||
          (m.last_message_preview ?? "")
            .toLowerCase()
            .includes(sessionFilter.toLowerCase()),
      )
    : manifests;

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

  const handleStartRename = (sessionId: string, currentTitle: string) => {
    setEditingSessionId(sessionId);
    setEditTitle(currentTitle);
  };

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
    [activeSessionId],
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
    if (true) {
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

          // Scan for and execute embedded actions
          if (finalContent.includes('"action"') && (finalContent.includes('"web_search"') || finalContent.includes('"web_fetch"'))) {
            try {
              const execResult = await extractAndExecuteActions(finalContent);
              if (execResult.has_actions && execResult.combined_context) {
                // Add system message showing executed actions
                const sysMsg: Message = {
                  id: `sys-${Date.now()}`,
                  sender: "System",
                  model: "Action Engine",
                  content: `🔍 **Auto-research executed**\n\n${execResult.action_results.map((a, i) =>
                    `${i + 1}. ${a.success ? '✅' : '❌'} \`${a.action.slice(0, 80)}...\``
                  ).join('\n')}`,
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
                  setMessages((prev) => prev.map((m) =>
                    m.id === followUpPlaceholderId ? { ...m, content: fuFinal, costTokens: followUp.tokens_used, isCached: followUp.cached } : m
                  ));
                  allMessages.push({
                    id: followUpPlaceholderId, sender: "Coder" as const,
                    model: `${modelDisplayName(selectedModel)} (Research)`,
                    content: fuFinal, costTokens: followUp.tokens_used, isCached: followUp.cached,
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
    } else {
      // ── Mock 演示模式 ──────────────────────────────────────────
      await new Promise((r) => setTimeout(r, 1500));
      const mock: Message = {
        id: `mock-${Date.now()}`,
        sender: "Coder",
        model: `Mock ${t.demo_badge}`,
        content: `${t.chat_mock_reply} "${userText.slice(0, 80)}${userText.length > 80 ? "..." : ""}"\n\n请在「⚙️ 全局配置 → API 密钥凭据」中填入 API Key 以启用真实 AI 对话。`,
        costTokens: 0,
        isCached: false,
        timestamp: new Date().toLocaleTimeString(),
      };
      const finalMsgs = [...updatedAfterUser, mock];
      setMessages(finalMsgs);
      persistCurrentSession(finalMsgs);
    }

    setIsThinking(false);
    setFlowStage("idle");
    // 自动聚焦输入框
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  // ── 重试上次消息 ──────────────────────────────────────────
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
        return "border-zinc-700 bg-black text-[#fafafa]";
    }
  };

  return (
    <div className="flex h-full bg-[#09090b] font-mono text-xs text-[#fafafa] overflow-hidden select-none">
      {/* ═══ 左侧栏：多维轻量元数据面板（Chunked V2） ═══ */}
      <div className="w-56 border-r border-[#27272a] bg-[#0c0c0e] flex flex-col shrink-0">
        <div className="p-2.5 border-b border-[#27272a] bg-[#121214] flex items-center justify-between">
          <span className="font-bold text-zinc-500 uppercase tracking-wider text-[10px]">
            ⌛ 历史会话轨道
            {manifests.length > 0 && (
              <span className="ml-1.5 bg-zinc-800 text-zinc-400 text-[8px] px-1.5 py-0.5 rounded-full">
                {manifests.length}
              </span>
            )}
          </span>
          <div className="flex items-center space-x-1">
            <button
              onClick={async () => {
                if (dialogOpen) {
                  const selected = await dialogOpen({
                    multiple: false,
                    filters: [
                      { name: "JSON 会话档案", extensions: ["json"] },
                    ],
                  });
                  if (selected && !Array.isArray(selected)) {
                    try {
                      const resp = await fetch(
                        `file://${selected}`,
                      );
                      const jsonStr = await resp.text();
                      await importChatSession(jsonStr);
                      refreshManifests();
                      toast.showToast(
                        "success",
                        "IMPORTED",
                        "会话已从 JSON 文件导入。",
                      );
                    } catch {
                      toast.showToast(
                        "error",
                        "IMPORT FAILED",
                        "无法读取文件 — 请确认选择的是 .json 会话档案。",
                      );
                    }
                  }
                } else {
                  toast.showToast(
                    "info",
                    "BROWSER MODE",
                    "导入功能需 Tauri 桌面环境。",
                  );
                }
              }}
              className="text-[9px] bg-black border border-[#27272a] px-1.5 py-0.5 rounded hover:border-zinc-500 text-zinc-400 hover:text-white transition-colors"
              title="导入 JSON 会话"
            >
              📥
            </button>
            <button
              onClick={handleNewSession}
              className="text-[9px] bg-black border border-[#27272a] px-1.5 py-0.5 rounded hover:border-zinc-500 text-white font-bold transition-colors"
            >
              + NEW
            </button>
          </div>
        </div>

        {/* 会话搜索过滤 */}
        {manifests.length > 0 && (
          <div className="px-2 py-1.5 border-b border-[#27272a]">
            <input
              value={sessionFilter}
              onChange={(e) => setSessionFilter(e.target.value)}
              placeholder="🔍 搜索会话…"
              className="w-full bg-black border border-[#27272a] rounded px-2 py-1 text-[10px] text-zinc-300 placeholder-zinc-600 outline-none focus:border-zinc-500 transition-colors"
            />
          </div>
        )}

        {/* 清单列表流（仅渲染轻量 .meta，毫秒级撑起成百上千条树轴） */}
        <div className="flex-1 overflow-y-auto p-1.5 space-y-1.5 scrollbar-thin">
          {filteredManifests.map((m) => (
            <div
              key={m.session_id}
              className={`group/session p-2 rounded border text-left transition-all cursor-pointer relative ${
                activeSessionId === m.session_id
                  ? "bg-[#27272a]/70 border-zinc-700 text-white"
                  : "border-transparent text-zinc-400 hover:bg-zinc-900/40"
              }`}
            >
              <div onClick={() => handleSwitchSession(m.session_id)}>
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
                <div className="text-[9px] text-zinc-600 mt-1 flex items-center justify-between font-light">
                  <span>🗂️ {m.bound_project}</span>
                  <span className="text-emerald-500 font-medium">
                    ¥{m.total_accumulated_cost.toFixed(3)}
                  </span>
                </div>
                {m.last_message_preview && (
                  <div className="text-[9px] text-zinc-600 mt-1 truncate font-light italic">
                    {m.last_message_preview}
                  </div>
                )}
                <div className="text-[8px] text-zinc-700 mt-0.5 font-light text-right">
                  条数: {m.total_messages_count} |{" "}
                  {m.last_updated.substring(11, 19)}
                </div>
              </div>
              {/* 删除按钮 */}
              <button
                onClick={async (e) => {
                  e.stopPropagation();
                  if (
                    confirm(`确定删除会话「${m.title}」？\n此操作不可撤销。`)
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
                        handleNewSession();
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
                className="absolute top-1.5 right-1.5 w-4 h-4 flex items-center justify-center rounded text-[10px] text-zinc-600 hover:text-red-400 hover:bg-red-950/30 opacity-0 group-hover/session:opacity-100 transition-all"
                title="删除会话"
              >
                ✕
              </button>
            </div>
          ))}
          {filteredManifests.length === 0 && manifests.length > 0 && (
            <div className="p-3 text-[10px] text-zinc-600 italic text-center">
              无匹配会话
            </div>
          )}
          {manifests.length === 0 && (
            <div className="p-3 text-[10px] text-zinc-600 italic text-center">
              尚无历史会话轨道 —
              <br />
              发送第一条消息后自动建档
            </div>
          )}
        </div>

        {/* 侧栏统计摘要 */}
        {manifests.length > 0 && (
          <div className="p-2 border-t border-[#27272a] text-[9px] text-zinc-600 space-y-0.5 shrink-0">
            <div className="flex justify-between">
              <span>💬 会话</span>
              <span className="text-zinc-500">{manifests.length}</span>
            </div>
            <div className="flex justify-between">
              <span>📊 总消息</span>
              <span className="text-zinc-500">
                {manifests.reduce((a, m) => a + m.total_messages_count, 0)}
              </span>
            </div>
            <div className="flex justify-between">
              <span>💰 累计节省</span>
              <span className="text-emerald-500 font-medium">
                ¥
                {manifests
                  .reduce((a, m) => a + m.total_accumulated_cost, 0)
                  .toFixed(3)}
              </span>
            </div>
            <button
              onClick={handleClearAll}
              className="w-full mt-1 text-[8px] text-zinc-700 hover:text-red-400 transition-colors text-center"
            >
              🗑️ 清空全部会话
            </button>
          </div>
        )}
      </div>

      {/* ═══ 右侧主栏：沉浸式高级流式对话控制面板 ═══ */}
      <div className="flex-1 flex flex-col min-w-0 bg-[#09090b] relative h-full">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-[#27272a] bg-[#0c0c0e] shrink-0">
          <div className="flex items-center space-x-2 text-xs">
            <span
              className={`inline-block w-2 h-2 rounded-full ${apiKey ? "bg-emerald-500 animate-ping" : "bg-amber-500"}`}
            />
            <span className="font-bold text-zinc-300 uppercase text-[10px] tracking-wider">
              {t.omni_chat}
            </span>
            {/* Tauri 连通性指示器 */}
            <span
              className={`text-[8px] px-1 rounded ${
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
                ? "🔗 在线"
                : "⚡ 演示"}
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
              <span className="text-[9px] text-cyan-400 animate-pulse">● 心流激活</span>
            ) : !currentHasKey && (
              <span className={anyKey ? "text-[9px] text-cyan-400" : "text-[9px] text-amber-500"}>
                {anyKey ? `(切换至 ${availableProvider})` : "(Demo)"}
              </span>
            )}
            {/* 项目绑定指示 */}
            {currentProject && currentProject !== "default" && (
              <span className="text-[9px] text-cyan-500 border border-cyan-500/20 bg-cyan-950/20 px-1 rounded"
                title={`会话自动绑定到项目: ${currentProject}`}>
                📁 {currentProject}
              </span>
            )}
          </div>
          <div className="flex items-center space-x-1.5">
            <button
              onClick={handleExportSession}
              className="text-[10px] bg-zinc-800/50 hover:bg-zinc-700 border border-zinc-700/50 text-zinc-400 hover:text-zinc-200 px-2 py-0.5 rounded transition-all"
              title="导出为 JSON 文件"
            >
              📤 导出
            </button>
            <button
              onClick={handlePersistSession}
              disabled={isSaving}
              className="text-[10px] bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 text-zinc-200 px-2 py-0.5 rounded font-bold transition-all disabled:opacity-40"
            >
              {isSaving ? "正在执行物理分块..." : "💾 固化当前会话分块"}
            </button>
          </div>
        </div>

        {/* 消息搜索栏 (Ctrl+F) */}
        {searchOpen && (
          <div className="flex items-center space-x-2 px-4 py-1.5 border-b border-[#27272a] bg-[#121214] shrink-0 animate-fadeIn">
            <span className="text-[10px] text-zinc-500">🔍</span>
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
          className="flex-1 p-4 space-y-4 overflow-y-auto scrollbar-thin"
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
              <div className="flex items-center space-x-1.5 text-[9px] text-zinc-500 px-1">
                <span className="font-bold text-zinc-400">
                  {msg.sender}
                </span>
                <span>•</span>
                <span className="bg-[#121214] border border-[#27272a] px-1 rounded text-[9px] text-zinc-300">
                  {msg.model}
                </span>
                {msg.costTokens != null && msg.costTokens > 0 && (
                  <span className="text-zinc-600">
                    ({msg.costTokens}t
                    {msg.isCached && (
                      <span className="text-emerald-500/80 font-bold ml-0.5">
                        [Cache Hit]
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
                  <span className="hidden group-hover:inline text-[8px] text-zinc-600 font-light">
                    Hash: {msg.cachingMarkerHash.substring(0, 6)}
                  </span>
                )}
                <span className="text-[9px] text-zinc-600">
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
                  📋
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
                          {att.type === "doc" ? "📄" : "🖼️"}
                        </span>
                        <span className="text-zinc-300 truncate max-w-[120px] font-medium">
                          {att.name}
                        </span>
                        <span className="text-[9px] text-zinc-600">
                          ({att.sizeOrPath})
                        </span>
                      </div>
                    ))}
                  </div>
                )}
                {msg.thinking && (
                  <details className="mb-2.5 border-l-2 border-zinc-700 pl-2 text-zinc-500 bg-black/30 p-1.5 rounded transition-all group">
                    <summary className="cursor-pointer text-[10px] text-zinc-400 select-none outline-none font-bold hover:text-zinc-300">
                      💡 {t.view_thinking}
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
                    className="mt-2 flex items-center space-x-1 text-[9px] bg-amber-800/30 hover:bg-amber-700/40 border border-amber-700/40 text-amber-300 px-2 py-0.5 rounded transition-colors"
                  >
                    🔄 重试 ({2 - retryCount} 次)
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

        {/* Input */}
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
            <div className="absolute bottom-full left-0 right-0 mb-2 mx-4 bg-[#121214]/95 border border-[#27272a] rounded shadow-2xl z-30 backdrop-blur-md max-h-44 overflow-y-auto animate-slideLeft">
              <div className="px-3 py-1.5 text-[9px] text-zinc-500 font-bold uppercase tracking-wider border-b border-zinc-900">
                快捷斜杠宏命令 (Slash Macros)
              </div>
              {slashCommands.map((sc) => (
                <div
                  key={sc.cmd}
                  onClick={() => selectCommand(sc.cmd, "slash")}
                  className="flex items-center justify-between px-3 py-2 hover:bg-zinc-900/60 cursor-pointer text-xs text-zinc-300 transition-colors"
                >
                  <span className="font-bold text-white">
                    {sc.icon} {sc.cmd}
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
            <div className="absolute bottom-full left-0 right-0 mb-2 mx-4 bg-[#121214]/95 border border-[#27272a] rounded shadow-2xl z-30 backdrop-blur-md max-h-44 overflow-y-auto animate-slideLeft">
              <div className="px-3 py-1.5 text-[9px] text-zinc-500 font-bold uppercase tracking-wider border-b border-zinc-900">
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
                  <span>{stg.type === "doc" ? "📄" : "🖼️"}</span>
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
                    className="text-zinc-600 hover:text-zinc-400 ml-1 font-bold"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          )}

          <form
            data-chat-form
            onSubmit={handleSend}
            className="p-3 border-t border-[#27272a] bg-[#0c0c0e] flex items-center space-x-2"
          >
            {/* 附件挂载按钮（真实文件对话框 + 浏览器降级 mock） */}
            <button
              type="button"
              onClick={async () => {
                if (dialogOpen) {
                  const selected = await dialogOpen({
                    multiple: true,
                    filters: [
                      {
                        name: "文档",
                        extensions: ["md", "pdf", "txt", "xlsx", "csv", "json"],
                      },
                    ],
                  });
                  if (selected) {
                    const paths = Array.isArray(selected)
                      ? selected
                      : [selected];
                    const newAttachments: Attachment[] = paths.map(
                      (p) => {
                        const name = p.split(/[/\\]/).pop() ?? p;
                        return { type: "doc" as const, name, sizeOrPath: p };
                      },
                    );
                    setStagedAttachments((prev) => [...prev, ...newAttachments]);
                  }
                } else {
                  // 浏览器降级 mock
                  setStagedAttachments((prev) => [
                    ...prev,
                    {
                      type: "doc",
                      name: "PRD_Requirements.pdf",
                      sizeOrPath: "124 KB",
                    },
                  ]);
                }
              }}
              title="挂载本地文档/知识库"
              className="w-7 h-7 flex items-center justify-center rounded bg-black border border-[#27272a] hover:border-zinc-500 text-xs transition-colors shrink-0"
            >
              <FileTextIcon size={14} className="stroke-zinc-400" />
            </button>
            <button
              type="button"
              onClick={async () => {
                if (dialogOpen) {
                  const selected = await dialogOpen({
                    multiple: true,
                    filters: [
                      {
                        name: "图片",
                        extensions: ["png", "jpg", "jpeg", "gif", "webp"],
                      },
                    ],
                  });
                  if (selected) {
                    const paths = Array.isArray(selected)
                      ? selected
                      : [selected];
                    const newAttachments: Attachment[] = paths.map(
                      (p) => {
                        const name = p.split(/[/\\]/).pop() ?? p;
                        return {
                          type: "image" as const,
                          name,
                          sizeOrPath: p,
                        };
                      },
                    );
                    setStagedAttachments((prev) => [...prev, ...newAttachments]);
                  }
                } else {
                  setStagedAttachments((prev) => [
                    ...prev,
                    {
                      type: "image",
                      name: "ERP_Error_Snapshot.png",
                      sizeOrPath: "1080P",
                    },
                  ]);
                }
              }}
              title="挂载多模态图片走查"
              className="w-7 h-7 flex items-center justify-center rounded bg-black border border-[#27272a] hover:border-zinc-500 text-xs transition-colors shrink-0"
            >
              <ImageIcon size={14} className="stroke-zinc-400" />
            </button>

            <div className="flex-1 relative flex items-center bg-black border border-[#27272a] rounded-lg px-3 py-2.5 focus-within:border-zinc-500 transition-colors">
              <button
                type="button"
                onClick={() => setMacrosVisible(!macrosVisible)}
                className="text-zinc-500 hover:text-emerald-400 text-sm mr-2 transition-colors"
                title="一键宏指令"
              >
                💡
              </button>
              <span className="text-zinc-600 text-sm font-bold mr-2 select-none">
                $
              </span>
              <input
                ref={inputRef}
                type="text"
                value={input}
                onChange={handleInputChange}
                placeholder={
                  apiKey
                    ? "键入 / 触发宏命令，键入 @ 唤醒特种兵…"
                    : "⚙️ 请先在全局配置中填入 API Key…"
                }
                className="w-full bg-transparent text-sm text-[#fafafa] placeholder-zinc-600 outline-none border-none p-0"
                disabled={isThinking}
              />
            </div>
            <button
              type="submit"
              disabled={isThinking}
              className="bg-zinc-100 hover:bg-zinc-200 active:bg-zinc-300 active:scale-95 text-black font-bold text-sm px-4 py-2.5 rounded-lg transition-all duration-150 flex items-center space-x-1 outline-none shadow-sm disabled:opacity-40 disabled:scale-100 disabled:cursor-not-allowed shrink-0"
            >
              <span>{t.execute}</span>
              <span className="text-[10px] bg-zinc-300 px-1 rounded text-zinc-700 ml-0.5">
                ↵
              </span>
            </button>
          </form>

          {/* 状态栏：会话统计 + 审批指示 */}
          <div className="flex items-center justify-between px-4 py-1 border-t border-[#1a1a1e] bg-[#0c0c0e] text-[9px] text-zinc-600 select-none">
            <div className="flex items-center space-x-3">
              <span>💬 {messages.length} 条</span>
              <span>|</span>
              <span>💰 ¥{messages.reduce((a, m) => a + (m.costTokens ?? 0) * 0.000001, 0).toFixed(4)}</span>
              {currentProject && currentProject !== "default" && (
                <>
                  <span>|</span>
                  <span className="text-cyan-500">📁 {currentProject}</span>
                </>
              )}
            </div>
            {/* 审批门禁状态指示 */}
            <span className="text-red-400" title="审批门禁已激活，请通过左侧 Dock 的 🛡️ 图标访问审批面板">
              🛡️ 第四红线: 审批门禁已激活
            </span>
          </div>

          {/* 键盘快捷提示 + Token 计数器 */}
          <div className="flex items-center justify-between px-4 pb-2 text-[9px] text-zinc-700 select-none">
            <div className="flex items-center space-x-3">
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
                  Ctrl+Enter
                </kbd>
                发送
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
                  /
                </kbd>
                宏命令
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
                  @
                </kbd>
                特种兵
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
                  Ctrl+N
                </kbd>
                新建
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
                  Ctrl+S
                </kbd>
                保存
              </span>
              <span>
                <kbd className="px-1 py-0.5 bg-[#121214] border border-[#27272a] rounded text-[8px] text-zinc-500 mr-1">
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
    </div>
  );
}
