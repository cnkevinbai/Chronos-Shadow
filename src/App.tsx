// Chronos-Shadow 主框架

import { useState, useEffect, useCallback } from "react";
import { I18nProvider, useT } from "@/lib/i18n-context";
import SdlcPipelinePanel from "@/views/SdlcPipelinePanel";
import ProjectExplorer from "@/views/ProjectExplorer";
import ChatPanel from "@/views/ChatPanel";
import SettingsPanel from "@/views/SettingsPanel";
import EvolutionConsole from "@/views/EvolutionConsole";
import AppGlueBinder from "@/views/AppGlueBinder";
import SkillMcpHub from "@/views/SkillMcpHub";
import RemoteHub from "@/views/RemoteHub";
import RedlineGuardPanel from "@/views/RedlineGuardPanel";
import SecurityShieldPanel from "@/components/SecurityShieldPanel";
import FooterBar from "@/components/FooterBar";
import FloatingBubble from "@/components/FloatingBubble";
import ErrorBoundary from "@/components/ErrorBoundary";
import { ToastProvider, useToast } from "@/components/ToastProvider";
import { ChatIcon, PipelineIcon, GlueIcon, McpIcon, ChronosFolderIcon, RemoteIcon, ChronosLogo } from "@/components/SvgIcons";
import { getSessionCost, getSavedCost, getSavingRate, getBuddySaved, loadSettings, getSandboxStatus, getRedlineStatus, getPipelineStats, startPipeline, pausePipeline, resumePipeline, advancePipeline, onPipelineEvent, getAvailableModels, setRouteMode as setRouteModeIpc } from "@/lib/tauri";
import type { RedlineStatus, OrchestratorStats } from "@/lib/types";

function modelLabel(m: string): string {
  const labels: Record<string, string> = {"deepseek-v4-pro":"DeepSeek V4-Pro","deepseek-v4-flash":"DeepSeek V4-Flash","kimi-k3":"Kimi K3","kimi-k2.7-code":"Kimi K2.7-Code","kimi-k2.7-code-highspeed":"Kimi K2.7-Code-HS","glm-5.2":"GLM-5.2","glm-5v-turbo":"GLM-5V-Turbo","glm-5.1":"GLM-5.1","glm-4.7":"GLM-4.7","ollama-local":"Ollama Local"};
  return labels[m] ?? m;
}

function AppInner() {
  const t = useT(); const toast = useToast();
  const [activeView, setActiveView] = useState<"workbench"|"settings"|"evolution">("workbench");
  const [dockView, setDockView] = useState<"chat"|"pipeline"|"glue"|"skills"|"remote"|"explorer">("chat");
  const [routeMode, setRouteMode] = useState<"auto"|"manual">("auto");
  const [selectedLLM, setSelectedLLM] = useState("deepseek-v4-pro");
  const [selectedVLM, setSelectedVLM] = useState("glm-5v-turbo");
  const [hasKeys, setHasKeys] = useState({deepseek:false,kimi:false,glm:false});

  useEffect(() => {
    let tid1: ReturnType<typeof setTimeout>|undefined; let tid2: ReturnType<typeof setTimeout>|undefined;
    const load = async () => { try { const s = await loadSettings(); setHasKeys({deepseek:s.has_key_deepseek??false,kimi:s.has_key_kimi??false,glm:s.has_key_glm??false}); } catch { tid2 = setTimeout(async () => { try { const s = await loadSettings(); setHasKeys({deepseek:s.has_key_deepseek??false,kimi:s.has_key_kimi??false,glm:s.has_key_glm??false}); } catch {} }, 1000); } };
    tid1 = setTimeout(load, 500);
    return () => { if(tid1) clearTimeout(tid1); if(tid2) clearTimeout(tid2); };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-[#09090b] text-[#fafafa] font-mono select-none">
      <header className="flex items-center justify-between px-4 py-2.5 border-b border-[#27272a] bg-[#121214] shrink-0"><div className="flex items-center space-x-4"><div className="flex items-center space-x-2"><ChronosLogo size={20} className="stroke-cyan-400"/><span className="font-bold text-sm tracking-wider text-white">{t.app_title}</span></div></div></header>
      <div className="flex-1 flex overflow-hidden">{activeView==="workbench"?(<div className="flex flex-1 overflow-hidden"><nav className="w-12 border-r border-[#27272a] bg-[#0c0c0e] flex flex-col items-center py-3 space-y-2 shrink-0"><DockButton active={dockView==="chat"} tip="chat" onClick={()=>setDockView("chat")}><ChatIcon size={18}/></DockButton><DockButton active={dockView==="pipeline"} tip="pipeline" onClick={()=>setDockView("pipeline")}><PipelineIcon size={18}/></DockButton><DockButton active={dockView==="glue"} tip="glue" onClick={()=>setDockView("glue")}><GlueIcon size={18}/></DockButton><DockButton active={dockView==="skills"} tip="skills" onClick={()=>setDockView("skills")}><McpIcon size={18}/></DockButton><DockButton active={dockView==="remote"} tip="remote" onClick={()=>setDockView("remote")}><RemoteIcon size={18}/></DockButton><DockButton active={dockView==="explorer"} tip="explorer" onClick={()=>setDockView("explorer")}><ChronosFolderIcon size={18}/></DockButton></nav><section className="flex-1 bg-[#09090b] overflow-hidden flex flex-col">{dockView==="chat"&&(<ChatPanel selectedModel={selectedLLM} apiKey="" hasKeys={hasKeys}/>)}{dockView==="pipeline"&&(<SdlcPipelinePanel/>)}{dockView==="glue"&&(<AppGlueBinder/>)}{dockView==="skills"&&(<SkillMcpHub/>)}{dockView==="remote"&&(<RemoteHub/>)}{dockView==="explorer"&&(<ProjectExplorer/>)}</section></div>):activeView==="evolution"?(<EvolutionConsole/>):(<SettingsPanel hasKeys={hasKeys} onKeyChange={(p,h)=>setHasKeys(prev=>({...prev,[p]:h}))}/>)}</div>
      <FooterBar/>
    </div>
  );
}

export default function App(){return(<I18nProvider><ToastProvider><ErrorBoundary><AppInner/></ErrorBoundary></ToastProvider></I18nProvider>);}
function DockButton({active,tip,onClick,children}:{active:boolean;tip:string;onClick:()=>void;children:React.ReactNode}){return(<button onClick={onClick} title={tip} className={`w-9 h-9 flex items-center justify-center rounded ${active?"bg-[#27272a] text-white":"text-zinc-500"}`}>{children}</button>);}
