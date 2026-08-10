// Auto-Routing Visualization Panel — 全局自动路由规则可视化
//
// 功能：
//   1. Agent→模型 分配矩阵 (谁用什么模型)
//   2. 关键词→Agent 路由规则表 (可搜索/过滤)
//   3. 模型能力对比卡 (质量/延迟/成本)
//   4. 路由统计 (命中率/使用频率)
//   5. 实时模型状态指示灯

import { useState, useEffect, useMemo } from "react";
import {
  Search, Activity,
  ArrowRight, Layers,
} from "lucide-react";
import { collabGetModelRanking } from "@/lib/tauri";
import { useT } from "@/lib/i18n-context";

// ─── 路由规则定义 ─────────────────────────────────────────────────

interface RouteRule {
  keywords: string[];
  agent: string;
  model: string;
  tier: "pro" | "flash";
  category: string;
  description: string;
}

interface AgentModelMapping {
  agent: string;
  model: string;
  tier: "pro" | "flash";
  type: string;
  icon: string;
}

interface ModelInfo {
  name: string;
  quality: string;
  latency: string;
  cost: string;
  online: boolean;
  bestFor: string;
}

const routeRules: RouteRule[] = [
  { keywords: ["架构", "设计", "重构", "拆分", "微服务", "选型"], agent: "backend-engineer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "后端架构设计与技术选型" },
  { keywords: ["安全", "漏洞", "注入", "XSS", "认证", "审计", "渗透"], agent: "security-review", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "安全漏洞审查与渗透测试" },
  { keywords: ["审查", "review", "检查代码", "代码质量"], agent: "review", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "代码审查与质量把控" },
  { keywords: ["数据库", "SQL", "查询", "索引", "N+1", "慢查询"], agent: "sql-optimizer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "SQL优化与数据库审查" },
  { keywords: ["API", "接口", "REST", "端点", "OpenAPI"], agent: "api-designer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "API设计与规范审查" },
  { keywords: ["性能", "优化", "慢", "bundle", "瓶颈", "加载速度"], agent: "perf", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "性能分析与优化" },
  { keywords: ["部署", "CI/CD", "Docker", "K8s", "DevOps", "上线"], agent: "devops-engineer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "DevOps与部署自动化" },
  { keywords: ["产品", "PRD", "需求", "竞品", "用户故事", "MVP"], agent: "product-manager", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "产品需求管理" },
  { keywords: ["UI", "样式", "CSS", "可访问性", "a11y", "审查组件"], agent: "ui-review", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "UI组件审查" },
  { keywords: ["前端", "状态管理", "React", "Vue", "组件拆分"], agent: "frontend-architect", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "前端架构设计" },
  { keywords: ["测试", "单元测试", "E2E", "覆盖率", "mock", "用例"], agent: "test-gen", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "测试用例生成" },
  { keywords: ["生成组件", "创建组件", "添加组件", "新建页面"], agent: "component-builder", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "组件脚手架" },
  { keywords: ["修复", "bug", "错误", "debug", "调试"], agent: "debug", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "错误诊断与修复" },
  { keywords: ["重构代码", "代码坏味道", "优化结构"], agent: "refactor", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "安全重构" },
  { keywords: ["CSS架构", "样式重构", "设计token", "响应式策略", "主题化"], agent: "css-architect", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "CSS架构顾问" },
  { keywords: ["UI设计", "设计系统", "动效", "交互规范", "Design Token"], agent: "ui-designer", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "UI设计系统" },
  { keywords: ["界面美化", "美化设计", "UI美化", "设计感", "美观", "视觉效果", "视觉升级", "UI改造"], agent: "ui-ux-pro-max", model: "deepseek-v4-flash", tier: "flash", category: "代码生成", description: "UI/UX专业设计" },
  { keywords: ["探索", "查找", "理解", "分析代码", "在哪里", "怎么工作"], agent: "explore", model: "deepseek-v4-flash", tier: "flash", category: "探索", description: "代码库探索" },
  { keywords: ["文档", "注释", "README", "说明", "写文档"], agent: "knowledge-base", model: "deepseek-v4-flash", tier: "flash", category: "探索", description: "知识库文档" },
  { keywords: ["资料", "调研", "最新技术", "外部信息", "查资料"], agent: "research", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "技术调研" },
  { keywords: ["技术咨询", "技术评估", "技术选型", "技术判断", "官方文档", "技术可行性"], agent: "software-advisor", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "技术咨询" },
  { keywords: ["合规", "法规", "GDPR", "PIPL", "隐私合规", "数据合规"], agent: "compliance-specialist", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "合规审查" },
  { keywords: ["安全架构", "威胁建模", "纵深防御", "密钥管理", "安全设计"], agent: "security-engineer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "安全架构设计" },
  { keywords: ["质量保障", "质量门禁", "Bug分类", "测试策略", "测试金字塔"], agent: "qa-engineer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "质量保障" },
  { keywords: ["移动端UI", "移动端设计", "iOS设计", "Material Design", "触控交互", "手势设计", "安全区域"], agent: "mobile-ui-designer", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "移动端UI设计" },
  { keywords: ["鸿蒙编译", "编译构建", "hvigor", "HAP", "HSP", "HAR", "ArkCompiler"], agent: "harmonyos-build-master", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "鸿蒙编译构建" },
  { keywords: ["Agent优化", "Agent审查", "Agent进化", "技能优化"], agent: "agent-evolution", model: "deepseek-v4-pro", tier: "pro", category: "深度推理", description: "Agent进化" },
];

const agentMappings: AgentModelMapping[] = [
  { agent: "backend-engineer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🏗️" },
  { agent: "security-review", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🛡️" },
  { agent: "review", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🔍" },
  { agent: "api-designer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🔌" },
  { agent: "devops-engineer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🚀" },
  { agent: "product-manager", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "📋" },
  { agent: "frontend-architect", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🎨" },
  { agent: "ui-review", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🖼️" },
  { agent: "sql-optimizer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🗄️" },
  { agent: "perf", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "⚡" },
  { agent: "compliance-specialist", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "⚖️" },
  { agent: "security-engineer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🔐" },
  { agent: "qa-engineer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "✅" },
  { agent: "research", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "📚" },
  { agent: "agent-evolution", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🧬" },
  { agent: "software-advisor", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "💡" },
  { agent: "mobile-ui-designer", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "📱" },
  { agent: "harmonyos-build-master", model: "deepseek-v4-pro", tier: "pro", type: "深度推理", icon: "🔧" },
  { agent: "test-gen", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "🧪" },
  { agent: "component-builder", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "🧩" },
  { agent: "debug", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "🐛" },
  { agent: "refactor", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "♻️" },
  { agent: "css-architect", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "🎯" },
  { agent: "ui-designer", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "✨" },
  { agent: "ui-ux-pro-max", model: "deepseek-v4-flash", tier: "flash", type: "代码生成", icon: "🌟" },
  { agent: "explore", model: "deepseek-v4-flash", tier: "flash", type: "探索", icon: "🔎" },
  { agent: "knowledge-base", model: "deepseek-v4-flash", tier: "flash", type: "探索", icon: "📝" },
];

// ─── Panel Component ───────────────────────────────────────────────

export default function AutoRoutingPanel() {
  const t = useT();
  const [activeTab, setActiveTab] = useState<"rules" | "models" | "matrix">("rules");
  const [search, setSearch] = useState("");
  const [filterTier, setFilterTier] = useState<"all" | "pro" | "flash">("all");
  const [selectedRule, setSelectedRule] = useState<RouteRule | null>(null);
  const [modelRanking, setModelRanking] = useState<ModelInfo[]>([]);

  useEffect(() => {
    collabGetModelRanking().then((r) => {
      if (r?.model_ranking) {
        setModelRanking(r.model_ranking.map((m) => ({
          name: m.name, quality: m.quality, latency: m.avg_latency,
          cost: m.cost, online: m.online, bestFor: "",
        })));
      }
    }).catch(() => {});
  }, []);

  const filteredRules = useMemo(() =>
    routeRules.filter((r) => {
      if (filterTier !== "all" && r.tier !== filterTier) return false;
      if (search) {
        const q = search.toLowerCase();
        return r.keywords.some((k) => k.includes(q)) ||
          r.agent.includes(q) || r.category.includes(q) || r.description.includes(q);
      }
      return true;
    }),
    [search, filterTier],
  );

  const proCount = agentMappings.filter((a) => a.tier === "pro").length;
  const flashCount = agentMappings.filter((a) => a.tier === "flash").length;
  const uniqueModels = [...new Set(agentMappings.map((a) => a.model))];

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center space-x-2 px-3 py-2.5 border-b border-cs-border">
        <Activity className="w-3.5 h-3 text-cs-accent" />
        <span className="text-[11px] font-bold text-cs-text tracking-wide">{t.ar_title}</span>
        <span className="text-[9px] text-cs-muted ml-auto">
          {proCount} pro · {flashCount} flash · {uniqueModels.length} {t.ar_model}
        </span>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-cs-border">
        {(["rules", "models", "matrix"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`flex-1 py-1.5 text-[10px] font-medium transition-colors ${
              activeTab === tab ? "text-cs-text border-b border-cs-accent" : "text-cs-muted hover:text-cs-dim"
            }`}
          >
            {tab === "rules" ? t.ar_rules : tab === "models" ? t.ar_models : t.ar_matrix}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {/* ─── Rules Tab ─────────────────────────────────────────── */}
        {activeTab === "rules" && (
          <div className="p-2 space-y-2">
            {/* Search + Filter */}
            <div className="flex items-center space-x-2">
              <div className="flex-1 flex items-center space-x-1 px-2 py-1 rounded border border-cs-border bg-cs-bg">
                <Search className="w-3 h-3 text-cs-muted" />
                <input
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder={t.ar_search_placeholder}
                  className="flex-1 bg-transparent text-cs-text text-[9px] outline-none placeholder:text-cs-muted"
                />
              </div>
              <select
                value={filterTier}
                onChange={(e) => setFilterTier(e.target.value as "all" | "pro" | "flash")}
                className="px-2 py-1 rounded border border-cs-border bg-cs-bg text-cs-text text-[9px]"
              >
                <option value="all">{t.ar_all}</option>
                <option value="pro">{t.ar_pro}</option>
                <option value="flash">{t.ar_flash}</option>
              </select>
            </div>

            {/* Stats bar */}
            <div className="flex items-center space-x-3 text-[9px] text-cs-muted px-1">
              <span className="flex items-center space-x-1">
                <Layers className="w-3 h-3" />
                <span>{filteredRules.length} {t.ar_rules_count}</span>
              </span>
              <span className="flex items-center space-x-1">
                <ArrowRight className="w-3 h-3" />
                <span>{[...new Set(filteredRules.map((r) => r.agent))].length} {t.ar_agents_count}</span>
              </span>
            </div>

            {/* Rule cards */}
            <div className="space-y-1">
              {filteredRules.map((rule) => (
                <div
                  key={rule.agent + rule.keywords[0]}
                  onClick={() => setSelectedRule(selectedRule?.agent === rule.agent ? null : rule)}
                  className={`p-2 rounded border transition-all cursor-pointer ${
                    selectedRule?.agent === rule.agent
                      ? "border-cs-accent bg-cs-accent/5"
                      : "border-cs-border bg-cs-surface hover:border-cs-dim"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <span className={`w-1.5 h-1.5 rounded-full ${rule.tier === "pro" ? "bg-purple-400" : "bg-emerald-400"}`} />
                      <span className="text-[10px] font-bold text-cs-text">{rule.agent}</span>
                      <span className={`text-[8px] px-1 py-0.5 rounded ${
                        rule.tier === "pro"
                          ? "bg-purple-950/30 text-purple-400 border border-purple-800/30"
                          : "bg-emerald-950/30 text-emerald-400 border border-emerald-800/30"
                      }`}>
                        {rule.tier === "pro" ? "Pro" : "Flash"}
                      </span>
                    </div>
                    <span className="text-[9px] text-cs-dim font-mono">{rule.model}</span>
                  </div>

                  {/* Keywords */}
                  <div className="flex flex-wrap gap-1 mt-1.5">
                    {rule.keywords.slice(0, 6).map((kw) => (
                      <span key={kw} className="text-[8px] px-1 py-0.5 rounded bg-cs-bg text-cs-muted">
                        {kw}
                      </span>
                    ))}
                    {rule.keywords.length > 6 && (
                      <span className="text-[8px] text-cs-dim">+{rule.keywords.length - 6}</span>
                    )}
                  </div>

                  {/* Expanded detail */}
                  {selectedRule?.agent === rule.agent && (
                    <div className="mt-2 pt-2 border-t border-cs-border space-y-1">
                      <div className="flex items-center justify-between text-[8px]">
                        <span className="text-cs-muted">{t.ar_category}</span>
                        <span className="text-cs-dim">{rule.category}</span>
                      </div>
                      <div className="flex items-center justify-between text-[8px]">
                        <span className="text-cs-muted">{t.ar_description}</span>
                        <span className="text-cs-dim">{rule.description}</span>
                      </div>
                      <div className="flex items-center justify-between text-[8px]">
                        <span className="text-cs-muted">{t.ar_route_model}</span>
                        <span className="text-cs-accent font-mono">{rule.model}</span>
                      </div>
                      <div className="flex items-center justify-between text-[8px]">
                        <span className="text-cs-muted">{t.ar_match_keywords}</span>
                        <span className="text-cs-dim">{rule.keywords.length}</span>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ─── Models Tab ──────────────────────────────────────────── */}
        {activeTab === "models" && (
          <div className="p-2 space-y-2">
            {/* Model cards */}
            {modelRanking.length > 0 ? (
              modelRanking.map((m) => (
                <div key={m.name} className="p-2 rounded border border-cs-border bg-cs-surface">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-[10px] font-bold text-cs-text">{m.name}</span>
                    <span className={`flex items-center space-x-1 text-[8px] ${m.online ? "text-emerald-400" : "text-red-400"}`}>
                      <span className={`w-1.5 h-1.5 rounded-full ${m.online ? "bg-emerald-400" : "bg-red-400"}`} />
                      {m.online ? t.ar_online : t.ar_offline}
                    </span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-[8px]">
                    <div>
                      <span className="text-cs-muted">{t.ar_quality}</span>
                      <div className="text-cs-accent font-mono">{m.quality}</div>
                    </div>
                    <div>
                      <span className="text-cs-muted">{t.ar_latency}</span>
                      <div className="text-cs-dim font-mono">{m.latency}</div>
                    </div>
                    <div>
                      <span className="text-cs-muted">{t.ar_cost}</span>
                      <div className="text-cs-dim font-mono">{m.cost}</div>
                    </div>
                  </div>
                  {/* Progress bar for quality */}
                  <div className="mt-1.5 h-1 bg-cs-bg rounded overflow-hidden">
                    <div
                      className="h-full bg-cs-accent rounded transition-all"
                      style={{ width: `${m.quality}%` }}
                    />
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-cs-muted text-[10px]">
                {t.ar_loading_models}
              </div>
            )}
          </div>
        )}

        {/* ─── Matrix Tab ──────────────────────────────────────────── */}
        {activeTab === "matrix" && (
          <div className="p-2">
            {/* Summary */}
            <div className="grid grid-cols-3 gap-2 mb-3">
              <div className="p-2 rounded border border-cs-border bg-cs-surface text-center">
                <div className="text-[14px] font-bold text-purple-400">{proCount}</div>
                <div className="text-[8px] text-cs-muted">Pro 深度推理</div>
              </div>
              <div className="p-2 rounded border border-cs-border bg-cs-surface text-center">
                <div className="text-[14px] font-bold text-emerald-400">{flashCount}</div>
                <div className="text-[8px] text-cs-muted">Flash 快速</div>
              </div>
              <div className="p-2 rounded border border-cs-border bg-cs-surface text-center">
                <div className="text-[14px] font-bold text-cs-accent">{uniqueModels.length}</div>
                <div className="text-[8px] text-cs-muted">模型类型</div>
              </div>
            </div>

            {/* Agent-Model mapping table */}
            <div className="text-[9px]">
              <div className="grid grid-cols-[1fr_auto_1fr] gap-1 items-center px-1 py-1 text-cs-muted font-medium border-b border-cs-border">
                <span>{t.ar_agent}</span>
                <span className="text-center">→</span>
                <span className="text-right">{t.ar_model}</span>
              </div>
              {agentMappings.map((m) => (
                <div
                  key={m.agent}
                  className="grid grid-cols-[1fr_auto_1fr] gap-1 items-center px-1 py-0.5 hover:bg-cs-accent/5 rounded transition-colors"
                >
                  <div className="flex items-center space-x-1.5">
                    <span>{m.icon}</span>
                    <span className="text-cs-text truncate">{m.agent}</span>
                  </div>
                  <ArrowRight className="w-3 h-3 text-cs-muted" />
                  <div className="flex items-center justify-end space-x-1.5">
                    <span className="text-cs-dim font-mono text-[8px]">{m.model}</span>
                    <span className={`w-1.5 h-1.5 rounded-full ${m.tier === "pro" ? "bg-purple-400" : "bg-emerald-400"}`} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Bottom bar */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-3 flex items-center text-[8px] text-cs-muted space-x-3">
        <Activity className="w-2.5 h-2.5 text-cs-accent" />
        <span>{t.ar_bottom_bar}</span>
      </div>
    </div>
  );
}