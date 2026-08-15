// Command Palette (Ctrl+K) — 全局命令搜索
import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { MessageSquare, GitGraph, Link2, Puzzle, Globe, Route, Server, FolderOpen, Shield, Search, Settings, FilePlus, Save, Download, Trash2, Keyboard, Zap } from "lucide-react";
import { useT } from "@/lib/i18n-context";
import type { LocaleDict } from "@/lib/i18n";

export interface PaletteCommand {
  id: string; label: string; description: string;
  icon: React.ComponentType<{ className?: string }>;
  category: "Navigate" | "Session" | "Actions" | "Settings";
  keywords: string[]; action: () => void;
}

interface Props { commands: PaletteCommand[]; open: boolean; onClose: () => void; }

export default function CommandPalette({ commands, open, onClose }: Props) {
  const t = useT();
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
  const catLabel: Record<string,string> = { Navigate: t.cmd_cat_navigate, Session: t.cmd_cat_session, Actions: t.cmd_cat_actions, Settings: t.cmd_cat_settings };
  const catColor: Record<string,string> = { Navigate:"text-cyan-400", Session:"text-purple-400", Actions:"text-emerald-400", Settings:"text-amber-400" };

  return (
    <div className="fixed inset-0 z-[9999] flex items-start justify-center pt-[15vh] bg-black/70 backdrop-blur-sm" onClick={e => { if(e.target===e.currentTarget) onClose(); }}>
      <div className="w-[520px] bg-cs-header border border-cs-border rounded-xl shadow-2xl overflow-hidden animate-fadeIn">
        <div className="flex items-center px-4 py-3 border-b border-cs-border">
          <Search className="w-4 h-4 text-zinc-500 mr-3 shrink-0" />
          <input ref={inputRef} value={query} onChange={e => { setQuery(e.target.value); setSel(0); }} onKeyDown={handleKey}
            placeholder={t.cmd_search_placeholder} className="flex-1 bg-transparent text-sm text-white placeholder-zinc-600 outline-none" />
          <kbd className="text-[10px] text-zinc-600 bg-cs-bg border border-cs-border px-1.5 py-0.5 rounded ml-2">ESC</kbd>
        </div>
        <div ref={listRef} className="max-h-[360px] overflow-y-auto p-2">
          {filtered.length===0 && <div className="text-center py-8 text-zinc-600 text-sm">{t.cmd_no_results}</div>}
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
          <div className="flex items-center space-x-3"><span>{t.cmd_hint_nav}</span><span>{t.cmd_hint_exec}</span><span>{t.cmd_hint_close}</span></div>
          <span>{filtered.length} {t.cmd_count}</span>
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
}, t: LocaleDict): PaletteCommand[] {
  return [
    { id:"nav-chat",label:t.cmd_chat,description:t.cmd_chat_desc,icon:MessageSquare,category:"Navigate",keywords:["chat","对话"],action:()=>opts.onNavigate("chat")},
    { id:"nav-pipeline",label:t.cmd_pipeline,description:t.cmd_pipeline_desc,icon:GitGraph,category:"Navigate",keywords:["pipeline","流水线"],action:()=>opts.onNavigate("pipeline")},
    { id:"nav-glue",label:t.cmd_glue,description:t.cmd_glue_desc,icon:Link2,category:"Navigate",keywords:["glue","窗口"],action:()=>opts.onNavigate("glue")},
    { id:"nav-skills",label:t.cmd_skills,description:t.cmd_skills_desc,icon:Puzzle,category:"Navigate",keywords:["skill","mcp"],action:()=>opts.onNavigate("skills")},
    { id:"nav-webintel",label:t.cmd_webintel,description:t.cmd_webintel_desc,icon:Globe,category:"Navigate",keywords:["web","搜索"],action:()=>opts.onNavigate("webintel")},
    { id:"nav-autoroute",label:t.cmd_autoroute,description:t.cmd_autoroute_desc,icon:Route,category:"Navigate",keywords:["route","路由"],action:()=>opts.onNavigate("autoroute")},
    { id:"nav-remote",label:t.cmd_remote,description:t.cmd_remote_desc,icon:Server,category:"Navigate",keywords:["remote","ssh"],action:()=>opts.onNavigate("remote")},
    { id:"nav-explorer",label:t.cmd_explorer,description:t.cmd_explorer_desc,icon:FolderOpen,category:"Navigate",keywords:["files","沙盒"],action:()=>opts.onNavigate("explorer")},
    { id:"nav-approval",label:t.cmd_approval,description:t.cmd_approval_desc,icon:Shield,category:"Navigate",keywords:["approval","审批"],action:()=>opts.onNavigate("approval")},
    { id:"sess-new",label:t.cmd_sess_new,description:t.cmd_sess_new_desc,icon:FilePlus,category:"Session",keywords:["new","新建"],action:opts.onNewSession},
    { id:"sess-save",label:t.cmd_sess_save,description:t.cmd_sess_save_desc,icon:Save,category:"Session",keywords:["save","保存"],action:opts.onSaveSession},
    { id:"sess-export",label:t.cmd_sess_export,description:t.cmd_sess_export_desc,icon:Download,category:"Session",keywords:["export","导出"],action:opts.onExportSession},
    { id:"sess-clear",label:t.cmd_sess_clear,description:t.cmd_sess_clear_desc,icon:Trash2,category:"Session",keywords:["clear","清空"],action:opts.onClearAll},
    { id:"act-toggle",label:t.cmd_act_toggle,description:t.cmd_act_toggle_desc,icon:MessageSquare,category:"Actions",keywords:["sidebar","侧栏"],action:opts.onToggleSidebar},
    { id:"act-focus",label:t.cmd_act_focus,description:t.cmd_act_focus_desc,icon:Zap,category:"Actions",keywords:["focus","输入"],action:opts.onFocusInput},
    { id:"act-shortcuts",label:t.cmd_act_shortcuts,description:t.cmd_act_shortcuts_desc,icon:Keyboard,category:"Actions",keywords:["shortcut","快捷键"],action:opts.onShowShortcuts},
    { id:"set-mode",label:t.cmd_set_mode,description:t.cmd_set_mode_desc,icon:Route,category:"Settings",keywords:["mode","路由"],action:opts.onToggleRouteMode},
    { id:"set-open",label:t.cmd_set_open,description:t.cmd_set_open_desc,icon:Settings,category:"Settings",keywords:["settings","配置"],action:opts.onOpenSettings},
  ];
}
