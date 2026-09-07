// Command Palette (Ctrl+K) — 全局命令搜索
import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { Search } from "lucide-react";
import { useT } from "@/lib/i18n-context";
import type { PaletteCommand } from "@/lib/palette-commands";

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
          <kbd className="text-[10px] text-zinc-500 bg-cs-bg border border-cs-border px-1.5 py-0.5 rounded ml-2">ESC</kbd>
        </div>
        <div ref={listRef} className="max-h-[360px] overflow-y-auto p-2">
          {filtered.length===0 && <div className="text-center py-8 text-zinc-500 text-sm">{t.cmd_no_results}</div>}
          {cats.map(cat => {
            const items = filtered.filter(c => c.category===cat);
            if (!items.length) return null;
            return <div key={cat} className="mb-1">
              <div className={`text-[10px] ${catColor[cat]} px-2 py-1 uppercase tracking-wider font-bold`}>{catLabel[cat]}</div>
              {items.map(cmd => {
                const idx = filtered.indexOf(cmd); const isSel = idx===sel; const Icon = cmd.icon;
                return <button key={cmd.id} onClick={()=>{cmd.action();onClose()}} onMouseEnter={()=>setSel(idx)}
                  className={`w-full flex items-center px-2 py-2 rounded-md text-left transition-colors ${isSel?"bg-zinc-800/80 text-white":"text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-200"}`}>
                  <Icon className="w-4 h-4 mr-3 shrink-0" />
                  <div className="flex-1 min-w-0"><div className="text-sm font-medium truncate">{cmd.label}</div><div className="text-[10px] text-zinc-500 truncate">{cmd.description}</div></div>
                  {isSel && <kbd className="text-[10px] text-zinc-500 ml-2">↵</kbd>}
                </button>;
              })}
            </div>;
          })}
        </div>
        <div className="flex items-center justify-between px-4 py-2 border-t border-cs-border text-[10px] text-zinc-500">
          <div className="flex items-center space-x-3"><span>{t.cmd_hint_nav}</span><span>{t.cmd_hint_exec}</span><span>{t.cmd_hint_close}</span></div>
          <span>{filtered.length} {t.cmd_count}</span>
        </div>
      </div>
    </div>
  );
}

