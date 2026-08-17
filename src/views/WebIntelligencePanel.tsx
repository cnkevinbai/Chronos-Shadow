// Web Intelligence Panel — 对外信息搜索抓取与分析
//
// 安全约束：
// - 所有搜索/抓取操作需通过 SecurityBoundary 检查
// - 域名白名单过滤
// - 端侧蒸馏仅喂结论给 LLM
// - 结果去敏 & 审计全记录

import { useState, useCallback, useEffect } from "react";
import {
  Search,
  Globe,
  Download,
  Brain,
  Shield,
  Plus,
  Trash2,
  Clock,
  BarChart3,
  ExternalLink,
  FileText,
  Database,
  Target,
  Zap,
  BookOpen,
  RefreshCw,
  AlertTriangle,
  CheckCircle,
  XCircle,
} from "lucide-react";
import { useT } from "@/lib/i18n-context";
import {
  webIntelSearch,
  webIntelFetch,
  webIntelResearch,
  webIntelAddDomain,
  webIntelRemoveDomain,
  webIntelListDomains,
  webIntelGetAuditLog,
  webIntelGetStats,
  webRerankResults,
  distillEntityRelations,
} from "@/lib/tauri";
import type {
  WebSearchResult,
  WebFetchResult,
  ResearchReport,
  WebAuditEntry,
  WebIntelStats,
} from "@/lib/types";

type Tab = "search" | "fetch" | "research" | "domains" | "audit";

export default function WebIntelligencePanel() {
  const t = useT();
  const [activeTab, setActiveTab] = useState<Tab>("search");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Search
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<WebSearchResult[]>([]);
  const [entityRelations, setEntityRelations] = useState<Array<{ source: string; target: string; relation: string }>>([]);

  // Fetch
  const [fetchUrl, setFetchUrl] = useState("");
  const [fetchResult, setFetchResult] = useState<WebFetchResult | null>(null);
  const [distill, setDistill] = useState(true);

  // Research
  const [researchTopic, setResearchTopic] = useState("");
  const [researchReport, setResearchReport] = useState<ResearchReport | null>(null);

  // Domains
  const [domains, setDomains] = useState<[string, string][]>([]);
  const [newDomain, setNewDomain] = useState("");
  const [newCategory, setNewCategory] = useState("custom");

  // Audit
  const [auditLog, setAuditLog] = useState<WebAuditEntry[]>([]);
  const [stats, setStats] = useState<WebIntelStats | null>(null);

  // Stats refresh
  const refreshStats = useCallback(async () => {
    try {
      const s = await webIntelGetStats();
      setStats(s);
    } catch { /* ignore */ }
  }, []);

  // Load domains on mount
  useEffect(() => {
    webIntelListDomains().then(setDomains).catch(() => {});
    refreshStats();
  }, [refreshStats]);

  // Load audit log
  const loadAuditLog = useCallback(async () => {
    try {
      const log = await webIntelGetAuditLog(30);
      setAuditLog(log);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    if (activeTab === "audit") loadAuditLog();
  }, [activeTab, loadAuditLog]);

  // ─── Handlers ─────────────────────────────────────────────────────

  const handleSearch = async () => {
    if (!query.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const results = await webIntelSearch(query, "bing", 8);
      // v2: 相关性重排序（查询词命中 + 来源权威）
      const reranked = await webRerankResults(query, JSON.stringify(results));
      setSearchResults(reranked.length > 0 ? reranked : results);
      // v2: 实体关系提取
      const content = results.map((r) => `${r.title}. ${r.snippet}`).join("\n");
      distillEntityRelations(content).then(setEntityRelations).catch(() => {});
      refreshStats();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleFetch = async () => {
    if (!fetchUrl.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const result = await webIntelFetch(fetchUrl, distill);
      setFetchResult(result);
      refreshStats();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleResearch = async () => {
    if (!researchTopic.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const report = await webIntelResearch(researchTopic);
      setResearchReport(report);
      refreshStats();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleAddDomain = async () => {
    if (!newDomain.trim()) return;
    try {
      await webIntelAddDomain(newDomain.trim(), newCategory);
      setNewDomain("");
      const updated = await webIntelListDomains();
      setDomains(updated);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemoveDomain = async (domain: string) => {
    try {
      await webIntelRemoveDomain(domain);
      const updated = await webIntelListDomains();
      setDomains(updated);
    } catch (e) {
      setError(String(e));
    }
  };

  // ─── Helpers ──────────────────────────────────────────────────────

  const getResultIcon = (result: string) => {
    switch (result) {
      case "allowed": return <CheckCircle className="w-3 h-3 text-emerald-400" />;
      case "blocked": return <XCircle className="w-3 h-3 text-red-400" />;
      case "error": return <AlertTriangle className="w-3 h-3 text-amber-400" />;
      default: return <Clock className="w-3 h-3 text-cs-muted" />;
    }
  };

  const domainCategories = ["docs", "community", "search", "api", "office", "custom"] as const;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center space-x-2 px-3 py-2.5 border-b border-cs-border">
        <Globe className="w-3.5 h-3 text-cs-accent" />
        <span className="text-[11px] font-bold text-cs-text tracking-wide">
          {t.wi_title}
        </span>
        {stats && (
          <span className="ml-auto text-[9px] text-cs-muted">
            {stats.total_searches + stats.total_fetches + stats.total_research} req · {stats.domains_whitelisted} domains
          </span>
        )}
      </div>

      {/* Tabs */}
      <div className="flex border-b border-cs-border">
        {([
          ["search", Search, t.wi_search],
          ["fetch", Download, t.wi_fetch],
          ["research", Brain, t.wi_research],
          ["domains", Shield, t.wi_domains],
          ["audit", BarChart3, t.wi_audit],
        ] as [Tab, typeof Search, string][]).map(([tab, Icon, label]) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`flex items-center space-x-1 px-3 py-1.5 text-[10px] font-medium transition-colors ${
              activeTab === tab
                ? "text-cs-text border-b border-cs-accent"
                : "text-cs-muted hover:text-cs-dim"
            }`}
          >
            <Icon className="w-3 h-3" />
            <span>{label}</span>
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-3">
        {/* Error banner */}
        {error && (
          <div className="mb-3 p-2 rounded border border-red-800/30 bg-red-950/20 text-red-400 text-[10px] flex items-start space-x-2">
            <AlertTriangle className="w-3.5 h-3.5 mt-0.5 shrink-0" />
            <span>{error}</span>
            <button onClick={() => setError(null)} className="ml-auto text-red-400/60 hover:text-red-400">
              ×
            </button>
          </div>
        )}

        {/* ─── Search Tab ────────────────────────────────────────── */}
        {activeTab === "search" && (
          <div className="space-y-3">
            <div className="flex space-x-2">
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                placeholder="输入搜索关键词..."
                className="flex-1 px-2.5 py-1.5 rounded border border-cs-border bg-cs-bg text-cs-text text-[11px] placeholder:text-cs-muted focus:border-cs-accent focus:outline-none"
              />
              <button
                onClick={handleSearch}
                disabled={loading || !query.trim()}
                className="flex items-center space-x-1 px-3 py-1.5 rounded bg-cs-accent text-white text-[10px] font-medium hover:bg-cs-accent/80 disabled:opacity-40 transition-colors"
              >
                {loading ? <RefreshCw className="w-3 h-3 animate-spin" /> : <Search className="w-3 h-3" />}
                <span>搜索</span>
              </button>
            </div>

            {/* Info */}
            <div className="flex items-center space-x-2 text-[9px] text-cs-muted">
              <Shield className="w-3 h-3 text-cs-accent" />
              <span>搜索引擎: Bing · 域名白名单过滤 · 结果端侧蒸馏</span>
            </div>

            {/* Results */}
            {searchResults.length > 0 && (
              <div className="space-y-2">
                <div className="text-[10px] font-bold text-cs-text">{searchResults.length} 条结果</div>
                {entityRelations.length > 0 && (
                  <div className="text-[8px] text-violet-300/80 font-mono">
                    实体关系: {entityRelations.slice(0, 3).map((rel) => `${rel.source}→${rel.target}`).join(" · ")}
                  </div>
                )}
                {searchResults.map((r, i) => (
                  <a
                    key={i}
                    href={r.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="block p-2 rounded border border-cs-border bg-cs-surface hover:border-cs-dim transition-colors"
                  >
                    <div className="flex items-center space-x-1.5 mb-1">
                      <span className="text-[11px] font-bold text-cs-accent truncate">{r.title}</span>
                      <ExternalLink className="w-3 h-3 text-cs-muted shrink-0" />
                    </div>
                    <div className="text-[9px] text-cs-dim leading-relaxed line-clamp-2">{r.snippet}</div>
                    <div className="flex items-center space-x-2 mt-1">
                      <span className="text-[8px] text-cs-muted truncate max-w-[200px]">{r.url}</span>
                      <span className="text-[8px] px-1 py-0.5 rounded bg-cs-accent/10 text-cs-accent">
                        {r.source}
                      </span>
                    </div>
                  </a>
                ))}
              </div>
            )}

            {searchResults.length === 0 && !loading && (
              <div className="text-center py-8 text-cs-muted text-[10px]">
                输入关键词搜索技术文档、社区问答等
              </div>
            )}
          </div>
        )}

        {/* ─── Fetch Tab ─────────────────────────────────────────── */}
        {activeTab === "fetch" && (
          <div className="space-y-3">
            <div className="flex space-x-2">
              <input
                type="text"
                value={fetchUrl}
                onChange={(e) => setFetchUrl(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleFetch()}
                placeholder="https://..."
                className="flex-1 px-2.5 py-1.5 rounded border border-cs-border bg-cs-bg text-cs-text text-[11px] placeholder:text-cs-muted focus:border-cs-accent focus:outline-none"
              />
              <button
                onClick={handleFetch}
                disabled={loading || !fetchUrl.trim()}
                className="flex items-center space-x-1 px-3 py-1.5 rounded bg-cs-accent text-white text-[10px] font-medium hover:bg-cs-accent/80 disabled:opacity-40 transition-colors"
              >
                {loading ? <RefreshCw className="w-3 h-3 animate-spin" /> : <Download className="w-3 h-3" />}
                <span>抓取</span>
              </button>
            </div>

            {/* Options */}
            <label className="flex items-center space-x-2 text-[10px] text-cs-dim cursor-pointer">
              <input
                type="checkbox"
                checked={distill}
                onChange={(e) => setDistill(e.target.checked)}
                className="rounded"
              />
              <Zap className="w-3 h-3 text-cs-warn" />
              <span>端侧蒸馏（仅保留关键结论）</span>
            </label>

            {/* Result */}
            {fetchResult && (
              <div className="space-y-2">
                {fetchResult.success ? (
                  <>
                    <div className="flex items-center space-x-2 text-[10px]">
                      <CheckCircle className="w-3.5 h-3.5 text-emerald-400" />
                      <span className="font-bold text-cs-text">{fetchResult.title || fetchResult.url}</span>
                      <span className="text-cs-muted">{fetchResult.content_length} chars</span>
                    </div>
                    {fetchResult.distilled && fetchResult.distilled_summary && (
                      <div className="p-2 rounded border border-amber-800/30 bg-amber-950/20">
                        <div className="flex items-center space-x-1.5 mb-1.5">
                          <Zap className="w-3 h-3 text-amber-400" />
                          <span className="text-[9px] font-bold text-amber-400">端侧蒸馏摘要</span>
                        </div>
                        <div className="text-[10px] text-amber-300/80">{fetchResult.distilled_summary}</div>
                        {fetchResult.key_points.length > 0 && (
                          <ul className="mt-1.5 space-y-0.5">
                            {fetchResult.key_points.map((kp, i) => (
                              <li key={i} className="text-[9px] text-amber-400/70 flex items-start space-x-1">
                                <span>•</span>
                                <span>{kp}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    )}
                    {!fetchResult.distilled && (
                      <div className="p-2 rounded border border-cs-border bg-cs-bg max-h-60 overflow-y-auto">
                        <pre className="text-[10px] text-cs-dim font-mono whitespace-pre-wrap">{fetchResult.content}</pre>
                      </div>
                    )}
                  </>
                ) : (
                  <div className="p-2 rounded border border-red-800/30 bg-red-950/20 text-red-400 text-[10px]">
                    {fetchResult.error || "抓取失败"}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* ─── Research Tab ──────────────────────────────────────── */}
        {activeTab === "research" && (
          <div className="space-y-3">
            <div className="flex space-x-2">
              <input
                type="text"
                value={researchTopic}
                onChange={(e) => setResearchTopic(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleResearch()}
                placeholder="输入研究主题，如 'Rust async runtime 对比'"
                className="flex-1 px-2.5 py-1.5 rounded border border-cs-border bg-cs-bg text-cs-text text-[11px] placeholder:text-cs-muted focus:border-cs-accent focus:outline-none"
              />
              <button
                onClick={handleResearch}
                disabled={loading || !researchTopic.trim()}
                className="flex items-center space-x-1 px-3 py-1.5 rounded bg-cs-accent text-white text-[10px] font-medium hover:bg-cs-accent/80 disabled:opacity-40 transition-colors"
              >
                {loading ? <RefreshCw className="w-3 h-3 animate-spin" /> : <Brain className="w-3 h-3" />}
                <span>研究</span>
              </button>
            </div>

            <div className="flex items-center space-x-2 text-[9px] text-cs-muted">
              <Shield className="w-3 h-3 text-cs-accent" />
              <span>多源聚合 · 交叉验证 · 端侧蒸馏 · 置信度评分</span>
            </div>

            {researchReport && (
              <div className="space-y-2">
                {/* Header */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <BookOpen className="w-3.5 h-3.5 text-cs-info" />
                    <span className="text-[11px] font-bold text-cs-text">{researchReport.topic}</span>
                  </div>
                  <span className={`text-[9px] px-1.5 py-0.5 rounded ${
                    researchReport.confidence >= 0.7 ? "bg-emerald-950/30 text-emerald-400" :
                    researchReport.confidence >= 0.4 ? "bg-amber-950/30 text-amber-400" :
                    "bg-red-950/30 text-red-400"
                  }`}>
                    置信度: {(researchReport.confidence * 100).toFixed(0)}%
                  </span>
                </div>

                {/* Summary */}
                <div className="p-2 rounded border border-cs-border bg-cs-surface">
                  <div className="text-[10px] text-cs-dim whitespace-pre-wrap">{researchReport.summary}</div>
                </div>

                {/* Key Findings */}
                {researchReport.key_findings.length > 0 && (
                  <div>
                    <div className="text-[10px] font-bold text-cs-text mb-1.5">核心发现</div>
                    <ul className="space-y-1">
                      {researchReport.key_findings.map((kf, i) => (
                        <li key={i} className="flex items-start space-x-1.5 text-[9px] text-cs-dim">
                          <CheckCircle className="w-3 h-3 text-emerald-400 mt-0.5 shrink-0" />
                          <span>{kf}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {/* Recommendations */}
                {researchReport.recommendations.length > 0 && (
                  <div>
                    <div className="text-[10px] font-bold text-cs-text mb-1.5">建议</div>
                    <ul className="space-y-1">
                      {researchReport.recommendations.map((rec, i) => (
                        <li key={i} className="flex items-start space-x-1.5 text-[9px] text-cs-dim">
                          <Zap className="w-3 h-3 text-amber-400 mt-0.5 shrink-0" />
                          <span>{rec}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {/* Sources */}
                {researchReport.sources.length > 0 && (
                  <div>
                    <div className="text-[10px] font-bold text-cs-text mb-1.5">
                      参考来源 ({researchReport.sources.length})
                    </div>
                    <div className="space-y-1">
                      {researchReport.sources.map((s, i) => (
                        <a
                          key={i}
                          href={s.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="flex items-center space-x-1.5 text-[9px] text-cs-accent hover:text-cs-accent/80"
                        >
                          <ExternalLink className="w-2.5 h-2.5" />
                          <span className="truncate">{s.title}</span>
                        </a>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* ─── Domains Tab ───────────────────────────────────────── */}
        {activeTab === "domains" && (
          <div className="space-y-3">
            {/* Add domain */}
            <div className="flex space-x-1.5">
              <input
                type="text"
                value={newDomain}
                onChange={(e) => setNewDomain(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleAddDomain()}
                placeholder="docs.rs"
                className="flex-1 px-2 py-1 rounded border border-cs-border bg-cs-bg text-cs-text text-[10px] placeholder:text-cs-muted focus:border-cs-accent focus:outline-none"
              />
              <select
                value={newCategory}
                onChange={(e) => setNewCategory(e.target.value)}
                className="px-1.5 py-1 rounded border border-cs-border bg-cs-bg text-cs-text text-[9px] focus:border-cs-accent focus:outline-none"
              >
                {domainCategories.map((c) => (
                  <option key={c} value={c}>{c}</option>
                ))}
              </select>
              <button
                onClick={handleAddDomain}
                disabled={!newDomain.trim()}
                className="flex items-center space-x-1 px-2 py-1 rounded bg-cs-accent text-white text-[9px] font-medium hover:bg-cs-accent/80 disabled:opacity-40 transition-colors"
              >
                <Plus className="w-3 h-3" />
              </button>
            </div>

            <div className="text-[9px] text-cs-muted flex items-center space-x-1.5">
              <Shield className="w-3 h-3 text-cs-accent" />
              <span>仅白名单内域名可被访问 · 首次访问需审批 · 全量审计</span>
            </div>

            {/* Domain list */}
            <div className="space-y-1">
              {domains.length === 0 && (
                <div className="text-center py-6 text-cs-muted text-[10px]">暂无白名单域名</div>
              )}
              {domains.map(([domain, category]) => (
                <div key={domain} className="flex items-center justify-between p-2 rounded border border-cs-border bg-cs-surface">
                  <div className="flex items-center space-x-2">
                    <Globe className="w-3 h-3 text-cs-accent" />
                    <span className="text-[10px] text-cs-text font-medium">{domain}</span>
                    <span className="text-[8px] px-1 py-0.5 rounded bg-cs-bg text-cs-muted">{category}</span>
                  </div>
                  <button
                    onClick={() => handleRemoveDomain(domain)}
                    className="p-1 rounded text-cs-muted hover:text-red-400 hover:bg-red-950/20 transition-colors"
                  >
                    <Trash2 className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ─── Audit Tab ─────────────────────────────────────────── */}
        {activeTab === "audit" && (
          <div className="space-y-3">
            {/* Stats summary */}
            {stats && (
              <div className="grid grid-cols-3 gap-2">
                <StatBox icon={<Search className="w-3 h-3" />} label={t.wi_stats_searches} value={stats.total_searches} />
                <StatBox icon={<Download className="w-3 h-3" />} label={t.wi_stats_fetches} value={stats.total_fetches} />
                <StatBox icon={<Brain className="w-3 h-3" />} label={t.wi_stats_research} value={stats.total_research} />
                <StatBox icon={<Globe className="w-3 h-3" />} label={t.wi_stats_domains} value={stats.domains_whitelisted} />
                <StatBox icon={<Shield className="w-3 h-3" />} label={t.wi_stats_blocked} value={stats.requests_blocked} color="red" />
                <StatBox icon={<Zap className="w-3 h-3" />} label={t.wi_stats_distilled} value={stats.total_distilled || 0} />
                <StatBox icon={<FileText className="w-3 h-3" />} label={t.wi_stats_compression} value={`${((stats.avg_compression_ratio || 0) * 100).toFixed(0)}%`} />
                <StatBox icon={<Database className="w-3 h-3" />} label={t.wi_stats_traffic_saved} value={formatBytes(stats.total_bytes_saved || 0)} />
                <StatBox icon={<Target className="w-3 h-3" />} label={t.wi_stats_cache_hit} value={`${((stats.cache_hit_rate || 0) * 100).toFixed(0)}%`} />
                <StatBox icon={<Zap className="w-3 h-3" />} label={t.wi_stats_unified_hits} value={stats.unified_cache_hits || 0} />
                <StatBox icon={<BarChart3 className="w-3 h-3" />} label={t.wi_stats_api_saved} value={stats.api_calls_saved || 0} color="green" />
                <StatBox icon={<Database className="w-3 h-3" />} label={t.wi_stats_unified_rate} value={`${((stats.unified_cache_hits || 0) / ((stats.unified_cache_hits || 0) + (stats.unified_cache_misses || 0) + 1) * 100).toFixed(0)}%`} />
              </div>
            )}

            {/* Audit log */}
            <div>
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-[10px] font-bold text-cs-text">审计日志</span>
                <button onClick={loadAuditLog} className="p-1 rounded text-cs-muted hover:text-cs-dim transition-colors">
                  <RefreshCw className="w-3 h-3" />
                </button>
              </div>
              <div className="space-y-1 max-h-80 overflow-y-auto">
                {auditLog.length === 0 && (
                  <div className="text-center py-4 text-cs-muted text-[9px]">暂无审计记录</div>
                )}
                {auditLog.map((entry, i) => (
                  <div
                    key={i}
                    className={`flex items-center space-x-2 p-1.5 rounded text-[9px] ${
                      entry.domain_allowed ? "bg-emerald-950/10" : "bg-red-950/10"
                    }`}
                  >
                    {getResultIcon(entry.result)}
                    <span className="text-cs-dim font-medium w-14 shrink-0">{entry.request_type}</span>
                    <span className="text-cs-muted truncate flex-1">{entry.target}</span>
                    <span className="text-cs-muted shrink-0">{entry.bytes_received}B</span>
                    <span className="text-cs-muted shrink-0">{entry.duration_ms}ms</span>
                    <span className="text-cs-muted shrink-0 w-16 text-right">
                      {new Date(entry.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Bottom status bar */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-3 flex items-center text-[8px] text-cs-muted space-x-3">
        <Shield className="w-2.5 h-2.5 text-cs-accent" />
        <span>{t.wi_security_notice}</span>
      </div>
    </div>
  );
}

// ─── Mini components ────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function StatBox({
  icon,
  label,
  value,
  color = "accent",
}: {
  icon: React.ReactNode;
  label: string;
  value: number | string;
  color?: string;
}) {
  const colorMap: Record<string, string> = {
    accent: "text-cs-accent",
    red: "text-red-400",
    green: "text-emerald-400",
    amber: "text-amber-400",
  };
  return (
    <div className="flex items-center space-x-1.5 p-2 rounded border border-cs-border bg-cs-surface">
      <span className={colorMap[color] || "text-cs-accent"}>{icon}</span>
      <div>
        <div className="text-[10px] font-bold text-cs-text">{value}</div>
        <div className="text-[8px] text-cs-muted">{label}</div>
      </div>
    </div>
  );
}
