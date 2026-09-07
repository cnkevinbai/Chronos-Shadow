// src/lib/palette-commands.ts — Command Palette 命令表构建（纯函数，独立于 CommandPalette 组件文件）
import {
  MessageSquare, GitGraph, Link2, Puzzle, Globe, Route, Server,
  FolderOpen, Shield, FilePlus, Save, Download, Trash2, Keyboard, Zap, Settings,
} from "lucide-react";
import type { LocaleDict } from "@/lib/i18n";

export interface PaletteCommand {
  id: string; label: string; description: string;
  icon: React.ComponentType<{ className?: string }>;
  category: "Navigate" | "Session" | "Actions" | "Settings";
  keywords: string[]; action: () => void;
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
