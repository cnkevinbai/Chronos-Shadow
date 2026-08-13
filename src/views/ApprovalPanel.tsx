// ApprovalPanel.tsx — 第四红线：人类审批门禁仪表盘 v2
// 字段名已对齐 Rust ApprovalRequest 序列化 (risk_level / submitted_at / decided_by / decision_comment)

import { useState, useEffect, useCallback } from "react";
import {
  listPendingApprovals,
  getApprovalAuditLog,
  getApprovalRules,
  getApprovalSuggestions,
  submitForApproval,
  submitForApprovalWithCost,
  decideApproval,
  addApprovalRule,
  removeApprovalRule,
  saveApprovalState,
} from "@/lib/tauri";
import type { ApprovalRequest, ApprovalRule, ApprovalSuggestion } from "@/lib/types";
import {
  Shield, Check, X, Clock, History, Plus, Trash2, AlertTriangle,
  Sparkles, DollarSign, GitMerge, Zap, Settings,
} from "lucide-react";

const RISK_GRADIENT: Record<number, string> = {
  1: "bg-emerald-950/60 border-emerald-500/40 text-emerald-400",
  2: "bg-emerald-950/50 border-emerald-500/30 text-emerald-400",
  3: "bg-lime-950/50 border-lime-500/30 text-lime-400",
  4: "bg-yellow-950/50 border-yellow-500/30 text-yellow-400",
  5: "bg-amber-950/50 border-amber-500/30 text-amber-400",
  6: "bg-orange-950/50 border-orange-500/30 text-orange-400",
  7: "bg-red-950/50 border-red-500/30 text-red-400",
  8: "bg-red-950/60 border-red-500/40 text-red-400",
  9: "bg-red-950/70 border-red-500/50 text-red-500",
  10: "bg-red-950/80 border-red-500/60 text-red-500 font-bold",
};

const RISK_LABEL: Record<number, string> = {
  1: "极低", 2: "低", 3: "较低", 4: "中低", 5: "中",
  6: "中高", 7: "高", 8: "较高", 9: "严重", 10: "致命",
};

const ACTION_ICON: Record<string, React.ReactNode> = {
  WorktreeMerge: <GitMerge className="w-3 h-3" />,
  PipelineAdvance: <Zap className="w-3 h-3" />,
  RemoteCommand: <span className="text-[9px]">&gt;_</span>,
  CostThreshold: <DollarSign className="w-3 h-3" />,
  FileDelete: <Trash2 className="w-3 h-3" />,
};
void ACTION_ICON;

const ACTION_LABEL: Record<string, string> = {
  WorktreeMerge: "Worktree 合并",
  PipelineAdvance: "流水线跃迁",
  RemoteCommand: "远程命令",
  CostThreshold: "资费超限",
  FileDelete: "文件删除",
};

export default function ApprovalPanel() {
  const [pending, setPending] = useState<ApprovalRequest[]>([]);
  const [auditLog, setAuditLog] = useState<ApprovalRequest[]>([]);
  const [rules, setRules] = useState<ApprovalRule[]>([]);
  const [suggestions, setSuggestions] = useState<ApprovalSuggestion[]>([]);
  const [view, setView] = useState<"pending" | "log" | "rules" | "suggest">("pending");
  const [comment, setComment] = useState("");
  const [expandedReq, setExpandedReq] = useState<string | null>(null);

  // ── 提交新审批表单 ────────────────────────────────────────────
  const [showSubmitForm, setShowSubmitForm] = useState(false);
  const [submitAction, setSubmitAction] = useState("worktree_merge");
  const [submitTarget, setSubmitTarget] = useState("");
  const [submitDesc, setSubmitDesc] = useState("");
  const [submitCost, setSubmitCost] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [p, l, r, s] = await Promise.all([
        listPendingApprovals(),
        getApprovalAuditLog(30),
        getApprovalRules(),
        getApprovalSuggestions(),
      ]);
      setPending(p);
      setAuditLog(l);
      setRules(r);
      setSuggestions(s);
    } catch { /* Tauri offline */ }
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  }, [refresh]);

  const handleSubmitApproval = async () => {
    if (!submitTarget.trim() || !submitDesc.trim()) return;
    try {
      const cost = parseFloat(submitCost);
      if (!isNaN(cost) && cost > 0) {
        await submitForApprovalWithCost(submitAction, submitTarget, submitDesc, "{}", cost);
      } else {
        await submitForApproval(submitAction, submitTarget, submitDesc, "{}");
      }
      setSubmitTarget(""); setSubmitDesc(""); setSubmitCost("");
      setShowSubmitForm(false);
      await saveApprovalState();
      refresh();
    } catch (e) { console.error(e); }
  };

  const handleDecide = async (reqId: string, decision: string) => {
    try {
      await decideApproval(reqId, decision, "Operator", comment || "—");
      setComment("");
      await saveApprovalState();
      refresh();
    } catch (e) { console.error(e); }
  };

  const handleAddRule = async () => {
    const at = prompt("操作类型 (worktree_merge / pipeline_advance / ssh_exec / cost_override / file_delete):");
    if (!at) return;
    const riskStr = prompt("风险等级 (1-10):", "5");
    if (!riskStr) return;
    const autoStr = prompt("自动放行阈值 (低于此风险自动通过):", "3");
    if (!autoStr) return;
    const desc = prompt("规则描述:", ACTION_LABEL[at] ?? at) || at;
    try {
      await addApprovalRule(at, parseInt(riskStr), parseInt(autoStr), desc);
      await saveApprovalState();
      refresh();
    } catch (e) { console.error(e); }
  };

  const handleRemoveRule = async (ruleId: string) => {
    try { await removeApprovalRule(ruleId); await saveApprovalState(); refresh(); }
    catch (e) { console.error(e); }
  };

  const pendingCount = pending.length;
  const highRiskCount = pending.filter(r => r.risk_level >= 7).length;

  return (
    <div className="flex flex-col h-full bg-cs-surface text-[11px]">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-cs-border bg-cs-header shrink-0">
        <div className="flex items-center space-x-1.5">
          <Shield className={`w-3 h-3 ${highRiskCount > 0 ? "text-red-400 animate-pulse" : "text-red-400"}`} />
          <span className="font-bold text-zinc-200">第四红线</span>
          {pendingCount > 0 && (
            <span className="px-1.5 py-0.5 text-[9px] bg-red-500/20 text-red-400 border border-red-500/30 rounded-full font-bold">
              {pendingCount}
            </span>
          )}
          {highRiskCount > 0 && <AlertTriangle className="w-3 h-3 text-red-500 animate-pulse" />}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-cs-border bg-cs-surface shrink-0">
        {([
          { id: "pending" as const, icon: Clock, label: "待审批" },
          { id: "log" as const, icon: History, label: "审计" },
          { id: "rules" as const, icon: Shield, label: "规则" },
          { id: "suggest" as const, icon: Sparkles, label: "建议" },
        ]).map(tab => {
          const Icon = tab.icon;
          const active = view === tab.id;
          return (
            <button key={tab.id} onClick={() => setView(tab.id)}
              className={`flex-1 flex items-center justify-center space-x-1 py-1.5 text-[10px] transition-colors ${
                active ? "text-white bg-cs-header border-b border-red-400" : "text-zinc-500 hover:text-zinc-300"
              }`}>
              <Icon className={`w-3 h-3 ${active ? "text-red-400" : ""}`} />
              <span>{tab.label}</span>
              {tab.id === "pending" && pendingCount > 0 && <span className="text-[9px] text-red-400">({pendingCount})</span>}
              {tab.id === "suggest" && suggestions.length > 0 && <span className="text-[9px] text-amber-400">({suggestions.length})</span>}
            </button>
          );
        })}
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {/* ── 提交审批表单 ── */}
        <button onClick={() => setShowSubmitForm(!showSubmitForm)}
          className="w-full flex items-center justify-center space-x-1 py-1.5 border border-dashed border-red-500/30 rounded text-[10px] text-red-400 hover:bg-red-950/20 transition-colors">
          <Shield className="w-3 h-3" />
          <span>{showSubmitForm ? "收起" : "提交审批请求"}</span>
        </button>
        {showSubmitForm && (
          <div className="p-2 border border-red-500/20 rounded bg-red-950/5 space-y-1.5">
            <select value={submitAction} onChange={e => setSubmitAction(e.target.value)}
              className="w-full bg-cs-bg border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-200">
              <option value="worktree_merge">Worktree 合并</option>
              <option value="pipeline_advance">流水线跃迁</option>
              <option value="ssh_exec">远程命令</option>
              <option value="cost_override">资费超限</option>
              <option value="file_delete">文件删除</option>
              <option value="config_change">配置变更</option>
            </select>
            <input type="text" placeholder="目标 ID (如 wt-0001)" value={submitTarget}
              onChange={e => setSubmitTarget(e.target.value)}
              className="w-full bg-cs-bg border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-200 placeholder-zinc-600" />
            <input type="text" placeholder="描述 (如: 合并 feature-x 到 main)" value={submitDesc}
              onChange={e => setSubmitDesc(e.target.value)}
              className="w-full bg-cs-bg border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-200 placeholder-zinc-600" />
            <div className="flex space-x-1">
              <input type="number" placeholder="预估费用 ¥ (可选)" value={submitCost}
                onChange={e => setSubmitCost(e.target.value)}
                className="flex-1 bg-cs-bg border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-200 placeholder-zinc-600" />
              <button onClick={handleSubmitApproval}
                className="px-3 py-1 bg-red-950/60 border border-red-500/40 text-red-400 rounded text-[10px] hover:bg-red-900/60">
                提交审批
              </button>
            </div>
          </div>
        )}

        {/* ── 待审批 ── */}
        {view === "pending" && (
          pending.length === 0 ? (
            <div className="text-center text-zinc-600 py-10">
              <Check className="w-5 h-5 mx-auto mb-2 text-emerald-600" />
              <span className="text-[11px]">所有操作已放行</span>
            </div>
          ) : (
            pending.map(req => {
              const expanded = expandedReq === req.id;
              return (
                <div key={req.id}
                  className={`p-2.5 border rounded bg-cs-header space-y-2 cursor-pointer ${
                    req.risk_level >= 7 ? "border-red-900/60" : "border-cs-border"
                  }`}
                  onClick={() => setExpandedReq(expanded ? null : req.id)}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center space-x-1.5 mb-1">
                        <span className={`text-[8px] px-1 py-0.5 rounded border font-bold ${RISK_GRADIENT[req.risk_level] ?? "text-zinc-400 bg-zinc-950 border-zinc-700"}`}>
                          R{req.risk_level} {RISK_LABEL[req.risk_level] ?? ""}
                        </span>
                        <span className="text-[9px] text-zinc-500">
                          {ACTION_LABEL[req.action_type] ?? req.action_type}
                        </span>
                      </div>
                      <div className="w-full h-1 bg-[#27272a] rounded-full mb-1.5">
                        <div className={`h-1 rounded-full ${
                          req.risk_level <= 3 ? "bg-emerald-500" :
                          req.risk_level <= 5 ? "bg-yellow-500" :
                          req.risk_level <= 7 ? "bg-orange-500" : "bg-red-500"
                        }`} style={{ width: `${req.risk_level * 10}%` }} />
                      </div>
                      <p className="text-zinc-300 text-[10px] truncate">{req.description}</p>
                      <div className="flex items-center space-x-2 mt-1 text-[9px] text-zinc-600">
                        <span>{req.id}</span>
                        <span>·</span>
                        <span>{new Date(req.submitted_at).toLocaleTimeString()}</span>
                        {req.estimated_cost != null && (
                          <>
                            <span>·</span>
                            <DollarSign className="w-2.5 h-2.5" />
                            <span>¥{req.estimated_cost.toFixed(2)}</span>
                          </>
                        )}
                      </div>
                    </div>
                    {req.risk_level >= 7 && <AlertTriangle className="w-3.5 h-3.5 text-red-400 shrink-0 ml-2 animate-pulse" />}
                  </div>

                  {expanded && (
                    <>
                      <div className="text-[9px] text-zinc-600 space-y-0.5 border-t border-[#1a1a1e] pt-1.5">
                        <div className="flex space-x-1"><span className="text-zinc-500">target:</span><span className="text-zinc-400">{req.target_id}</span></div>
                        {req.metadata && <div className="flex space-x-1"><span className="text-zinc-500">meta:</span><span className="text-zinc-400 truncate">{req.metadata}</span></div>}
                        {req.auditor_prescreen && (
                          <div className={`text-[9px] ${req.auditor_prescreen.passed ? "text-emerald-500" : "text-red-500"}`}>
                            Auditor: {req.auditor_prescreen.passed ? "✅ 通过" : "❌ 不通过"} · {req.auditor_prescreen.summary}
                          </div>
                        )}
                      </div>
                      <div className="flex items-center space-x-2 pt-1 border-t border-[#1a1a1e]">
                        <input type="text" placeholder="审批备注" value={comment}
                          onChange={e => setComment(e.target.value)}
                          onClick={e => e.stopPropagation()}
                          className="flex-1 bg-cs-bg border border-cs-border rounded px-2 py-1 text-[10px] text-zinc-300 placeholder-zinc-600" />
                        <button onClick={e => { e.stopPropagation(); handleDecide(req.id, "Approve"); }}
                          className="px-2.5 py-1 bg-emerald-950/60 border border-emerald-500/40 text-emerald-400 rounded text-[10px] hover:bg-emerald-900/60">
                          <Check className="w-3 h-3 inline mr-0.5" />通过</button>
                        <button onClick={e => { e.stopPropagation(); handleDecide(req.id, "Reject"); }}
                          className="px-2.5 py-1 bg-red-950/60 border border-red-500/40 text-red-400 rounded text-[10px] hover:bg-red-900/60">
                          <X className="w-3 h-3 inline mr-0.5" />驳回</button>
                      </div>
                    </>
                  )}
                </div>
              );
            })
          )
        )}

        {/* ── 审计日志 ── */}
        {view === "log" && (
          auditLog.length === 0 ? (
            <div className="text-center text-zinc-600 py-10"><History className="w-5 h-5 mx-auto mb-2" /><span>暂无审计记录</span></div>
          ) : (
            auditLog.map(req => {
              const isOk = req.status === "Approved" || req.status === "AutoApproved";
              const isBad = req.status === "Rejected";
              return (
                <div key={req.id} className={`p-2 border rounded text-[10px] ${
                  isOk ? "border-emerald-900/30 bg-emerald-950/5" :
                  isBad ? "border-red-900/30 bg-red-950/5" : "border-cs-border bg-cs-header"
                }`}>
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-zinc-400 font-bold">{ACTION_LABEL[req.action_type] ?? req.action_type}</span>
                    <span className={`text-[9px] px-1 py-0.5 rounded ${
                      req.status === "AutoApproved" ? "text-emerald-400 bg-emerald-950/30" :
                      req.status === "Approved" ? "text-emerald-400 bg-emerald-950/30" :
                      req.status === "Rejected" ? "text-red-400 bg-red-950/30" :
                      req.status === "Expired" ? "text-zinc-500 bg-zinc-950/30" :
                      "text-yellow-400 bg-yellow-950/30"
                    }`}>
                      {req.status === "AutoApproved" ? "自动" : req.status === "Approved" ? "通过" :
                       req.status === "Rejected" ? "驳回" : req.status === "Expired" ? "过期" : req.status}
                    </span>
                  </div>
                  <p className="text-zinc-500 truncate">{req.description}</p>
                  <div className="flex justify-between mt-1 text-[9px] text-zinc-600">
                    <span>{req.decided_by ?? "—"}{req.decision_comment ? ` · ${req.decision_comment}` : ""}</span>
                    <span>{req.decided_at ? new Date(req.decided_at).toLocaleTimeString() : ""}</span>
                  </div>
                </div>
              );
            })
          )
        )}

        {/* ── 规则管理 ── */}
        {view === "rules" && (
          <>
            <button onClick={handleAddRule}
              className="w-full flex items-center justify-center space-x-1 py-1.5 border border-dashed border-cs-border rounded text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors">
              <Plus className="w-3 h-3" /><span>添加规则</span>
            </button>
            {rules.map(rule => (
              <div key={rule.id} className={`p-2 border rounded text-[10px] ${
                rule.enabled ? "border-cs-border bg-cs-header" : "border-[#1a1a1e] bg-[#0a0a0c] opacity-60"
              }`}>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-zinc-300 font-bold">{rule.name}</span>
                  <button onClick={() => handleRemoveRule(rule.id)}
                    className="text-zinc-600 hover:text-red-400 transition-colors">
                    <Trash2 className="w-3 h-3" />
                  </button>
                </div>
                <div className="flex flex-wrap gap-x-2 text-[9px] text-zinc-500">
                  <span>阈值 ≤{rule.auto_approve_below_risk}</span>
                  <span>·</span>
                  <span>超时 {rule.timeout_secs}s</span>
                  {rule.project_scope && <><span>·</span><span className="text-cyan-500">🏷️ {rule.project_scope}</span></>}
                  {rule.enable_auditor_prescreen && <><span>·</span><span className="text-amber-500">🔍 Auditor预检</span></>}
                </div>
              </div>
            ))}
          </>
        )}

        {/* ── 演化建议 ── */}
        {view === "suggest" && (
          suggestions.length === 0 ? (
            <div className="text-center text-zinc-600 py-10">
              <Sparkles className="w-5 h-5 mx-auto mb-2" />
              <span className="text-[11px]">暂无优化建议<br/>积累更多审批数据后自动生成</span>
            </div>
          ) : (
            suggestions.map((s, i) => (
              <div key={i} className="p-2.5 border border-amber-900/30 rounded bg-amber-950/5 space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-amber-400 font-bold text-[10px] flex items-center space-x-1">
                    <Sparkles className="w-3 h-3" /><span>{s.rule_name}</span>
                  </span>
                  <span className="text-[9px] text-amber-600">{Math.round(s.confidence * 100)}% 置信</span>
                </div>
                <p className="text-zinc-400 text-[10px]">{s.reason}</p>
                <div className="flex items-center space-x-1 text-[9px] text-zinc-500">
                  <span>阈值 {s.current_threshold} → {s.suggested_threshold}</span>
                </div>
              </div>
            ))
          )
        )}
      </div>

      <div className="px-3 py-1.5 border-t border-cs-border bg-cs-header text-[9px] text-zinc-600 flex items-center justify-between shrink-0">
        <div className="flex items-center space-x-3">
          <Settings className="w-2.5 h-2.5" />
          <span>{rules.length} 规则</span>
          <span>·</span>
          <span>{auditLog.length} 审计</span>
        </div>
        <span className={pendingCount > 0 ? "text-red-400" : "text-zinc-600"}>
          {pendingCount > 0 ? `${pendingCount} 待处理` : "全部放行"}
        </span>
      </div>
    </div>
  );
}
