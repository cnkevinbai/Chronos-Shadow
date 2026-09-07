// src/lib/use-toast.ts — Toast hook（独立于 ToastProvider 组件文件，满足 fast-refresh 单组件导出约束）
import { useContext } from "react";
import { ToastContext, type ToastContextType } from "@/lib/toast-context";

export function useToast(): ToastContextType {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within <ToastProvider>");
  return ctx;
}
