import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import ErrorBoundary from "./ErrorBoundary";

// React 19 更严格的 JSX 类型要求组件有显式返回类型（否则 "cannot be used as a JSX component"）
function ThrowsError(): ReactNode {
  throw new Error("boom");
}

describe("ErrorBoundary", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("正常渲染子内容", () => {
    render(
      <ErrorBoundary>
        <p>normal content</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText("normal content")).toBeInTheDocument();
  });

  it("子组件抛错时渲染错误 UI", () => {
    // 抑制 React 渲染错误日志
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(
      <ErrorBoundary>
        <ThrowsError />
      </ErrorBoundary>,
    );
    expect(screen.getByText("界面渲染异常")).toBeInTheDocument();
    expect(screen.getByText("boom")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重新加载" })).toBeInTheDocument();
  });
});
