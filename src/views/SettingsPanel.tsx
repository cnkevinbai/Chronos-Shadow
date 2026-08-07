// src/views/SettingsPanel.tsx — 全局系统高级配置面板 (白皮书 6.7)
//
// IDE 设置网格布局：API密钥 / 成本风控 / 局域网 / 安全红线
// 配置持久化 + Tauri IPC 实时广播到 Rust 后端

import { useState, useEffect, useRef } from "react";
import { useLang, useT } from "@/lib/i18n-context";
import { useToast } from "@/components/ToastProvider";
import { setModelApiKey, loadSettings, saveSettings, checkLanHealth } from "@/lib/tauri";
import { ChronosLogo, KeyIcon, GlobeIcon, ShieldIcon, CoinsIcon } from "@/components/SvgIcons";

type Tab = "api" | "cost" | "lan" | "security" | "lang" | "about";

interface SettingsPanelProps {
  hasKeys: { deepseek: boolean; kimi: boolean; glm: boolean };
  onKeyChange: (provider: string, has: boolean) => void;
}

export default function SettingsPanel({ hasKeys, onKeyChange }: SettingsPanelProps) {
  const [activeTab, setActiveTab] = useState<Tab>("api");

  // 成本风控
  const [costCap, setCostCap] = useState(5.0);
  const [costCapEnabled, setCostCapEnabled] = useState(true);
  const [cachingPriority, setCachingPriority] = useState(true);

  // 局域网
  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");
  const [lanModel, setLanModel] = useState("deepseek-v4-flash");
  const [lanTimeout, setLanTimeout] = useState(3500);
  const [autoFallback, setAutoFallback] = useState(true);
  const [ollamaStatus, setOllamaStatus] = useState<
    "idle" | "checking" | { ok: string[] } | { err: string }
  >("idle");

  // 安全红线
  const [maxHealing, setMaxHealing] = useState(3);
  const [astAudit, setAstAudit] = useState(true);
  const [blockGpl, setBlockGpl] = useState(true);
  const [privacyBlur, setPrivacyBlur] = useState(true);

  const [saving, setSaving] = useState(false);
  const savingTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { lang, setLang } = useLang();
  const t = useT();
  const toast = useToast();

  // Cleanup saving timer on unmount
  useEffect(() => {
    return () => { if (savingTimer.current) clearTimeout(savingTimer.current); };
  }, []);

  // Load settings from backend on mount
  useEffect(() => {
    loadSettings().then((s) => {
      setCostCap(s.cost_cap);
      setCostCapEnabled(s.cost_cap_enabled);
      setOllamaUrl(s.ollama_url);
      setLanModel(s.lan_model);
      setLanTimeout(s.lan_timeout);
      setAutoFallback(s.auto_fallback);
      setMaxHealing(s.max_healing);
      setAstAudit(s.ast_audit);
      setBlockGpl(s.block_gpl);
      setPrivacyBlur(s.privacy_blur);
      // Restore key presence flags from vault
      if (s.has_key_deepseek) onKeyChange("deepseek", true);
      if (s.has_key_kimi) onKeyChange("kimi", true);
      if (s.has_key_glm) onKeyChange("glm", true);
    }).catch(() => {});
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      // Save settings first
      const result = await saveSettings({
        version: 1,
        cost_cap: costCap, cost_cap_enabled: costCapEnabled,
        ollama_url: ollamaUrl, lan_model: lanModel, lan_timeout: lanTimeout,
        auto_fallback: autoFallback, max_healing: maxHealing,
        ast_audit: astAudit, block_gpl: blockGpl, privacy_blur: privacyBlur,
        caching_priority: cachingPriority, accumulated_cost: 0,
        api_key_deepseek: "", api_key_kimi: "", api_key_glm: "",
      });
      // Router keys are now vault-resolved server-side — no sync needed
      toast.showToast("success", "CONFIG SAVED", result);
    } catch (e) {
      toast.showToast("error", "SAVE FAILED", String(e));
    }
    if (savingTimer.current) clearTimeout(savingTimer.current);
    savingTimer.current = setTimeout(() => setSaving(false), 600);
  };

  const tabs: { id: Tab; icon: React.ReactNode; label: string }[] = [
    { id: "api", icon: <KeyIcon size={14} className="stroke-current" />, label: t.settings_api_credentials },
    { id: "cost", icon: <CoinsIcon size={14} className="stroke-current" />, label: t.settings_cost_risk },
    { id: "lan", icon: <GlobeIcon size={14} className="stroke-current" />, label: t.settings_lan_gateway },
    { id: "security", icon: <ShieldIcon size={14} className="stroke-current" />, label: t.settings_security },
    { id: "lang", icon: <GlobeIcon size={14} className="stroke-current" />, label: t.settings_language },
    { id: "about", icon: <ChronosLogo size={14} className="stroke-current" />, label: "关于 & 开源隐私" },
  ];

  return (
    <div className="flex h-full bg-[#09090b] font-mono text-sm text-[#fafafa] select-none">
      {/* Left nav */}
      <div className="w-48 border-r border-[#27272a] bg-[#0c0c0e] p-2 flex flex-col">
        <div className="px-3 py-2 text-[10px] font-bold text-zinc-500 uppercase tracking-wider">
          {t.settings}
        </div>
        <div className="space-y-0.5 flex-1">
          {tabs.map((t) => (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id)}
              className={`w-full flex items-center space-x-2 px-3 py-1.5 rounded text-left text-xs transition-all ${
                activeTab === t.id
                  ? "bg-[#27272a] text-white font-bold"
                  : "text-zinc-400 hover:bg-zinc-900"
              }`}
            >
              <span>{t.icon}</span>
              <span>{t.label}</span>
            </button>
          ))}
        </div>

        <div className="p-2 border-t border-[#27272a]">
          <button
            onClick={handleSave}
            disabled={saving || activeTab === "about"}
            className="w-full bg-zinc-100 hover:bg-zinc-200 active:bg-zinc-300 active:scale-[0.98] text-black font-bold text-xs py-1.5 rounded transition-all duration-150 disabled:opacity-50 disabled:scale-100 disabled:cursor-not-allowed"
          >
            {saving ? t.syncing : t.apply_changes}
          </button>
        </div>
      </div>

      {/* Right content */}
      <div className="flex-1 p-6 overflow-y-auto animate-fadeIn">
        {activeTab === "api" && (
          <SettingsSection title={t.settings_api_credentials} desc={lang === "zh" ? "API 密钥存储于 Windows 凭据保险箱，前端永不可见。在此粘贴新密钥以更新。" : "API keys vaulted in Windows Credential Manager. Paste new key below to update."}>
            {[
              { provider: "deepseek" as const, label: t.settings_deepseek_key, has: hasKeys.deepseek },
              { provider: "kimi" as const, label: t.settings_kimi_key, has: hasKeys.kimi },
              { provider: "glm" as const, label: t.settings_glm_key, has: hasKeys.glm },
            ].map(({ provider, label, has }) => (
              <div key={provider} className="flex flex-col space-y-1">
                <label className="text-[11px] font-medium text-zinc-400">
                  {label} {has && <span className="text-emerald-400 text-[10px]">✓ 已配置</span>}
                </label>
                <input
                  type="password"
                  placeholder={has ? "•••••••• (已存储于凭据保险箱)" : "在此输入 API Key"}
                  onChange={async (e) => {
                    const key = e.target.value.trim();
                    if (!key) return;
                    try {
                      const { vaultApiKey } = await import("@/lib/tauri");
                      await vaultApiKey(provider, key);
                      onKeyChange(provider, true);
                      e.target.value = "";
                      toast.showToast("success", "KEY VAULTED", `[${provider}] 已写入 Windows 凭据保险箱。`);
                    } catch {
                      toast.showToast("error", "VAULT FAILED", "凭据写入失败，请检查系统权限。");
                    }
                  }}
                  className="bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
                />
              </div>
            ))}
          </SettingsSection>
        )}

        {activeTab === "cost" && (
          <SettingsSection title={t.settings_cost_risk} desc={lang === "zh" ? "控制长链路多步自动化执行时的最高开销额度，后端实时计费引擎强制执行。" : "Control max cost for long-chain multi-step automation. Backend billing engine enforces hard cap."}>
            <ToggleRow
              label={t.settings_cost_cap_label}
              sub={t.settings_cost_cap_desc}
              enabled={costCapEnabled}
              onChange={setCostCapEnabled}
            />
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">{t.cost_amount_label}</label>
              <input
                type="number"
                disabled={!costCapEnabled}
                value={costCap}
                onChange={(e) => setCostCap(Number(e.target.value))}
                className="bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white disabled:opacity-30 focus:border-zinc-500 outline-none w-28"
              />
            </div>
            <ToggleRow
              label={t.settings_caching_label}
              sub={t.settings_caching_desc}
              enabled={cachingPriority}
              onChange={setCachingPriority}
            />
          </SettingsSection>
        )}

        {activeTab === "lan" && (
          <SettingsSection title={t.settings_lan_gateway} desc={lang === "zh" ? "配置本地私有模型节点。云端超时或余额不足时毫秒级无感降级热切换。" : "Configure local private model nodes. Seamless millisecond-level hot-swap when cloud times out or balance runs low."}>
            {/* Ollama 端点配置 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">
                {t.settings_ollama_url}
              </label>
              <div className="flex items-center space-x-2">
                <input
                  type="text"
                  value={ollamaUrl}
                  onChange={(e) => setOllamaUrl(e.target.value)}
                  placeholder="http://localhost:11434"
                  className="flex-1 bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
                />
                <button
                  onClick={async () => {
                    setOllamaStatus("checking");
                    try {
                      const models = await checkLanHealth();
                      if (models.length > 0) {
                        setOllamaStatus({ ok: models });
                        // 自动选择第一个模型
                        if (!lanModel || !models.includes(lanModel)) {
                          setLanModel(models[0]);
                        }
                      } else {
                        setOllamaStatus({ err: "已连接但无可用模型" });
                      }
                    } catch {
                      setOllamaStatus({ err: "无法连接到 Ollama — 请确认服务已启动" });
                    }
                  }}
                  disabled={ollamaStatus === "checking"}
                  className="text-[10px] bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 text-zinc-200 px-3 py-1.5 rounded transition-colors disabled:opacity-40 shrink-0"
                >
                  {ollamaStatus === "checking" ? "⏳" : "🩺 检测"}
                </button>
              </div>
            </div>

            {/* 连接状态指示 */}
            {ollamaStatus !== "idle" && (
              <div
                className={`p-2 rounded border text-[10px] ${
                  typeof ollamaStatus === "object" && "ok" in ollamaStatus
                    ? "border-emerald-500/30 bg-emerald-950/20 text-emerald-400"
                    : ollamaStatus === "checking"
                      ? "border-amber-500/30 bg-amber-950/20 text-amber-400"
                      : "border-red-500/30 bg-red-950/20 text-red-400"
                }`}
              >
                {typeof ollamaStatus === "object" && "ok" in ollamaStatus && (
                  <div className="space-y-1.5">
                    <div className="flex items-center space-x-1">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                      <span className="font-bold">已连接</span>
                      <span className="text-zinc-500">
                        — {ollamaStatus.ok.length} 个模型
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-1">
                      {ollamaStatus.ok.map((m) => (
                        <button
                          key={m}
                          onClick={() => setLanModel(m)}
                          className={`px-2 py-0.5 rounded text-[9px] border transition-colors ${
                            lanModel === m
                              ? "border-emerald-400 bg-emerald-400/20 text-emerald-300"
                              : "border-zinc-700 text-zinc-400 hover:border-zinc-500"
                          }`}
                        >
                          {m}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
                {ollamaStatus === "checking" && (
                  <div className="flex items-center space-x-2">
                    <span className="animate-pulse">⏳</span>
                    <span>正在探测 Ollama 服务…</span>
                  </div>
                )}
                {typeof ollamaStatus === "object" && "err" in ollamaStatus && (
                  <div>
                    <span className="text-red-400">❌ {ollamaStatus.err}</span>
                    <div className="text-zinc-600 mt-1">
                      请确认 Ollama 已安装并运行：<code className="text-zinc-500">ollama serve</code>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* 当前选定模型 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">
                {t.settings_lan_model}
              </label>
              <input
                type="text"
                value={lanModel}
                onChange={(e) => setLanModel(e.target.value)}
                className="bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
              />
            </div>

            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">{t.settings_lan_timeout}</label>
              <input
                type="number"
                value={lanTimeout}
                onChange={(e) => setLanTimeout(Number(e.target.value))}
                className="bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-28"
              />
            </div>
            <ToggleRow label={t.settings_fallback_label} sub={t.settings_fallback_desc} enabled={autoFallback} onChange={setAutoFallback} />
          </SettingsSection>
        )}

        {activeTab === "security" && (
          <SettingsSection title={t.settings_security} desc={lang === "zh" ? "防幻觉过滤策略与物理隔离红线配置。" : "Anti-hallucination filters & physical sandbox rules."}>
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">{t.settings_healing_label}</label>
              <input type="number" value={maxHealing} onChange={(e) => setMaxHealing(Number(e.target.value))}
                className="bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-20" />
              <span className="text-[10px] text-zinc-600">{t.settings_healing_desc}</span>
            </div>
            <ToggleRow label={t.settings_ast_label} sub={t.settings_ast_desc} enabled={astAudit} onChange={setAstAudit} />
            <ToggleRow label={t.settings_gpl_label} sub={t.settings_gpl_desc} enabled={blockGpl} onChange={setBlockGpl} />
            <ToggleRow label={t.settings_privacy_label} sub={t.settings_privacy_desc} enabled={privacyBlur} onChange={setPrivacyBlur} />
          </SettingsSection>
        )}

        {activeTab === "lang" && (
          <SettingsSection title={lang === "zh" ? "🌐 界面语言" : "🌐 Language"} desc={lang === "zh" ? "切换 Chronos-Shadow 全局界面显示语言。" : "Switch Chronos-Shadow global UI language."}>
            <div className="space-y-2">
              {[
                { code: "zh" as const, label: "简体中文", sub: "Chinese (Simplified)" },
                { code: "en" as const, label: "English", sub: "英语" },
              ].map(({ code, label, sub }) => (
                <button
                  key={code}
                  onClick={() => setLang(code)}
                  className={`w-full flex items-center justify-between px-4 py-3 rounded border text-left text-xs transition-all ${
                    lang === code ? "border-emerald-500/50 bg-emerald-950/20 text-emerald-400" : "border-[#27272a] text-zinc-400 hover:border-zinc-500"
                  }`}
                >
                  <div>
                    <div className="font-bold">{label}</div>
                    <div className="text-[10px] text-zinc-500">{sub}</div>
                  </div>
                  {lang === code && <span className="text-emerald-400 text-lg">✓</span>}
                </button>
              ))}
            </div>
          </SettingsSection>
        )}

        {activeTab === "about" && (
          <div className="space-y-5 max-w-2xl animate-fadeIn text-xs leading-relaxed text-zinc-400">
            {/* 产品徽标与版本 */}
            <div className="flex items-center space-x-4 border-b border-zinc-900 pb-4">
              <div className="w-12 h-12 bg-black border-2 border-zinc-700 rounded-lg flex items-center justify-center shadow-xl select-none">
                <ChronosLogo size={22} className="stroke-cyan-400" />
              </div>
              <div>
                <h2 className="text-sm font-bold text-white tracking-wide">Chronos-Shadow (时空之影)</h2>
                <p className="text-[10px] text-zinc-500 font-light mt-0.5">Version 2026.1.0-Stable · Powered by Tauri v2 & Rust Core</p>
              </div>
            </div>

            {/* 产品介绍 */}
            <div className="space-y-1">
              <h4 className="font-bold text-white">关于 Chronos-Shadow</h4>
              <p className="font-light text-zinc-400">
                Chronos-Shadow 是一款将大模型潜能与 Windows 系统底层操控深度融合的下一代工业级开源桌面智能体。
                内置标准 MCP 客户端总线、自适应分布式集群管理、游戏化降本对账单，以及 AES-256-GCM 会话加密与 Windows 凭据管理器密钥托管。
              </p>
            </div>

            {/* Apache 2.0 许可 */}
            <div className="space-y-1.5">
              <h4 className="font-bold text-white">开源授权 (Apache 2.0)</h4>
              <div className="bg-black border border-zinc-900 p-2.5 rounded h-28 overflow-y-auto font-mono text-[10px] text-zinc-600 leading-normal whitespace-pre-wrap select-text">
{`Copyright 2026 Chronos-Shadow Open Source Team.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at http://apache.org

Unless required by applicable law, software distributed under
the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
or implied.  See the License for specific language governing
permissions and limitations under the License.`}
              </div>
            </div>

            {/* 隐私保护声明 */}
            <div className="space-y-1">
              <h4 className="font-bold text-white">金融级隐私保护</h4>
              <div className="bg-black/40 border border-zinc-900/60 p-3 rounded-md space-y-2 text-[10px] font-light">
                <div className="flex items-start space-x-1.5">
                  <span className="text-emerald-400 shrink-0">✔</span>
                  <p><b className="text-zinc-300">零明文密钥外泄：</b>API Key 托管于 Windows Credential Vault，磁盘不留存明文。</p>
                </div>
                <div className="flex items-start space-x-1.5">
                  <span className="text-emerald-400 shrink-0">✔</span>
                  <p><b className="text-zinc-300">端侧 CV 脱敏：</b>多模态走查前 ONNX 本地模型强制像素级打码，隐私绝不出海。</p>
                </div>
                <div className="flex items-start space-x-1.5">
                  <span className="text-emerald-400 shrink-0">✔</span>
                  <p><b className="text-zinc-300">AES-256-GCM 会话加密：</b>历史会话分块落盘前流式加密，密钥硬件指纹绑定，离线破解不可行。</p>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Reusable sub-components ──────────────────────────────────────

function SettingsSection({
  title,
  desc,
  children,
}: {
  title: string;
  desc: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-4 max-w-xl">
      <div>
        <h3 className="text-sm font-bold text-white mb-1">{title}</h3>
        <p className="text-xs text-zinc-500">{desc}</p>
      </div>
      <div className="space-y-3 pt-1">{children}</div>
    </div>
  );
}

function ToggleRow({
  label,
  sub,
  enabled,
  onChange,
}: {
  label: string;
  sub: string;
  enabled: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-[#27272a]/50">
      <div className="flex flex-col space-y-0.5">
        <span className="text-[11px] font-medium text-white">{label}</span>
        <span className="text-[10px] text-zinc-500">{sub}</span>
      </div>
      <button
        onClick={() => onChange(!enabled)}
        className={`w-8 h-4 rounded-full p-0.5 transition-colors relative outline-none ${
          enabled ? "bg-emerald-500" : "bg-[#27272a]"
        }`}
      >
        <div
          className={`w-3 h-3 rounded-full bg-white transition-transform ${
            enabled ? "translate-x-4" : "translate-x-0"
          }`}
        />
      </button>
    </div>
  );
}


