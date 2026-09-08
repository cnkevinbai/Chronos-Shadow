// src/lib/dialogs.ts — 统一对话框工具
// 背景：Tauri v2 WebView2 默认禁用原生 window.alert/confirm/prompt，
// 直接调用会静默失效（确认框返回 undefined、错误无提示）。
// 本模块在 Tauri 环境使用 @tauri-apps/plugin-dialog（capabilities 已含 dialog:default），
// 浏览器模式自动降级为原生对话框。
const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type DialogKind = "info" | "warning" | "error";

/** 确认对话框（替代 window.confirm）。返回 true=用户确认。 */
export async function appConfirm(
  msg: string,
  opts?: { title?: string; kind?: DialogKind },
): Promise<boolean> {
  if (isTauri()) {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    return await confirm(msg, {
      title: opts?.title ?? "CHRONOS-SHADOW",
      kind: opts?.kind ?? "warning",
    });
  }
  return window.confirm(msg);
}

/** 消息提示框（替代 window.alert）。 */
export async function appAlert(
  msg: string,
  opts?: { title?: string; kind?: DialogKind },
): Promise<void> {
  if (isTauri()) {
    const { message } = await import("@tauri-apps/plugin-dialog");
    await message(msg, {
      title: opts?.title ?? "CHRONOS-SHADOW",
      kind: opts?.kind ?? "info",
    });
  } else {
    window.alert(msg);
  }
}

/** 输入对话框（替代 window.prompt）——Tauri 下无原生等价物，浏览器降级。
 *  桌面端需要输入时请使用内联表单 / Modal，本函数仅供过渡。 */
export async function appPrompt(
  msg: string,
  defaultValue = "",
): Promise<string | null> {
  if (!isTauri()) return window.prompt(msg, defaultValue) ?? null;
  return null;
}
