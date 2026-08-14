import { describe, it, expect } from "vitest";
import {
  getModel,
  getModelDisplay,
  getLLMs,
  getVLMs,
  classifyModelKeys,
} from "./models";

describe("models 注册表", () => {
  it("getModel 返回正确条目", () => {
    const m = getModel("deepseek-v4-pro");
    expect(m?.provider).toBe("deepseek");
    expect(m?.supportsCache).toBe(true);
    expect(m?.contextWindow).toBe(131072);
  });

  it("getModel 对未知 key 返回 undefined", () => {
    expect(getModel("not-a-model")).toBeUndefined();
  });

  it("getModelDisplay 返回 shortDisplay，未知回退到 key", () => {
    expect(getModelDisplay("deepseek-v4-flash")).toBe("DeepSeek V4-Flash");
    expect(getModelDisplay("unknown-model")).toBe("unknown-model");
  });

  it("getLLMs 过滤非视觉模型", () => {
    const llms = getLLMs();
    expect(llms.length).toBeGreaterThan(0);
    expect(llms.every((m) => !m.isVision)).toBe(true);
  });

  it("getVLMs 只含视觉模型", () => {
    const vlms = getVLMs();
    expect(vlms.length).toBeGreaterThan(0);
    expect(vlms.every((m) => m.isVision)).toBe(true);
  });

  it("classifyModelKeys 正确分类 + 标记未知", () => {
    const result = classifyModelKeys([
      "deepseek-v4-pro",
      "glm-5v-turbo",
      "not-a-real-model",
    ]);
    expect(result.llms).toContain("deepseek-v4-pro");
    expect(result.vlms).toContain("glm-5v-turbo");
    expect(result.unknown).toEqual(["not-a-real-model"]);
  });
});
