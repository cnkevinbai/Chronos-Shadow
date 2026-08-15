import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import DiffViewer, { MiniDiff } from "./DiffViewer";

describe("MiniDiff", () => {
  it("相同内容渲染原文", () => {
    const { container } = render(<MiniDiff before="hello" after="hello" />);
    expect(container.textContent).toBe("hello");
  });

  it("差异内容包含增删文本", () => {
    const { container } = render(<MiniDiff before="old line" after="new line" />);
    expect(container.textContent).toContain("old line");
    expect(container.textContent).toContain("new line");
  });
});

describe("DiffViewer", () => {
  it("空内容显示占位", () => {
    render(<DiffViewer />);
    expect(screen.getByText(/选择两个快照/i)).toBeInTheDocument();
  });

  it("显示增删计数", () => {
    render(<DiffViewer leftContent="a\nb" rightContent="a\nc" />);
    expect(screen.getByText("+1")).toBeInTheDocument();
    expect(screen.getByText("-1")).toBeInTheDocument();
  });
});
