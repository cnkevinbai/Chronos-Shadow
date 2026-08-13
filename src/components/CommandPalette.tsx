// Command Palette (Ctrl+K) — 全局命令搜索
import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { MessageSquare, GitGraph, Link2, Puzzle, Globe, Route, Server, FolderOpen, Shield, Search, Settings, FilePlus, Save, Download, Trash2, Keyboard, Zap } from "lucide-react";

export interface PaletteCommand {
  id: string; label: string; description: string;
  icon: React.ComponentType<{ className?: string }>;
  category: "Navigate" | "Session" | "Actions" | "Settings";
  keywords: string[]; action: () => void;
}

interface Props { commands: PaletteCommand[]; open: boolean; onClose: () => void; }

export default function CommandPalette({ commands, open, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(() => {
    if (!query.trim()) return commands;
    const q = query.toLowerCase();
    return commands.filter(c => c.label.toLowerCase().includes(q) || c.description.toLowerCase().includes(q) || c.keywords.some(k => k.includes(q)));
  }, [commands, query]);

  useEffect(() => { if (open) { setQuery(""); setSel(0); setTimeout(() => inputRef.current?.focus(), 50); } }, [open]);

  const handleKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setSel(i => Math.min(i+1, filtered.length-1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setSel(i => Math.max(i-1,0)); }
    else if (e.key === "Enter") { e.preventDefault(); if (filtered[sel]) { filtered[sel].action(); onClose(); } }
    else if (e.key === "Escape") onClose();
  }, [filtered, sel, onClose]);

  useEffect(() => { if (listRef.current) { const el = listRef.current.children[sel] as HTMLElement|undefined; el?.scrollIntoView({block:"nearest"}); } }, [sel]);

  if (!open) return null;

  const cats = ["Navigate","Session","Actions","Settings"] as const;
  const catLabel: Record<string,string> = { Navigate:"导航", Session:"会话", Actions:"操作", Settings:"设置" };
  const catColor: Record<string,string> = { Navigate:"text-cyan-400", Session:"text-purple-400", Actions:"text-emerald-400", Settings:"text-amber-400" };

  return (
    <div className="fixed inset-0 z-[9999] flex items-start justify-center pt-[15vh] bg-black/70 backdrop-blur-sm" onClick={e => { if(e.target===e.currentTarget) onClose(); }}>
      <div className="w-[520px] bg-cs-header border border-cs-border rounded-xl shadow-2xl overflow-hidden animate-fadeIn">
        <div className="flex items-center px-4 py-3 border-b border-cs-border">
          <Search className="w-4 h-4 text-zinc-500 mr-3 shrink-0" />
          <input ref={inputRef} value={query} onChange={e => { setQuery(e.target.value); setSel(0); }} onKeyDown={handleKey}
            placeholder="输入命令名称搜索…" className="flex-1 bg-transparent text-sm text-white placeholder-zinc-600 outline-none" />
          <kbd className="text-[10px] text-zinc-600 bg-cs-bg border border-cs-border px-1.5 py-0.5 rounded ml-2">ESC</kbd>
        </div>
        <div ref={listRef} className="max-h-[360px] overflow-y-auto p-2">
          {filtered.length===0 && <div className="text-center py-8 text-zinc-600 text-sm">未找到匹配命令</div>}
          {cats.map(cat => {
            const items = filtered.filter(c => c.category===cat);
            if (!items.length) return null;
            return <div key={cat} className="mb-1">
              <div className={`text-[9px] ${catColor[cat]} px-2 py-1 uppercase tracking-wider font-bold`}>{catLabel[cat]}</div>
              {items.map(cmd => {
                const idx = filtered.indexOf(cmd); const isSel = idx===sel; const Icon = cmd.icon;
                return <button key={cmd.id} onClick={()=>{cmd.action();onClose()}} onMouseEnter={()=>setSel(idx)}
                  className={`w-full flex items-center px-2 py-2 rounded-md text-left transition-colors ${isSel?"bg-zinc-800/80 text-white":"text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-200"}`}>
                  <Icon className="w-4 h-4 mr-3 shrink-0" />
                  <div className="flex-1 min-w-0"><div className="text-sm font-medium truncate">{cmd.label}</div><div className="text-[10px] text-zinc-600 truncate">{cmd.description}</div></div>
                  {isSel && <kbd className="text-[9px] text-zinc-500 ml-2">↵</kbd>}
                </button>;
              })}
            </div>;
          })}
        </div>
        <div className="flex items-center justify-between px-4 py-2 border-t border-cs-border text-[9px] text-zinc-600">
          <div className="flex items-center space-x-3"><span>↑↓ 导航</span><span>↵ 执行</span><span>ESC 关闭</span></div>
          <span>{filtered.length} 条命令</span>
        </div>
      </div>
    </div>
  );
}

export function buildPaletteCommands(opts: {
  onNavigate: (v: string) => void; onNewSession: () => void; onSaveSession: () => void;
  onExportSession: () => void; onClearAll: () => void; onToggleSidebar: () => void;
  onFocusInput: () => void; onToggleRouteMode: () => void;
  onOpenSettings: () => void; onShowShortcuts: () => void;
}): PaletteCommand[] {
  return [
    { id:"nav-chat",label:"全局对话",description:"AI 对话面板",icon:MessageSquare,category:"Navigate",keywords:["chat","对话"],action:()=>opts.onNavigate("chat")},
    { id:"nav-pipeline",label:"调度流水线",description:"7-Agent SDLC",icon:GitGraph,category:"Navigate",keywords:["pipeline","流水线"],action:()=>opts.onNavigate("pipeline")},
    { id:"nav-glue",label:"跨软件粘合",description:"WorkBuddy 窗口绑定",icon:Link2,category:"Navigate",keywords:["glue","窗口"],action:()=>opts.onNavigate("glue")},
    { id:"nav-skills",label:"技能中枢",description:"技能与MCP管理",icon:Puzzle,category:"Navigate",keywords:["skill","mcp"],action:()=>opts.onNavigate("skills")},
    { id:"nav-webintel",label:"Web智能搜索",description:"搜索/抓取/研究",icon:Globe,category:"Navigate",keywords:["web","搜索"],action:()=>opts.onNavigate("webintel")},
    { id:"nav-autoroute",label:"自动路由",description:"关键词→Agent路由",icon:Route,category:"Navigate",keywords:["route","路由"],action:()=>opts.onNavigate("autoroute")},
    { id:"nav-remote",label:"远程服务器",description:"SSH编译管理",icon:Server,category:"Navigate",keywords:["remote","ssh"],action:()=>opts.onNavigate("remote")},
    { id:"nav-explorer",label:"项目沙盒",description:"文件树/检查点",icon:FolderOpen,category:"Navigate",keywords:["files","沙盒"],action:()=>opts.onNavigate("explorer")},
    { id:"nav-approval",label:"审批门禁",description:"第四红线安全审批",icon:Shield,category:"Navigate",keywords:["approval","审批"],action:()=>opts.onNavigate("approval")},
    { id:"sess-new",label:"新建会话",description:"开启空白研发航道",icon:FilePlus,category:"Session",keywords:["new","新建"],action:opts.onNewSession},
    { id:"sess-save",label:"保存会话",description:"固化当前对话到磁盘",icon:Save,category:"Session",keywords:["save","保存"],action:opts.onSaveSession},
    { id:"sess-export",label:"导出会话JSON",description:"导出当前会话",icon:Download,category:"Session",keywords:["export","导出"],action:opts.onExportSession},
    { id:"sess-clear",label:"清空全部会话",description:"删除所有历史档案",icon:Trash2,category:"Session",keywords:["clear","清空"],action:opts.onClearAll},
    { id:"act-toggle",label:"切换侧栏",description:"展开/收起历史会话",icon:MessageSquare,category:"Actions",keywords:["sidebar","侧栏"],action:opts.onToggleSidebar},
    { id:"act-focus",label:"聚焦输入框",description:"光标移动到输入框",icon:Zap,category:"Actions",keywords:["focus","输入"],action:opts.onFocusInput},
    { id:"act-shortcuts",label:"快捷键帮助",description:"查看全部快捷键",icon:Keyboard,category:"Actions",keywords:["shortcut","快捷键"],action:opts.onShowShortcuts},
    { id:"set-mode",label:"切换路由模式",description:"自动/手动路由",icon:Route,category:"Settings",keywords:["mode","路由"],action:opts.onToggleRouteMode},
    { id:"set-open",label:"全局配置",description:"API密钥/成本风控",icon:Settings,category:"Settings",keywords:["settings","配置"],action:opts.onOpenSettings},
  ];
}
