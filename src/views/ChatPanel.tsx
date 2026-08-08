// src/views/ChatPanel.tsx
// v5: async multi-agent flow monitoring with live stage indicators

import React, { useState, useEffect, useRef, useCallback } from "react";
import { useT } from "@/lib/i18n-context";
import { useToast } from "@/components/ToastProvider";
import { chatApiStream, onChatStreamChunk, getModelEndpoint, saveChatSessionChunk, loadChatSessionChunk, listHistoricalMetaManifests, deleteChatSession, exportChatSession, renameChatSession, importChatSession } from "@/lib/tauri";
import type { SessionMetaManifest } from "@/lib/types";
import { renderMarkdown } from "@/lib/utils";
import { createElement, type ReactNode } from "react";

interface Attachment { type: "doc"|"image"; name: string; sizeOrPath: string; }
interface Message { id: string; sender: "User"|"PM"|"UI Designer"|"Coder"|"System"|"Explore"|"Auditor"|"Scout"|"Compaction"; model: string; content: string; attachments?: Attachment[]; thinking?: string; costTokens?: number; isCached?: boolean; timestamp: string; cachingMarkerHash?: string; }
interface ChatPanelProps { selectedModel: string; apiKey?: string; hasKeys?: { deepseek: boolean; kimi: boolean; glm: boolean }; }

function modelDisplayName(model: string): string { const m: Record<string,string> = {"deepseek-v4-pro":"DeepSeek V4-Pro","deepseek-v4-flash":"DeepSeek V4-Flash","kimi-k3":"Kimi K3","kimi-k2.7-code":"Kimi K2.7-Code","kimi-k2.7-code-highspeed":"Kimi K2.7-Code-HS","glm-5.2":"GLM-5.2","glm-5v-turbo":"GLM-5V-Turbo","glm-5.1":"GLM-5.1","glm-4.7":"GLM-4.7"}; return m[model]??model; }

function deriveTitle(messages: Message[]): string { const u = messages.find(m=>m.sender==="User"); if(u){ const t=u.content.replace(/\s+/g," ").trim(); return t.length>24?t.slice(0,24)+"\u2026":t; } return "未命名研发 Quest"; }

type MdNode = string|{type:string;props:Record<string,unknown>};
function renderMdNode(node:MdNode):ReactNode{ if(typeof node==="string")return node; if(!node||typeof node!=="object")return null; const{type,props}=node; const{children,...rest}=props as Record<string,unknown>; const cn=Array.isArray(children)?children.map((c,i)=><React.Fragment key={i}>{renderMdNode(c as MdNode)}</React.Fragment>):children as ReactNode; return createElement(type as string,rest,cn); }
function MarkdownContent({text}:{text:string}){const nodes=renderMarkdown(text);return <div className="whitespace-pre-wrap font-medium">{nodes.map((n,i)=><React.Fragment key={i}>{renderMdNode(n)}</React.Fragment>)}</div>;}

export default function ChatPanel({selectedModel,apiKey="",hasKeys={deepseek:false,kimi:false,glm:false}}:ChatPanelProps){
  const kfm=(m:string)=>m.startsWith("deepseek")?hasKeys.deepseek:m.startsWith("kimi")?hasKeys.kimi:m.startsWith("glm")?hasKeys.glm:false;
  const currentHasKey=kfm(selectedModel); const anyKey=hasKeys.deepseek||hasKeys.kimi||hasKeys.glm;
  const availableProvider=hasKeys.deepseek?"DeepSeek":hasKeys.kimi?"Kimi":hasKeys.glm?"GLM":null;
  const t=useT(); const toast=useToast();
  const [manifests,setManifests]=useState<SessionMetaManifest[]>([]);
  const [activeSessionId,setActiveSessionId]=useState<string>(()=>`sess-${Date.now()}`);
  const [isSaving,setIsSaving]=useState(false);
  const refreshManifests=useCallback(()=>{listHistoricalMetaManifests().then(r=>{if(r)setManifests(r)}).catch(()=>{})},[]);
  useEffect(()=>{refreshManifests()},[refreshManifests]);
  const [input,setInput]=useState("");
  const [messages,setMessages]=useState<Message[]>([{id:"msg-0",sender:"System",model:"Local System",content:anyKey?t.chat_welcome_connected:t.chat_welcome_demo,timestamp:new Date().toLocaleTimeString()}]);
  useEffect(()=>{setMessages(p=>[{...p[0],content:anyKey?t.chat_welcome_connected:t.chat_welcome_demo},...p.slice(1)])},[anyKey,t]);
  const [isThinking,setIsThinking]=useState(false);
  const [flowStage,setFlowStage]=useState<"idle"|"connecting"|"thinking"|"streaming">("idle");
  const [flowStartMs,setFlowStartMs]=useState(0);
  const [flowTick,setFlowTick]=useState(0);
  useEffect(()=>{if(!isThinking)return;const iv=setInterval(()=>setFlowTick(t=>t+1),200);return()=>clearInterval(iv)},[isThinking]);
  const flowDots=".".repeat((flowTick%3)+1);
  const flowElapsed=isThinking?((Date.now()-flowStartMs)/1000).toFixed(1):"0.0";
  const stageLabel={idle:"",connecting:"连接中",thinking:"推理中",streaming:"流式接收"}[flowStage];
  const chatEndRef=useRef<HTMLDivElement>(null);
  const inputRef=useRef<HTMLInputElement>(null);
  const messagesRef=useRef<Message[]>(messages);
  useEffect(()=>{messagesRef.current=messages},[messages]);
  const [stagedAttachments,setStagedAttachments]=useState<Attachment[]>([]);
  const [lastUserInput,setLastUserInput]=useState("");
  const [retryCount,setRetryCount]=useState(0);
  const [showSlashMenu,setShowSlashMenu]=useState(false);
  const [showAtMenu,setShowAtMenu]=useState(false);
  const [searchOpen,setSearchOpen]=useState(false);
  const [sessionFilter,setSessionFilter]=useState("");
  const filteredManifests=sessionFilter.trim()?manifests.filter(m=>m.title.toLowerCase().includes(sessionFilter.toLowerCase())||(m.last_message_preview??"").toLowerCase().includes(sessionFilter.toLowerCase())):manifests;
  const persistCurrentSession=useCallback((msgs:Message[])=>{const p={meta:{session_id:activeSessionId,title:deriveTitle(msgs),bound_project:"Chronos-V4-Demo",last_updated:new Date().toISOString(),total_messages_count:msgs.length,total_accumulated_cost:msgs.reduce((a,m)=>a+(m.costTokens??0)*0.000001,0)},messages:msgs.map(m=>({id:m.id,sender:m.sender,model:m.model,content:m.content,thinking:m.thinking??null,cost_tokens:m.costTokens??0,timestamp:m.timestamp,caching_marker_hash:""}))}; saveChatSessionChunk(p).catch(()=>{})},[activeSessionId]);
  const handleNewSession=()=>{const id=`sess-${Date.now()}`;setActiveSessionId(id);setMessages([{id:"msg-0",sender:"System",model:"Local System",content:"新智能研发航道已开启。",timestamp:new Date().toLocaleTimeString()}]);};
  const handleSend=async(e:React.FormEvent)=>{e.preventDefault();if((!input.trim()&&stagedAttachments.length===0)||isThinking)return;const txt=input.trim();setInput("");setLastUserInput(txt);setRetryCount(0);setShowSlashMenu(false);setShowAtMenu(false);const userMsg:Message={id:`user-${Date.now()}`,sender:"User",model:"Human Operator",content:txt,attachments:stagedAttachments.length>0?[...stagedAttachments]:undefined,timestamp:new Date().toLocaleTimeString()};setStagedAttachments([]);const updated=[...messages,userMsg];setMessages(updated);setIsThinking(true);setFlowStage("connecting");setFlowStartMs(Date.now());try{const cms=messages.filter(m=>m.sender==="User"||m.sender==="Coder"||m.sender==="PM").map(m=>({role:m.sender==="User"?"user":"assistant",content:m.content}));cms.push({role:"user",content:txt});const ep=await getModelEndpoint(selectedModel);const sid=`stream-${Date.now()}`;const sp:Message={id:sid,sender:"Coder",model:`${modelDisplayName(selectedModel)} (Stream)`,content:"",costTokens:0,isCached:false,timestamp:new Date().toLocaleTimeString()};setMessages([...updated,sp]);let sc="";setFlowStage("thinking");const ul=await onChatStreamChunk(c=>{if(flowStage!=="streaming")setFlowStage("streaming");sc+=c;setMessages(p=>p.map(m=>m.id===sid?{...m,content:sc}:m))});let r;try{r=await chatApiStream(ep,apiKey,selectedModel,cms,4096)}finally{ul()}if(r.success){setMessages(p=>p.map(m=>m.id===sid?{...m,content:r.content||sc,costTokens:r.tokens_used,isCached:r.cached,model:`${modelDisplayName(selectedModel)} (API)`}:m));persistCurrentSession([...updated,{...sp,content:r.content||sc,costTokens:r.tokens_used,isCached:r.cached,model:`${modelDisplayName(selectedModel)} (API)`}]);refreshManifests()}else{setMessages(p=>p.filter(m=>m.id!==sid));const em:Message={id:`err-${Date.now()}`,sender:"System",model:"Error",content:`${t.chat_error_api}: ${r.error??"unknown"}`,timestamp:new Date().toLocaleTimeString()};setMessages([...updated,em]);persistCurrentSession([...updated,em])}}catch(err){const em:Message={id:`err-${Date.now()}`,sender:"System",model:"Error",content:`${t.chat_error_network}: ${err instanceof Error?err.message:String(err)}`,timestamp:new Date().toLocaleTimeString()};setMessages([...updated,em]);persistCurrentSession([...updated,em])}setIsThinking(false);setFlowStage("idle");setTimeout(()=>inputRef.current?.focus(),50)};

  return (<div className="flex h-full bg-[#09090b] font-mono text-xs text-[#fafafa] overflow-hidden">
    <div className="flex-1 flex flex-col">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-[#27272a] bg-[#121214] shrink-0">
        <span className="text-[10px] text-zinc-400 font-bold">
          {isThinking?`${stageLabel}${flowDots} ${flowElapsed}s`:currentHasKey?modelDisplayName(selectedModel):anyKey?`${t.agent_listening.replace("...","")} \u2014 ${availableProvider} \u53ef\u7528`:t.agent_listening}
        </span>
        <span className={isThinking?"text-[9px] text-cyan-400 animate-pulse":!currentHasKey?(anyKey?"text-[9px] text-cyan-400":"text-[9px] text-amber-500"):"text-[9px] text-emerald-400"}>
          {isThinking?"\u25cf \u5fc3\u6d41\u6fc0\u6d3b":!currentHasKey?(anyKey?`(\u5207\u6362\u81f3 ${availableProvider})`:"(Demo)"):"\u2713 \u5df2\u8fde\u63a5"}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-2" ref={msgContainerRef}>{messages.map(msg=>(<div key={msg.id} className={`p-2 border rounded ${msg.sender==="User"?"border-[#27272a] bg-black/20 ml-8":msg.sender==="System"?"border-zinc-800 bg-zinc-900/40 text-zinc-400 text-[10px]":"border-emerald-500/30 bg-emerald-950/10 mr-8"}`}><div className="flex items-center justify-between mb-1"><span className="text-[9px] font-bold text-zinc-500">{msg.sender}</span><span className="text-[8px] text-zinc-600">{msg.timestamp}</span></div><MarkdownContent text={msg.content}/></div>))}<div ref={chatEndRef}/></div>
      <form data-chat-form onSubmit={handleSend} className="flex items-center p-2 border-t border-[#27272a] bg-[#0c0c0e] shrink-0">
        <input ref={inputRef} value={input} onChange={e=>setInput(e.target.value)} disabled={isThinking} placeholder={t.chat_placeholder} className="flex-1 bg-black border border-[#27272a] rounded px-3 py-1.5 text-xs text-white focus:border-zinc-500 outline-none disabled:opacity-40"/>
        <button type="submit" disabled={isThinking||(!input.trim()&&stagedAttachments.length===0)} className="ml-2 px-3 py-1.5 bg-zinc-100 hover:bg-zinc-200 text-black font-bold text-xs rounded disabled:opacity-40">{isThinking?"\u231b":"\u2191"}</button>
      </form>
    </div>
  </div>);
}
