import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ToastProvider, useToast } from "./ToastProvider";

afterEach(() => {
  vi.useRealTimers();
});

function Trigger() {
  const { showToast } = useToast();
  return (
    <button onClick={() => showToast("success", "已保存", "配置已写入")}>
      trigger
    </button>
  );
}

describe("ToastProvider", () => {
  it("showToast 渲染标题与消息", () => {
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByText("trigger"));
    expect(screen.getByText("已保存")).toBeInTheDocument();
    expect(screen.getByText("配置已写入")).toBeInTheDocument();
  });

  it("点击 ✕ 关闭 toast", () => {
    vi.useFakeTimers();
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByText("trigger"));
    expect(screen.getByText("已保存")).toBeInTheDocument();

    fireEvent.click(screen.getByText("✕"));
    expect(screen.queryByText("已保存")).not.toBeInTheDocument();
  });
});
