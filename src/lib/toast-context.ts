// src/lib/toast-context.ts — Toast 上下文定义（独立于 ToastProvider 组件文件，满足 fast-refresh 单组件导出约束）
import { createContext } from "react";

export type ToastType = "info" | "success" | "warning" | "error";

export interface ToastContextType {
  showToast: (type: ToastType, title: string, message: string) => void;
}

export const ToastContext = createContext<ToastContextType | undefined>(undefined);
