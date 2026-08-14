import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import Modal from "./Modal";

describe("Modal", () => {
  it("open=false 时不渲染任何内容", () => {
    const { container } = render(
      <Modal open={false} onClose={() => {}} title="test">
        <p>content</p>
      </Modal>,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("open=true 时渲染标题和子内容", () => {
    render(
      <Modal open={true} onClose={() => {}} title="My Title">
        <p>Hello content</p>
      </Modal>,
    );
    expect(screen.getByText("My Title")).toBeInTheDocument();
    expect(screen.getByText("Hello content")).toBeInTheDocument();
  });

  it("点击关闭按钮触发 onClose", () => {
    const onClose = vi.fn();
    render(
      <Modal open={true} onClose={onClose} title="t">
        <p>c</p>
      </Modal>,
    );
    fireEvent.click(screen.getByRole("button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
