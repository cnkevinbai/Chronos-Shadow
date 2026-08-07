// 智能意图捕获 — 一键宏指令卡片推荐

import { useT } from "@/lib/i18n-context";

interface QuickMacrosProps {
  onSelect: (prompt: string) => void;
  visible: boolean;
}

const MACROS = [
  { id: "login", icon: "✨", label: "帮我重构登录面板并跑通本地 CI/CD" },
  { id: "kanban", icon: "📋", label: "为当前项目生成一个看板管理组件" },
  { id: "api", icon: "🔌", label: "帮我封装后端 RESTful 接口并生成 OpenAPI 文档" },
  { id: "audit", icon: "🛡️", label: "对项目执行一次全量安全审计扫描" },
  { id: "deploy", icon: "🚀", label: "打包 Docker 镜像并推送到私有仓库" },
];

export default function QuickMacros({ onSelect, visible }: QuickMacrosProps) {
  const t = useT();
  if (!visible) return null;

  return (
    <div className="absolute bottom-full left-0 right-0 mb-2 px-4 animate-slideLeft">
      <div className="bg-[#121214]/98 border border-[#27272a] rounded-lg p-3 shadow-2xl backdrop-blur-md">
        <div className="text-[10px] text-zinc-500 font-bold mb-2 uppercase tracking-wider">
          {t.macros_title}
        </div>
        <div className="flex flex-wrap gap-2">
          {MACROS.map((m) => (
            <button
              key={m.id}
              onClick={() => onSelect(m.label)}
              className="flex items-center space-x-1.5 bg-black border border-[#27272a] hover:border-emerald-500/50 hover:bg-emerald-950/20 text-zinc-300 hover:text-emerald-400 text-[11px] px-3 py-1.5 rounded-lg transition-all"
            >
              <span>{m.icon}</span>
              <span className="text-left">{m.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
