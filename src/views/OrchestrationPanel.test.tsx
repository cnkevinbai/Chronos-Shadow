// src/views/OrchestrationPanel.test.tsx — 任务编排面板冒烟测试
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import OrchestrationPanel from "./OrchestrationPanel";

vi.mock("@/lib/i18n-context", () => ({
  useT: () => new Proxy({}, { get: (_t, k) => String(k) }),
  useLang: () => ({ lang: "zh", setLang: () => {} }),
}));

vi.mock("@/lib/tauri", () => ({
  orchScheduleQuality: vi.fn().mockResolvedValue({ quality_score: "88.5", parallel_groups: 3, completion_rate: "42.9%" }),
  orchParallelGroups: vi.fn().mockResolvedValue({
    total_groups: 1,
    groups: [{ group: 0, tasks: [{ id: "T1", title: "实现登录页", priority: 3, dependencies: [], status: "TaskStatus::Pending" }] }],
  }),
  orchTopologicalSort: vi.fn().mockResolvedValue(["T1", "T2"]),
  orchExecutableTasks: vi.fn().mockResolvedValue({ count: 1, tasks: [{ id: "T1", title: "实现登录页", priority: 3, dependencies: [] }] }),
  orchSmartRetry: vi.fn().mockResolvedValue([]),
  analyzeTask: vi.fn().mockResolvedValue({
    intent: "CodingImplementation",
    confidence: 0.87,
    recommended_agent: "Coder",
    recommended_model: "deepseek-v4-pro",
    model_reason: "代码实现任务，推荐 DeepSeek 高性价比",
    matched_skill: null,
    optimization_tip: "启用 DeepSeek Context Caching 可节省约 60% tokens",
    suggest_subagent: true,
    secondary_intents: [["CodeReview", 0.42, 0.3]],
  }),
}));

import { analyzeTask } from "@/lib/tauri";

describe("OrchestrationPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("渲染标题、质量分卡片与三个 Tab", async () => {
    render(<OrchestrationPanel />);
    expect(screen.getByText("dock_orchestrator")).toBeInTheDocument();
    expect(await screen.findByText("88.5")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /意图分析/ })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /并行组/ })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /执行顺序/ })).toBeInTheDocument();
  });

  it("意图分析：输入任务描述后调用 analyze_task 并渲染推荐结果", async () => {
    render(<OrchestrationPanel />);
    const input = screen.getByLabelText(/任务描述|Task description/);
    fireEvent.change(input, { target: { value: "实现一个登录页面" } });
    fireEvent.click(screen.getByLabelText(/运行分析|Run analysis/));
    await waitFor(() => expect(analyzeTask).toHaveBeenCalledWith("实现一个登录页面"));
    expect(await screen.findByText("CodingImplementation")).toBeInTheDocument();
    expect(screen.getByText("Coder")).toBeInTheDocument();
  });

  it("切换到并行组 Tab 渲染泳道与任务卡", async () => {
    render(<OrchestrationPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /并行组/ }));
    expect(await screen.findByText(/并行组 0/)).toBeInTheDocument();
    expect(screen.getByText("实现登录页")).toBeInTheDocument();
  });
});
