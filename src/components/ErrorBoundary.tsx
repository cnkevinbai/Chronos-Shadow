// ErrorBoundary — 全局崩溃边界，防止单组件渲染错误导致整个应用白屏

import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error("[Chronos-Shadow] Render crash:", error.message);
    console.error("[Chronos-Shadow] Component stack:", info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-screen bg-[#09090b] text-[#fafafa] font-mono select-none">
          <div className="flex flex-col items-center space-y-6 max-w-md text-center px-6">
            <div className="text-4xl">⚠️</div>
            <h1 className="text-lg font-bold text-red-400">界面渲染异常</h1>
            <p className="text-sm text-zinc-400 leading-relaxed">
              应用遇到了一个未预期的渲染错误。请尝试刷新页面。
            </p>
            <code className="text-xs text-zinc-500 bg-[#121214] border border-[#27272a] rounded px-3 py-2 max-h-20 overflow-auto w-full">
              {this.state.error?.message ?? "未知错误"}
            </code>
            <button
              onClick={() => {
                this.setState({ hasError: false, error: null });
                window.location.reload();
              }}
              className="px-4 py-2 bg-red-500/20 border border-red-500/40 text-red-400 rounded hover:bg-red-500/30 transition-all duration-150 text-sm font-bold"
            >
              重新加载
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
