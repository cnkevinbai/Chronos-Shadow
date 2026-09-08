// src/views/SettingsPanel.test.tsx — 全局配置面板核心链路测试
// mock 策略：i18n 字典以 Proxy 返回键名本身（断言稳定、不随语言漂移）；
// tauri.ts 全部 IPC mock；useToast mock 收集调用。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import SettingsPanel from "./SettingsPanel";

const showToastMock = vi.fn();
const onKeyChangeMock = vi.fn();

vi.mock("@/lib/i18n-context", () => ({
  useT: () => new Proxy({}, { get: (_t, k) => String(k) }),
  useLang: () => ({ lang: "zh", setLang: () => {} }),
}));

vi.mock("@/lib/use-toast", () => ({
  useToast: () => ({ showToast: showToastMock }),
}));

vi.mock("@/lib/tauri", () => ({
  loadSettings: vi.fn().mockResolvedValue({
    cost_cap: 9.5,
    cost_cap_enabled: true,
    ollama_url: "http://localhost:11434",
    lan_model: "deepseek-v4-flash",
    lan_timeout: 3500,
    auto_fallback: true,
    max_healing: 3,
    ast_audit: true,
    block_gpl: true,
    privacy_blur: true,
    has_key_deepseek: true,
    has_key_kimi: false,
    has_key_glm: false,
  }),
  saveSettings: vi.fn().mockResolvedValue("CONFIG WRITTEN"),
  checkLanHealth: vi.fn().mockResolvedValue({ ok: [] }),
  getUserProfile: vi.fn().mockResolvedValue({
    display_name: "开发者",
    nickname: "伙伴",
avatar: "",
    personality: "friendly",
    theme: "dark",
    work_hours_start: 9,
    work_hours_end: 18,
    skill_level: 50,
    work_mode: "solo",
  }),
  updateUserProfile: vi.fn().mockResolvedValue(undefined),
  getAchievements: vi.fn().mockResolvedValue([]),
  vaultApiKey: vi.fn().mockResolvedValue(undefined),
}));

import { loadSettings, saveSettings, vaultApiKey } from "@/lib/tauri";

function renderPanel() {
  return render(
    <SettingsPanel
      hasKeys={{ deepseek: false, kimi: false, glm: false }}
      onKeyChange={onKeyChangeMock}
    />,
  );
}

describe("SettingsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("默认渲染 API 凭据 Tab 并显示三家 provider", () => {
    renderPanel();
    expect(screen.getAllByText("settings_api_credentials").length).toBeGreaterThan(0);
    expect(screen.getByText("settings_deepseek_key")).toBeInTheDocument();
    expect(screen.getByText("settings_kimi_key")).toBeInTheDocument();
    expect(screen.getByText("settings_glm_key")).toBeInTheDocument();
  });

  it("挂载时调用 loadSettings 并将成本上限应用到输入框", async () => {
    renderPanel();
    await waitFor(() => expect(loadSettings).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByText("settings_cost_risk"));
    await waitFor(() => {
      const amount = screen.getByDisplayValue("9.5");
      expect(amount).toBeInTheDocument();
    });
  });

  it("loadSettings 返回的 key 状态会触发 onKeyChange 回调", async () => {
    renderPanel();
    await waitFor(() => {
      expect(onKeyChangeMock).toHaveBeenCalledWith("deepseek", true);
    });
    expect(onKeyChangeMock).not.toHaveBeenCalledWith("kimi", true);
  });

  it("点击保存按钮触发 saveSettings + updateUserProfile 链路并弹出成功 toast", async () => {
    renderPanel();
    fireEvent.click(screen.getByText("apply_changes"));
    await waitFor(() => expect(saveSettings).toHaveBeenCalledTimes(1));
    expect(saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ version: 1, privacy_blur: true }),
    );
    await waitFor(() => {
      expect(showToastMock).toHaveBeenCalledWith(
        "success",
        "CONFIG SAVED",
        "CONFIG WRITTEN",
      );
    });
  });

  it("在 API Tab 输入密钥触发 vaultApiKey 并回调 onKeyChange", async () => {
    renderPanel();
    const input = screen.getAllByPlaceholderText("在此输入 API Key")[0];
    fireEvent.change(input, { target: { value: "sk-test-123" } });
    await waitFor(() => {
      expect(vaultApiKey).toHaveBeenCalledWith("deepseek", "sk-test-123");
    });
    await waitFor(() => {
      expect(onKeyChangeMock).toHaveBeenCalledWith("deepseek", true);
    });
  });
});
