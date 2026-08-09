import { useState, useEffect } from "react";
import { useT } from "@/lib/i18n-context";
import { listSkills, listMcpServers, type SkillItem, type McpServerItem, mcpConnectAndInit, mcpFetchTools } from "@/lib/tauri";
import {
  Zap,
  Plug,
  Server,
  Database,
  GitBranch,
  Globe,
  Circle,
  ExternalLink,
  Upload,
  Code2,
  FileJson,
  Terminal,
} from "lucide-react";
import Modal from "@/components/Modal";

interface Skill {
  id: string;
  name: string;
  description: string;
  type: "skill" | "subagent";
  enabled: boolean;
  synced: boolean;
  model?: string;
  premium?: boolean;
  badge?: string;
  category?: string;
}

interface McpServer {
  id: string;
  name: string;
  transport: "stdio" | "sse";
  tools: number;
  resources: number;
  connected: boolean;
}

const skills: Skill[] = [
  // ─── 超级专属 Skill (顶级置顶) ──────────────────────────────────
  { id: "vlm_privacy_dynamic_mask", name: "🛡️ VLM-Privacy DynamicMask", description: "ONNX端侧实时隐私遮罩 · 敏感数据绝不出海 · 视觉开销暴省80%", type: "skill", enabled: true, synced: true, premium: true, badge: "🛡️ 风控", category: "security" },
  { id: "win32_handle_texthijacker", name: "⚡ Win32-Handle TextHijacker", description: "句柄级免Vision无感数据对齐 · 速度提升300% · Token千分之一", type: "skill", enabled: true, synced: true, premium: true, badge: "⚡ 极速", category: "workbuddy" },
  { id: "chronos_omni_rewind_trigger", name: "⏳ Chronos-OmniRewind Trigger", description: "VSS原子冷备份+一键时空逆转 · 零沉没成本 · 秒级回滚", type: "skill", enabled: true, synced: true, premium: true, badge: "⏳ 回溯", category: "security" },
  // ─── 超级专属 Skill 4-6 ────────────────────────────────────────
  { id: "cluster_docker_hothealer", name: "🐳 Cluster-Docker HotHealer", description: "SSH隧道切入远程容器静默编译 · Stderr截获端侧自愈", type: "skill", enabled: true, synced: true, premium: true, badge: "🌐 云盾", category: "devops" },
  { id: "vlm_uitree_aligner", name: "🎯 VLM-UITree Aligner", description: "Win32 UIA控件树磁吸纠偏 · 像素吸附杜绝误点击 · VLM开销一折", type: "skill", enabled: true, synced: true, premium: true, badge: "🎯 吸附", category: "workbuddy" },
  { id: "evolution_delta_packer", name: "📦 Evolution-Delta Packer", description: "错题本经验脱敏打包导出 · 0Token团队共享 · 企业知识资产", type: "skill", enabled: true, synced: true, premium: true, badge: "📦 资产", category: "evolution" },
  { id: "omnidesign_matrix", name: "🎨 OmniDesign-Matrix", description: "自然语言→跨端UI/UX代码 · Vercel/Linear/Apple三主题 · ONNX像素走查", type: "skill", enabled: true, synced: true, premium: true, badge: "🎨 设计", category: "design" },
  // ─── 专属内置 Skill (置顶高亮) ───────────────────────────────────
  { id: "vlm_diff_inspector", name: "🖼️ VLM-Diff Inspector", description: "像素级多模态还原度走查 · 暴省80% VLM Token", type: "skill", enabled: true, synced: true, premium: true, badge: "⚡ 降本", category: "quality" },
  { id: "context_glue_excelfiller", name: "📊 Context-Glue ExcelFiller", description: "Win32句柄直写批量填表 · Token压缩至千分之一", type: "skill", enabled: true, synced: true, premium: true, badge: "⚡ 降本", category: "workbuddy" },
  { id: "checkpoints_chronotrigger", name: "⏱️ Checkpoints-ChronoTrigger", description: "VSS卷影+窗口快照 · 秒级时光倒流回滚", type: "skill", enabled: true, synced: true, premium: true, badge: "🛡️ 安全", category: "security" },
  // ─── 常规 Skills ─────────────────────────────────────────────
  { id: "explore", name: "@Explore", description: "源码只读检索", type: "subagent", enabled: true, synced: true, model: "flash" },
  { id: "scout", name: "@Scout", description: "远程文档抓取", type: "subagent", enabled: true, synced: true, model: "flash" },
  { id: "compaction", name: "@Compaction", description: "上下文压缩蒸馏", type: "subagent", enabled: true, synced: true, model: "flash" },
  { id: "ppt-gen", name: "PPT 自动生成", description: "Markdown→PPTX 转换", type: "skill", enabled: false, synced: false },
  { id: "excel-merge", name: "Excel 智能合并", description: "多表合并与清洗", type: "skill", enabled: true, synced: true },
  { id: "code-review", name: "Code Review", description: "代码审查模板", type: "skill", enabled: true, synced: true },
];

interface McpServer {
  id: string;
  name: string;
  transport: "stdio" | "sse";
  tools: number;
  resources: number;
  connected: boolean;
  premium?: boolean;
  badge?: string;
  description?: string;
}

const mcpServers: McpServer[] = [
  // ─── 三大超级 MCP 服务 ─────────────────────────────────────────
  { id: "win32-registry", name: "🔍 Win32 Registry Sensor", transport: "stdio", tools: 3, resources: 0, connected: true, premium: true, badge: "🖥️ 系统", description: "注册表读取+环境变量注入，确定性探测替代模型猜测" },
  { id: "local-vector-glue", name: "🧠 Local Vector Glue", transport: "stdio", tools: 2, resources: 2, connected: true, premium: true, badge: "⚡ 压缩", description: "ONNX端侧向量检索，代码上下文体积压缩至千分之一" },
  { id: "audit-vault", name: "🔒 Audit Vault", transport: "stdio", tools: 3, resources: 0, connected: true, premium: true, badge: "🛡️ 合规", description: "AST增量审查+GPL协议扫描，端侧零云端请求" },
  // ─── 常规 MCP ─────────────────────────────────────────────────
  { id: "postgres", name: "PostgreSQL", transport: "stdio", tools: 12, resources: 3, connected: true },
  { id: "github", name: "GitHub API", transport: "sse", tools: 8, resources: 2, connected: true },
  { id: "filesystem", name: "Filesystem", transport: "stdio", tools: 6, resources: 1, connected: true },
  { id: "slack", name: "Slack", transport: "sse", tools: 4, resources: 0, connected: false },
];

export default function SkillMcpHub() {
  const _t = useT(); void _t;
  const t = _t;
  const [activeTab, setActiveTab] = useState<"skills" | "mcp">("skills");
  const [selectedMcpTool, setSelectedMcpTool] = useState<string | null>(null);
  const [schemaOpen, setSchemaOpen] = useState(false);
  const [schemaData, setSchemaData] = useState<{ server: string; tool: string; schema: string } | null>(null);

  // Live data
  const [liveSkills, setLiveSkills] = useState<SkillItem[]>([]);
  const [liveMcp, setLiveMcp] = useState<McpServerItem[]>([]);

  useEffect(() => {
    listSkills().then(setLiveSkills).catch(() => {});
    listMcpServers().then(setLiveMcp).catch(() => {});
    const iv = setInterval(() => {
      listSkills().then(setLiveSkills).catch(() => {});
      listMcpServers().then(setLiveMcp).catch(() => {});
    }, 5000);
    return () => clearInterval(iv);
  }, []);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center space-x-2 px-3 py-2.5 border-b border-cs-border">
        <Zap className="w-3.5 h-3.5 text-cs-warn" />
        <span className="text-[11px] font-bold text-cs-text tracking-wide">
          {t.ecosystem_hub}
        </span>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-cs-border">
        <button
          onClick={() => setActiveTab("skills")}
          className={`flex-1 py-2 text-[10px] font-medium transition-colors ${
            activeTab === "skills"
              ? "text-cs-text border-b border-cs-accent"
              : "text-cs-muted hover:text-cs-dim"
          }`}
        >
          {t.skills} ({liveSkills.length || skills.length})
        </button>
        <button
          onClick={() => setActiveTab("mcp")}
          className={`flex-1 py-2 text-[10px] font-medium transition-colors ${
            activeTab === "mcp"
              ? "text-cs-text border-b border-cs-accent"
              : "text-cs-muted hover:text-cs-dim"
          }`}
        >
          {t.mcp} ({liveMcp.length || mcpServers.length})
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === "skills" ? (
          <div className="p-3">
            {/* Import button */}
            <button className="w-full flex items-center justify-center space-x-1.5 px-2 py-1.5 rounded border border-cs-border text-cs-muted text-[10px] hover:border-cs-dim hover:text-cs-dim transition-colors mb-3">
              <Upload className="w-3 h-3" />
              <span>{t.import_skill}</span>
            </button>

            {/* Skill cards grid — 实时数据优先，mock 兜底 */}
            <div className="grid grid-cols-2 gap-2">
              {(liveSkills.length > 0 ? liveSkills.map(s => ({
                id: s.manifest.id, name: s.manifest.name, description: s.manifest.description || "",
                type: (s.state === "subagent" ? "subagent" : "skill") as "skill" | "subagent",
                enabled: true, synced: true,
                badge: s.state === "subagent" ? "🤖 Agent" : "⚡ Skill",
                category: s.state || "general",
              })) : skills).map((skill) => (
                <SkillCard key={skill.id} skill={skill} />
              ))}
            </div>
          </div>
        ) : (
          <div className="p-3 space-y-2">
            {(liveMcp.length > 0 ? liveMcp.map(s => ({
              id: s.id, name: s.name, transport: (typeof s.transport === "object" && "Stdio" in (s.transport || {}) ? "stdio" : "sse") as "stdio" | "sse",
              tools: s.tools_count ?? 0, resources: s.resources_count ?? 0,
              connected: s.connected ?? false,
            })) : mcpServers).map((server) => (
              <McpServerCard
                key={server.id}
                server={server}
                expanded={selectedMcpTool === server.id}
                onToggle={() =>
                  setSelectedMcpTool(
                    selectedMcpTool === server.id ? null : server.id,
                  )
                }
                onViewSchema={(srv, tool, schema) => {
                  setSchemaData({ server: srv, tool, schema });
                  setSchemaOpen(true);
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* JSON Schema Modal */}
      <Modal
        open={schemaOpen}
        onClose={() => setSchemaOpen(false)}
        title={schemaData ? `${schemaData.server} :: ${schemaData.tool}` : "JSON Schema"}
      >
        {schemaData && (
          <pre className="text-[10px] text-cs-dim font-mono whitespace-pre-wrap overflow-auto max-h-[60vh] bg-cs-bg p-3 rounded border border-cs-border">
            {formatJson(schemaData.schema)}
          </pre>
        )}
      </Modal>

      {/* Bottom */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-3 flex items-center text-[9px] text-cs-muted space-x-3">
        <Plug className="w-2.5 h-2.5 text-cs-accent" />
        <span>
          {liveMcp.filter((s: McpServerItem) => s.connected).length || mcpServers.filter((s) => s.connected).length}/{liveMcp.length || mcpServers.length}{" "}
          {t.mcp_connected_label}
        </span>
        <span className="ml-auto">
          {liveSkills.filter((s: SkillItem) => s.state === "Active").length || skills.filter((s) => s.enabled).length}/{liveSkills.length || skills.length} {t.skills_active_label}
        </span>
      </div>
    </div>
  );
}

function SkillCard({ skill }: { skill: Skill }) {
  const t = useT();
  return (
    <div
      className={`p-2 rounded border transition-all relative ${
        skill.premium
          ? skill.enabled
            ? "border-cyan-500/40 bg-cyan-950/10 hover:border-cyan-500/60 shadow-[0_0_8px_rgba(34,211,238,0.08)]"
            : "border-cyan-500/20 bg-cyan-950/5 opacity-60"
          : skill.enabled
            ? "border-cs-border bg-cs-surface hover:border-cs-dim"
            : "border-cs-border/30 bg-cs-bg/30 opacity-60"
      }`}
    >
      {/* Premium badge */}
      {skill.premium && skill.badge && (
        <span className={`absolute -top-1.5 -right-1.5 text-[7px] px-1.5 py-0.5 rounded-full font-bold border ${
          skill.badge.includes("安全") ? "bg-red-950/80 border-red-800/50 text-red-400" : "bg-emerald-950/80 border-emerald-800/50 text-emerald-400"
        }`}>
          {skill.badge}
        </span>
      )}

      <div className="flex items-start justify-between mb-1">
        <div className="flex items-center space-x-1">
          {skill.premium ? (
            <Zap className={`w-3 h-3 ${skill.badge?.includes("安全") ? "text-red-400" : "text-cyan-400"}`} />
          ) : skill.type === "subagent" ? (
            <Code2 className="w-3 h-3 text-cs-info" />
          ) : (
            <FileJson className="w-3 h-3 text-cs-warn" />
          )}
          <span className={`text-[10px] font-bold ${skill.premium ? "text-cyan-300" : "text-cs-text"}`}>
            {skill.name}
          </span>
        </div>
        {/* Hot-reload toggle */}
        <button
          onClick={() => {
            // In production: call IPC to toggle skill hot-reload
          }}
          className={`w-7 h-4 rounded-full p-0.5 transition-colors relative outline-none ${
            skill.enabled ? (skill.premium ? "bg-cyan-500" : "bg-cs-accent") : "bg-[#27272a]"
          }`}
        >
          <div className={`w-3 h-3 rounded-full bg-white transition-transform ${skill.enabled ? "translate-x-3" : "translate-x-0"}`} />
        </button>
      </div>
      <div className={`text-[8px] mb-1.5 ${skill.premium ? "text-cyan-400/80" : "text-cs-muted"}`}>
        {skill.description}
      </div>
      <div className="flex items-center space-x-2">
        {skill.premium && (
          <span className={`px-1 py-0.5 rounded text-[7px] font-bold ${
            skill.badge?.includes("安全") ? "bg-red-950/50 text-red-400 border border-red-800/30" : "bg-cyan-950/50 text-cyan-400 border border-cyan-800/30"
          }`}>
            {skill.badge}
          </span>
        )}
        {skill.model && (
          <span className="px-1 py-0.5 rounded bg-cs-info/10 text-cs-info text-[7px]">
            {skill.model}
          </span>
        )}
        <span
          className={`flex items-center space-x-0.5 text-[7px] ${
            skill.synced ? "text-cs-accent" : "text-cs-muted"
          }`}
        >
          <Circle className={`w-1 h-1 ${skill.synced ? "fill-cs-accent" : ""}`} />
          <span>{skill.synced ? t.prompt_synced : t.not_synced}</span>
        </span>
      </div>
    </div>
  );
}

function formatJson(json: string): string {
  try {
    return JSON.stringify(JSON.parse(json), null, 2);
  } catch {
    return json;
  }
}

function McpServerCard({
  server,
  expanded,
  onToggle,
  onViewSchema,
}: {
  server: McpServer;
  expanded: boolean;
  onToggle: () => void;
  onViewSchema?: (server: string, tool: string, schema: string) => void;
}) {
  const t = useT();
  const transportIcons: Record<string, React.ComponentType<{ className?: string }>> = {
    stdio: Terminal,
    sse: Globe,
  };
  const TransportIcon = transportIcons[server.transport] || Globe;

  return (
    <div
      className={`rounded border transition-all cursor-pointer relative ${
        server.premium
          ? expanded
            ? "border-cyan-500/50 bg-cyan-950/15 shadow-[0_0_10px_rgba(34,211,238,0.1)]"
            : "border-cyan-500/30 bg-cyan-950/10 hover:border-cyan-500/50"
          : expanded
            ? "border-cs-accent-border bg-cs-accent-dim/10"
            : server.connected
              ? "border-cs-border bg-cs-surface hover:border-cs-dim"
              : "border-cs-border/30 bg-cs-bg/30 opacity-60"
      }`}
      onClick={onToggle}
    >
      {/* Premium badge */}
      {server.premium && server.badge && (
        <span className="absolute -top-1.5 -right-1.5 text-[7px] px-1.5 py-0.5 rounded-full font-bold bg-cyan-950/80 border border-cyan-800/50 text-cyan-400">
          {server.badge}
        </span>
      )}

      <div className="flex items-center px-3 py-2">
        <div className="flex items-center space-x-2 flex-1">
          {server.premium ? (
            <Zap className="w-3 h-3 text-cyan-400" />
          ) : server.name === "PostgreSQL" ? <Database className="w-3 h-3 text-cs-info" />
          : server.name === "GitHub API" ? <GitBranch className="w-3 h-3 text-cs-dim" />
          : server.name === "Filesystem" ? <Server className="w-3 h-3 text-cs-accent" />
          : server.name === "Slack" ? <Globe className="w-3 h-3 text-cs-warn" />
          : <Server className="w-3 h-3 text-cs-muted" />}
          <div>
            <div className={`text-[10px] font-bold ${server.premium ? "text-cyan-300" : "text-cs-text"}`}>
              {server.name}
            </div>
            <div className="flex items-center space-x-1.5 text-[8px] text-cs-muted">
              <TransportIcon className="w-2 h-2" />
              <span>{server.transport.toUpperCase()}</span>
              {server.premium && server.description && (
                <span className="text-cyan-400/60 truncate max-w-[120px]">· {server.description}</span>
              )}
            </div>
          </div>
        </div>
        <div className="flex items-center space-x-3">
          <div className="text-[9px] text-cs-dim">
            <span className="font-bold">{server.tools}</span> {t.tools_label}
          </div>
          <div
            className={`w-1.5 h-1.5 rounded-full ${
              server.connected ? "bg-cs-accent" : "bg-cs-muted"
            }`}
          />
        </div>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div className="px-3 pb-2 border-t border-cs-border/50 pt-2 space-y-1">
          <div className="flex items-center justify-between text-[8px]">
            <span className="text-cs-muted">{t.status_label}</span>
            <span
              className={`font-medium ${
                server.connected ? "text-cs-accent" : "text-cs-muted"
              }`}
            >
              {server.connected ? t.connected : t.disconnected}
            </span>
          </div>
          <div className="flex items-center justify-between text-[8px]">
            <span className="text-cs-muted">{t.transport_label}</span>
            <span className="text-cs-dim">[{server.transport.toUpperCase()}]</span>
          </div>
          <div className="flex items-center justify-between text-[8px]">
            <span className="text-cs-muted">{t.resources_label}</span>
            <span className="text-cs-dim">{server.resources} paths</span>
          </div>
          <div className="flex space-x-1 mt-1">
            <button
              className="flex-1 flex items-center justify-center space-x-1 px-2 py-1 rounded bg-emerald-950/30 border border-emerald-800/30 text-emerald-400 text-[8px] hover:bg-emerald-900/40 transition-colors"
              onClick={async (e) => {
                e.stopPropagation();
                try { await mcpConnectAndInit(server.id); } catch {}
              }}>
              连接
            </button>
            <button
              className="flex-1 flex items-center justify-center space-x-1 px-2 py-1 rounded bg-cyan-950/30 border border-cyan-800/30 text-cyan-400 text-[8px] hover:bg-cyan-900/40 transition-colors"
              onClick={async (e) => {
                e.stopPropagation();
                try { await mcpFetchTools(server.id); } catch {}
              }}>
              刷新工具
            </button>
          </div>
          <button
            className="w-full mt-1 flex items-center justify-center space-x-1 px-2 py-1 rounded bg-cs-bg border border-cs-border text-cs-dim text-[8px] hover:border-cs-dim transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              onViewSchema?.(
                server.name,
                "tools",
                JSON.stringify(
                  {
                    type: "object",
                    properties: {
                      query: { type: "string", description: "SQL query to execute" },
                      params: { type: "array", items: { type: "string" } },
                    },
                    required: ["query"],
                  },
                  null,
                  2,
                ),
              );
            }}
          >
            <ExternalLink className="w-2 h-2" />
            <span>{t.view_schema}</span>
          </button>
        </div>
      )}
    </div>
  );
}
