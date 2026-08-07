// 端侧通知总线 (Toast Notification System)
// 高响应异步自动销毁 + 角色专属色彩 + 磨砂玻璃极客风
//
// 类型：
// - info:    系统提示 / 路由切换
// - success: 执行成功 / 省钱对账
// - warning: 红线拦截 / 手动覆盖
// - error:   熔断 / 编译失败

import { createContext, useContext, useState, useCallback, type ReactNode } from "react";
import { CoinsIcon, ShieldIcon, AlertOctagonIcon, TerminalIcon } from "./SvgIcons";

export type ToastType = "info" | "success" | "warning" | "error";

interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message: string;
}

interface ToastContextType {
  showToast: (type: ToastType, title: string, message: string) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const showToast = useCallback((type: ToastType, title: string, message: string) => {
    const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    setToasts((prev) => [...prev, { id, type, title, message }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 2800);
  }, []);

  const getStyle = (type: ToastType) => {
    switch (type) {
      case "success":
        return "border-emerald-500/40 bg-[#0c120c] text-emerald-400 shadow-[0_0_15px_rgba(16,185,129,0.12)]";
      case "warning":
        return "border-amber-500/40 bg-[#14100c] text-amber-400 shadow-[0_0_15px_rgba(245,158,11,0.12)]";
      case "error":
        return "border-red-500/40 bg-[#140c0c] text-red-400 shadow-[0_0_20px_rgba(239,68,68,0.15)] animate-shake";
      default:
        return "border-[#27272a] bg-[#121214] text-zinc-300 shadow-[0_0_12px_rgba(0,0,0,0.4)]";
    }
  };

  const getIcon = (type: ToastType) => {
    switch (type) {
      case "success": return <CoinsIcon size={16} className="stroke-emerald-400" />;
      case "warning": return <ShieldIcon size={16} className="stroke-amber-400" />;
      case "error": return <AlertOctagonIcon size={16} className="stroke-red-400" />;
      default: return <TerminalIcon size={16} className="stroke-zinc-400" />;
    }
  };

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className="fixed top-14 right-4 z-50 flex flex-col space-y-2 pointer-events-none max-w-xs w-full">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`pointer-events-auto border p-3 rounded-md flex items-start space-x-2.5 text-xs backdrop-blur-md transition-all duration-300 font-mono animate-slideLeft ${getStyle(toast.type)}`}
          >
            <span className="shrink-0 mt-0.5">{getIcon(toast.type)}</span>
            <div className="flex flex-col space-y-0.5 flex-1 min-w-0">
              <span className="font-bold text-white tracking-wide text-[10px]">{toast.title}</span>
              <span className="text-[10px] leading-snug font-light opacity-85">{toast.message}</span>
            </div>
            <button
              onClick={() => setToasts((prev) => prev.filter((t) => t.id !== toast.id))}
              className="text-zinc-500 hover:text-zinc-300 text-[10px] p-0.5 outline-none shrink-0"
            >
              ✕
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextType {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within <ToastProvider>");
  return ctx;
}
