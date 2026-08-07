import { useState } from "react";
import { useT } from "@/lib/i18n-context";
import { validateModelOutput, resetFuse } from "@/lib/tauri";
import {
  ShieldAlert,
  Eye,
  EyeOff,
  ScanLine,
  Redo,
  AlertTriangle,
  XCircle,
  CheckCircle2,
  Clock,
  FlaskConical,
} from "lucide-react";

interface TimelineEvent {
  id: number;
  time: string;
  action: string;
  detail: string;
  status: "ok" | "warn" | "fail" | "info";
}

function buildTimeline(rs: import("@/lib/types").RedlineStatus | null | undefined): TimelineEvent[] {
  const now = new Date().toLocaleTimeString();
  const events: TimelineEvent[] = [];
  let id = 0;

  if (rs) {
    if (rs.schema_active) {
      events.push({ id: ++id, time: now, action: "Schema 强校验", detail: `serde_json 反序列化器运行中${rs.schema_last_check ? " · 上次: " + rs.schema_last_check : ""}`, status: "ok" });
    }
    if (rs.sandbox_active) {
      events.push({ id: ++id, time: now, action: "文件沙盒白名单", detail: `Scope: ${rs.sandbox_root} · 已拦截 ${rs.blocked_paths} 路径`, status: "ok" });
    }
    if (rs.healing_enabled && rs.current_loop > 0) {
      events.push({ id: ++id, time: now, action: `自愈循环 #${rs.current_loop}`, detail: `已触发自愈 · 剩余 ${rs.max_loop - rs.current_loop} 次`, status: rs.fused ? "fail" : "warn" });
    }
    if (rs.fused) {
      events.push({ id: ++id, time: now, action: "熔断触发", detail: `自愈已达上限 ${rs.max_loop}/${rs.max_loop} · 已挂起`, status: "fail" });
    }
    if (rs.blocked_paths > 0) {
      events.push({ id: ++id, time: now, action: "路径拦截", detail: `${rs.blocked_paths} 次越权路径已被拦截`, status: "warn" });
    }
  }

  // always show baseline events
  events.push({ id: ++id, time: now, action: "AST Diff 审计", detail: "TypeScript/TSX 实时监控中", status: "ok" });
  events.push({ id: ++id, time: now, action: "0 Token Blocker", detail: "屏幕差分检测运行中", status: "ok" });
  events.push({ id: ++id, time: now, action: "CV 隐私遮罩", detail: "端侧模型就绪 · 实时打码", status: "ok" });

  return events;
}

interface RedlineGuardPanelProps {
  redlineStatus?: import("@/lib/types").RedlineStatus | null;
}

export default function RedlineGuardPanel({ redlineStatus }: RedlineGuardPanelProps) {
  const _t = useT(); void _t;
  const t = _t;
  const [privacyEnabled, setPrivacyEnabled] = useState(true);
  const [selectedRegion, setSelectedRegion] = useState<string | null>(null);

  const privacyRegions = [
    { id: "chat", label: t.privacy_region_chat, active: true },
    { id: "password", label: t.privacy_region_password, active: true },
    { id: "banking", label: t.privacy_region_banking, active: true },
    { id: "custom", label: t.privacy_region_custom, active: false },
  ];

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-cs-border">
        <div className="flex items-center space-x-2">
          <ShieldAlert className="w-3.5 h-3.5 text-cs-danger" />
          <span className="text-[11px] font-bold text-cs-text tracking-wide">
            {t.redline_guard}
          </span>
        </div>
        <div className="flex items-center space-x-1">
          <span className="text-[9px] text-cs-muted">Privacy</span>
          <button
            onClick={() => setPrivacyEnabled(!privacyEnabled)}
            className={`w-7 h-4 rounded-full p-0.5 transition-colors ${
              privacyEnabled ? "bg-cs-accent" : "bg-cs-border"
            }`}
          >
            <div
              className={`w-3 h-3 rounded-full bg-white transition-transform ${
                privacyEnabled ? "translate-x-3" : "translate-x-0"
              }`}
            />
          </button>
        </div>
      </div>

      {/* Privacy Overlay Preview */}
      <div className="px-3 py-2.5 border-b border-cs-border/50">
        <div className="text-[9px] text-cs-muted mb-2 flex items-center space-x-1.5">
          {privacyEnabled ? (
            <EyeOff className="w-2.5 h-2.5 text-cs-accent" />
          ) : (
            <Eye className="w-2.5 h-2.5 text-cs-dim" />
          )}
          <span>CV 隐私遮罩区域</span>
        </div>
        {/* Mini desktop preview */}
        <div className="relative w-full h-20 rounded border border-cs-border bg-cs-bg overflow-hidden">
          {/* Simulated windows */}
          <div className="absolute top-2 left-2 right-2 h-12 rounded bg-cs-header border border-cs-border/50" />
          {/* Privacy blur overlay */}
          {privacyEnabled && (
            <>
              <div className="absolute top-3 right-6 w-16 h-8 rounded bg-cs-danger/20 border border-cs-danger/40 flex items-center justify-center">
                <span className="text-[7px] text-cs-danger">*** BLUR ***</span>
              </div>
              <div className="absolute bottom-3 left-4 w-12 h-6 rounded bg-cs-danger/20 border border-cs-danger/40 flex items-center justify-center">
                <span className="text-[7px] text-cs-danger">****</span>
              </div>
            </>
          )}
          {/* Scan line effect */}
          <div className="absolute inset-0 pointer-events-none overflow-hidden opacity-5">
            <ScanLine className="w-full h-full" />
          </div>
        </div>

        {/* Privacy region toggles */}
        <div className="flex flex-wrap gap-1.5 mt-2">
          {privacyRegions.map((region) => (
            <button
              key={region.id}
              onClick={() => setSelectedRegion(region.id)}
              className={`px-2 py-0.5 rounded text-[8px] border transition-colors ${
                region.active
                  ? selectedRegion === region.id
                    ? "border-cs-danger text-cs-danger bg-cs-danger/10"
                    : "border-cs-danger/30 text-cs-danger/70"
                  : "border-cs-border text-cs-muted"
              }`}
            >
              {region.label}
            </button>
          ))}
          <button className="px-2 py-0.5 rounded text-[8px] border border-cs-border text-cs-muted hover:border-cs-dim transition-colors">
            {t.add_mask_region}
          </button>
        </div>
      </div>

      {/* Redline 操作按钮 */}
      <div className="px-3 py-1.5 border-b border-cs-border/50 flex items-center space-x-1.5">
        <button
          onClick={async () => {
            try {
              const result = await validateModelOutput('{"action":"file_read","params":{"path":"src/main.tsx"}}');
              alert(`校验通过: ${result}`);
            } catch (e) { alert(`校验拦截: ${e}`); }
          }}
          className="flex items-center space-x-1 text-[8px] bg-purple-950/30 border border-purple-800/30 text-purple-400 hover:bg-purple-900/40 px-2 py-0.5 rounded transition-colors">
          <FlaskConical className="w-2.5 h-2.5" /> 测试校验
        </button>
        {redlineStatus?.fused && (
          <button
            onClick={async () => {
              try { await resetFuse(); alert("熔断器已重置。"); } catch(e) { alert(`失败: ${e}`); }
            }}
            className="text-[8px] bg-red-950/30 border border-red-800/30 text-red-400 hover:bg-red-900/40 px-2 py-0.5 rounded transition-colors font-bold">
            ⚡ 重置熔断
          </button>
        )}
      </div>

      {/* Redline status indicators — live from IPC */}
      <div className="px-3 py-2 border-b border-cs-border/50 space-y-1.5">
        <RedlineIndicator
          label="Schema 强校验"
          status={redlineStatus?.schema_active ? "active" : "fail"}
          detail={redlineStatus ? "serde_json 反序列化器运行中" : "未连接"}
        />
        <RedlineIndicator
          label="文件沙盒白名单"
          status={redlineStatus?.sandbox_active ? "active" : "fail"}
          detail={`Scope: ${redlineStatus?.sandbox_root ?? "..."} | 已拦截 ${redlineStatus?.blocked_paths ?? 0} 路径`}
        />
        <RedlineIndicator
          label="自愈熔断计数器"
          status={redlineStatus?.fused ? "fail" : redlineStatus && redlineStatus.current_loop > 0 ? "warn" : "active"}
          detail={`Max_Healing_Loop: ${redlineStatus?.current_loop ?? 0}/${redlineStatus?.max_loop ?? 3}`}
        />
      </div>

      {/* Execution Timeline */}
      <div className="flex-1 overflow-y-auto">
        <div className="px-3 py-2 flex items-center space-x-2">
          <Clock className="w-3 h-3 text-cs-muted" />
          <span className="text-[10px] text-cs-muted font-medium">
            {t.execution_timeline}
          </span>
          <span className="text-[9px] text-cs-dim ml-auto">
            {buildTimeline(redlineStatus).length} events
          </span>
        </div>
        <div className="relative pl-8 pr-3 pb-2">
          {/* Timeline line */}
          <div className="absolute left-[23px] top-1 bottom-1 w-[1px] bg-cs-border" />

          {buildTimeline(redlineStatus).map((event) => (
            <div key={event.id} className="relative mb-1.5">
              {/* Node dot */}
              <div className="absolute left-[-17px] top-1.5">
                {event.status === "ok" && (
                  <CheckCircle2 className="w-3 h-3 text-cs-accent" />
                )}
                {event.status === "warn" && (
                  <AlertTriangle className="w-3 h-3 text-cs-warn" />
                )}
                {event.status === "fail" && (
                  <XCircle className="w-3 h-3 text-cs-danger" />
                )}
                {event.status === "info" && (
                  <div className="w-3 h-3 rounded-full border border-cs-muted bg-cs-bg" />
                )}
              </div>

              {/* Event card */}
              <div
                className={`ml-1 p-1.5 rounded border text-[9px] ${
                  event.status === "fail"
                    ? "border-cs-danger/30 bg-cs-danger/5"
                    : event.status === "warn"
                      ? "border-cs-warn/20 bg-cs-warn/5"
                      : "border-cs-border/40 bg-cs-bg/30"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span
                    className={`font-medium ${
                      event.status === "fail"
                        ? "text-cs-danger"
                        : event.status === "warn"
                          ? "text-cs-warn"
                          : "text-cs-dim"
                    }`}
                  >
                    {event.action}
                  </span>
                  <span className="text-cs-muted">{event.time}</span>
                </div>
                <div className="text-cs-muted mt-0.5">{event.detail}</div>

                {/* Red highlight for failed/healing events */}
                {event.status === "fail" && (
                  <div className="mt-1 flex items-center space-x-1">
                    <Redo className="w-2.5 h-2.5 text-cs-warn" />
                    <span className="text-cs-warn">
                      {t.self_healing_triggered} #{event.id === 6 ? "1" : "0"} triggered
                    </span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Bottom: fusion counter — live */}
      <div className="h-6 border-t border-cs-border bg-cs-bg px-3 flex items-center text-[9px] text-cs-muted">
        <ShieldAlert className={`w-2.5 h-2.5 mr-1.5 ${redlineStatus?.fused ? "text-cs-danger" : "text-cs-accent"}`} />
        <span>{t.healing_fuse}: </span>
        <span className={`font-bold ml-1 ${redlineStatus?.fused ? "text-cs-danger" : "text-cs-warn"}`}>
          {redlineStatus?.current_loop ?? 0}/{redlineStatus?.max_loop ?? 3}
        </span>
        <span className="text-cs-muted ml-1">
          {redlineStatus?.fused ? `⚠ ${t.fused}` : t.attempts_used}
        </span>
        <span className="ml-auto text-cs-dim">
          {privacyEnabled ? t.privacy_on : t.privacy_off}
        </span>
      </div>
    </div>
  );
}

function RedlineIndicator({
  label,
  status,
  detail,
}: {
  label: string;
  status: "active" | "warn" | "fail";
  detail: string;
}) {
  return (
    <div className="flex items-center space-x-2">
      <div
        className={`w-1.5 h-1.5 rounded-full ${
          status === "active"
            ? "bg-cs-accent"
            : status === "warn"
              ? "bg-cs-warn"
              : "bg-cs-danger"
        }`}
      />
      <span className="text-[9px] text-cs-dim flex-1">{label}</span>
      <span className="text-[8px] text-cs-muted">{detail}</span>
    </div>
  );
}
