// 通用模态框组件
import { X } from "lucide-react";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
}

export default function Modal({ open, onClose, title, children }: ModalProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/70 backdrop-blur-sm"
        onClick={onClose}
      />
      {/* Content */}
      <div className="relative bg-cs-surface border border-cs-border rounded-lg shadow-2xl max-w-lg w-full mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-cs-border">
          <span className="text-[11px] font-bold text-cs-text">{title}</span>
          <button
            onClick={onClose}
            className="p-0.5 rounded hover:bg-cs-border/50 text-cs-muted hover:text-cs-text transition-colors"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
        {/* Body */}
        <div className="flex-1 overflow-auto p-4">{children}</div>
      </div>
    </div>
  );
}
