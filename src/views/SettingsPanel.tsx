// src/views/SettingsPanel.tsx — 全局系统高级配置面板 (白皮书 6.7)
//
// IDE 设置网格布局：API密钥 / 成本风控 / 局域网 / 安全红线
// 配置持久化 + Tauri IPC 实时广播到 Rust 后端

import { useState, useEffect, useRef } from "react";
import { useLang, useT } from "@/lib/i18n-context";
import { APP_VERSION } from "@/lib/version";
import { useToast } from "@/components/ToastProvider";
import { loadSettings, saveSettings, checkLanHealth, getUserProfile, updateUserProfile, getAchievements } from "@/lib/tauri";
import { ChronosLogo, KeyIcon, GlobeIcon, ShieldIcon, CoinsIcon } from "@/components/SvgIcons";
import { MODELS } from "@/lib/models";
import type { Achievement } from "@/lib/types";

type Tab = "api" | "cost" | "lan" | "security" | "lang" | "personalization" | "about";

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

  // 个性化
  const [displayName, setDisplayName] = useState("开发者");
  const [nickname, setNickname] = useState("伙伴");
  const [avatar, setAvatar] = useState("🦀");
  const [personality, setPersonality] = useState("friendly");
  const [theme, setTheme] = useState("dark");
  const [workHoursStart, setWorkHoursStart] = useState(9);
  const [workHoursEnd, setWorkHoursEnd] = useState(18);
  const [skillLevel, setSkillLevel] = useState(50);
  const [workMode, setWorkMode] = useState("solo");
  const [achievements, setAchievements] = useState<Achievement[]>([]);

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

  // 加载个性化画像 + 成就
  useEffect(() => {
    getUserProfile().then((p) => {
      setDisplayName(p.display_name);
      setNickname(p.nickname);
      setAvatar(p.avatar);
      setPersonality(p.personality);
      setTheme(p.theme);
      setWorkHoursStart(p.work_hours_start);
      setWorkHoursEnd(p.work_hours_end);
      setSkillLevel(p.skill_level);
      setWorkMode(p.work_mode);
    }).catch(() => {});
    getAchievements().then(setAchievements).catch(() => {});
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
      // 个性化画像（独立于 config.json，存内存 + 重启恢复）
      await updateUserProfile({
        displayName, nickname, avatar, personality, theme,
        workHoursStart, workHoursEnd, skillLevel, workMode,
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
    { id: "personalization", icon: <span className="text-[14px]">🎨</span>, label: "个性化" },
    { id: "about", icon: <ChronosLogo size={14} className="stroke-current" />, label: "关于 & 开源隐私" },
  ];

  return (
    <div className="flex h-full bg-cs-bg font-mono text-sm text-cs-text select-none">
      {/* Left nav */}
      <div className="w-48 border-r border-cs-border bg-cs-surface p-2 flex flex-col">
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

        <div className="p-2 border-t border-cs-border">
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
                  className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
                />
              </div>
            ))}

            <ModelMatrix />
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
                className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white disabled:opacity-30 focus:border-zinc-500 outline-none w-28"
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
                  className="flex-1 bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
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
                className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none"
              />
            </div>

            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">{t.settings_lan_timeout}</label>
              <input
                type="number"
                value={lanTimeout}
                onChange={(e) => setLanTimeout(Number(e.target.value))}
                className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-28"
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
                className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-20" />
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
                    lang === code ? "border-emerald-500/50 bg-emerald-950/20 text-emerald-400" : "border-cs-border text-zinc-400 hover:border-zinc-500"
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

        {activeTab === "personalization" && (
          <SettingsSection title="🎨 个性化" desc="让 Chronos-Shadow 记住你，成为有温度的伙伴。">
            {/* 头像 + 名字 */}
            <div className="flex items-center space-x-3">
              <div className="text-4xl w-14 h-14 flex items-center justify-center bg-black border border-cs-border rounded-xl shrink-0">
                {avatar}
              </div>
              <div className="flex-1 space-y-2">
                <div className="flex flex-col space-y-1">
                  <label className="text-[11px] font-medium text-zinc-400">你的名字</label>
                  <input value={displayName} onChange={(e) => setDisplayName(e.target.value)}
                    className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none" />
                </div>
                <div className="flex flex-col space-y-1">
                  <label className="text-[11px] font-medium text-zinc-400">昵称（用于问候）</label>
                  <input value={nickname} onChange={(e) => setNickname(e.target.value)}
                    className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none" />
                </div>
              </div>
            </div>

            {/* 头像 emoji 选择 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">头像</label>
              <div className="flex flex-wrap gap-1.5">
                {["🦀", "🦊", "🐱", "🐼", "🦄", "🐙", "🤖", "👾", "🌟", "🔥", "🌙", "⚡"].map((e) => (
                  <button key={e} onClick={() => setAvatar(e)}
                    className={`w-8 h-8 rounded-lg flex items-center justify-center text-lg border transition-all ${avatar === e ? "border-emerald-400 bg-emerald-400/20" : "border-cs-border hover:border-zinc-500"}`}>
                    {e}
                  </button>
                ))}
              </div>
            </div>

            {/* 系统人格 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">系统人格</label>
              <div className="flex space-x-2">
                {[
                  { v: "professional", l: "💼 专业" },
                  { v: "friendly", l: "🤗 友好" },
                  { v: "playful", l: "🎮 活泼" },
                ].map(({ v, l }) => (
                  <button key={v} onClick={() => setPersonality(v)}
                    className={`px-3 py-1.5 rounded border text-xs transition-all ${personality === v ? "border-emerald-400 bg-emerald-400/20 text-emerald-300" : "border-cs-border text-zinc-400 hover:border-zinc-500"}`}>
                    {l}
                  </button>
                ))}
              </div>
            </div>

            {/* 工作时段 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">工作时段（影响问候语）</label>
              <div className="flex items-center space-x-2">
                <input type="number" min={0} max={23} value={workHoursStart} onChange={(e) => setWorkHoursStart(Number(e.target.value))}
                  className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-20" />
                <span className="text-zinc-500">—</span>
                <input type="number" min={1} max={24} value={workHoursEnd} onChange={(e) => setWorkHoursEnd(Number(e.target.value))}
                  className="bg-black border border-cs-border rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none w-20" />
                <span className="text-[10px] text-zinc-600">时</span>
              </div>
            </div>

            {/* 技能等级 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">技能熟练度：{skillLevel}</label>
              <input type="range" min={0} max={100} value={skillLevel} onChange={(e) => setSkillLevel(Number(e.target.value))}
                className="w-full accent-emerald-500" />
            </div>

            {/* 工作模式 */}
            <div className="flex flex-col space-y-1">
              <label className="text-[11px] font-medium text-zinc-400">工作模式</label>
              <div className="flex space-x-2">
                {[
                  { v: "solo", l: "🧑‍💻 独自" },
                  { v: "collaborative", l: "🤝 协作" },
                  { v: "learning", l: "📚 学习" },
                ].map(({ v, l }) => (
                  <button key={v} onClick={() => setWorkMode(v)}
                    className={`px-3 py-1.5 rounded border text-xs transition-all ${workMode === v ? "border-emerald-400 bg-emerald-400/20 text-emerald-300" : "border-cs-border text-zinc-400 hover:border-zinc-500"}`}>
                    {l}
                  </button>
                ))}
              </div>
            </div>

            {/* 成就墙 */}
            <div className="pt-2 border-t border-cs-border/50">
              <div className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider mb-2">🏆 成就</div>
              {achievements.length === 0 ? (
                <div className="text-[11px] text-zinc-600">使用 Chronos-Shadow 后，成就将在此点亮。</div>
              ) : (
                <div className="grid grid-cols-3 gap-2">
                  {achievements.map((a) => (
                    <div key={a.id} className={`p-2 rounded border text-center ${a.unlocked ? "border-emerald-500/40 bg-emerald-950/20" : "border-cs-border/50 bg-black/40 opacity-60"}`}>
                      <div className="text-xl">{a.unlocked ? a.emoji : "🔒"}</div>
                      <div className="text-[10px] font-bold text-zinc-300 mt-1">{a.name}</div>
                      <div className="text-[9px] text-zinc-500 mt-0.5">{a.description}</div>
                      <div className="mt-1 h-1 bg-zinc-800 rounded overflow-hidden">
                        <div className="h-full bg-emerald-400" style={{ width: `${Math.round(a.progress * 100)}%` }} />
                      </div>
                    </div>
                  ))}
                </div>
              )}
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
                <p className="text-[10px] text-zinc-500 font-light mt-0.5">Version {APP_VERSION} · Powered by Tauri v2 & Rust Core</p>
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
    <div className="flex items-center justify-between py-2 border-b border-cs-border/50">
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

function ModelMatrix() {
  const providerLabel: Record<string, string> = {
    deepseek: "DeepSeek", kimi: "Kimi", glm: "GLM", ollama: "Ollama",
  };
  const tierBadge: Record<string, { label: string; cls: string }> = {
    premium: { label: "旗舰", cls: "text-amber-400 border-amber-500/40 bg-amber-950/20" },
    standard: { label: "标准", cls: "text-cyan-400 border-cyan-500/40 bg-cyan-950/20" },
    budget: { label: "经济", cls: "text-emerald-400 border-emerald-500/40 bg-emerald-950/20" },
    free: { label: "免费", cls: "text-zinc-400 border-zinc-600/40 bg-zinc-900/40" },
  };
  const providers = ["deepseek", "kimi", "glm", "ollama"] as const;

  return (
    <div className="pt-3 border-t border-cs-border/50">
      <div className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider mb-2">
        模型能力矩阵 (Provider / Cost)
      </div>
      <div className="space-y-3">
        {providers.map((p) => {
          const list = MODELS.filter((m) => m.provider === p);
          if (list.length === 0) return null;
          return (
            <div key={p}>
              <div className="text-[10px] font-bold text-zinc-400 mb-1">{providerLabel[p]}</div>
              <div className="space-y-1">
                {list.map((m) => {
                  const tb = tierBadge[m.costTier] ?? tierBadge.standard;
                  return (
                    <div key={m.key} className="flex items-center justify-between px-2 py-1 rounded bg-black/40 border border-cs-border/50">
                      <div className="flex items-center space-x-2 min-w-0">
                        <span className="text-[11px] text-zinc-300 truncate">{m.shortDisplay}</span>
                        {m.isVision && (
                          <span className="text-[9px] text-purple-400 border border-purple-500/40 bg-purple-950/20 px-1 rounded">视觉</span>
                        )}
                      </div>
                      <div className="flex items-center space-x-1.5 shrink-0">
                        <span className={`text-[9px] px-1.5 py-0.5 rounded border ${tb.cls}`}>{tb.label}</span>
                        <span className="text-[9px] text-zinc-600" title="上下文窗口 (tokens)">{(m.contextWindow / 1000).toFixed(0)}K ctx</span>
                        {m.supportsCache && (
                          <span className="text-[9px] text-emerald-400" title="支持 Context Caching">⚡缓存</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}


