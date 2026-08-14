import { describe, it, expect } from "vitest";
import { cn, parseMarkdown } from "./utils";

describe("parseMarkdown", () => {
  it("解析 ``` 代码块", () => {
    const blocks = parseMarkdown("```ts\nconst x = 1;\n```");
    expect(blocks).toEqual([
      { type: "code", content: "const x = 1;", lang: "ts" },
    ]);
  });

  it("解析无语言代码块", () => {
    const blocks = parseMarkdown("```\nplain\n```");
    expect(blocks).toEqual([
      { type: "code", content: "plain", lang: undefined },
    ]);
  });

  it("解析列表项", () => {
    const blocks = parseMarkdown("- a\n- b\n- c");
    expect(blocks[0]).toMatchObject({ type: "list", items: ["a", "b", "c"] });
  });

  it("解析粗体", () => {
    const blocks = parseMarkdown("**hello** world");
    expect(blocks[0]).toEqual({ type: "bold", content: "hello" });
  });

  it("解析内联代码", () => {
    const blocks = parseMarkdown("use `unwrap()` carefully");
    expect(blocks.some((b) => b.type === "inlineCode" && b.content === "unwrap()")).toBe(true);
  });

  it("普通文本原样返回", () => {
    const blocks = parseMarkdown("hello world");
    expect(blocks).toEqual([{ type: "text", content: "hello world" }]);
  });
});

describe("cn", () => {
  it("合并 class 字符串", () => {
    expect(cn("px-2", "text-red")).toBe("px-2 text-red");
  });

  it("过滤假值", () => {
    expect(cn("px-2", undefined, false, "text-red")).toBe("px-2 text-red");
  });
});
