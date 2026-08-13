// Chronos-Shadow 统一模型注册表 (Single Source of Truth)
export interface ModelEntry {
  key: string; display: string; shortDisplay: string;
  provider: "deepseek" | "kimi" | "glm" | "ollama"; isVision: boolean;
  contextWindow: number; supportsCache: boolean;
  costTier: "premium" | "standard" | "budget" | "free"; bestFor: string;
}
export const MODELS: ModelEntry[] = [
  { key:"deepseek-v4-pro",display:"DeepSeek V4-Pro (深度推理)",shortDisplay:"DeepSeek V4-Pro",provider:"deepseek",isVision:false,contextWindow:131072,supportsCache:true,costTier:"premium",bestFor:"长文本理解与慢思考推理" },
  { key:"deepseek-v4-flash",display:"DeepSeek V4-Flash (代码生成)",shortDisplay:"DeepSeek V4-Flash",provider:"deepseek",isVision:false,contextWindow:65536,supportsCache:true,costTier:"budget",bestFor:"默认写码审计：一折Caching缓存" },
  { key:"kimi-k3",display:"Kimi K3 (超长项目分析)",shortDisplay:"Kimi K3",provider:"kimi",isVision:false,contextWindow:65536,supportsCache:false,costTier:"premium",bestFor:"超长项目分析长文本" },
  { key:"kimi-k2.7-code",display:"Kimi K2.7-Code (代码专用)",shortDisplay:"Kimi K2.7-Code",provider:"kimi",isVision:false,contextWindow:65536,supportsCache:false,costTier:"standard",bestFor:"稳定写码" },
  { key:"kimi-k2.7-code-highspeed",display:"Kimi K2.7-Code-HS (极速编程)",shortDisplay:"Kimi K2.7-Code-HS",provider:"kimi",isVision:false,contextWindow:65536,supportsCache:false,costTier:"standard",bestFor:"紧急编译阻断极速写码" },
  { key:"glm-5.2",display:"GLM-5.2 (原生Agent规划)",shortDisplay:"GLM-5.2",provider:"glm",isVision:false,contextWindow:128000,supportsCache:false,costTier:"standard",bestFor:"原生大模型工具链极速编排" },
  { key:"glm-5v-turbo",display:"GLM-5V-Turbo (高精视觉)",shortDisplay:"GLM-5V-Turbo",provider:"glm",isVision:true,contextWindow:32768,supportsCache:false,costTier:"premium",bestFor:"视觉多模态全能走查" },
  { key:"glm-5.1",display:"GLM-5.1 (稳定推理)",shortDisplay:"GLM-5.1",provider:"glm",isVision:false,contextWindow:128000,supportsCache:false,costTier:"standard",bestFor:"稳定推理·生产环境" },
  { key:"glm-4.7",display:"GLM-4.7 (高性价比)",shortDisplay:"GLM-4.7",provider:"glm",isVision:false,contextWindow:32768,supportsCache:false,costTier:"budget",bestFor:"高性价比·日常推理" },
  { key:"ollama-local",display:"Ollama Local (0资费)",shortDisplay:"Ollama Local",provider:"ollama",isVision:false,contextWindow:8192,supportsCache:false,costTier:"free",bestFor:"LAN离线降级热备" },
];
export function getModel(key:string){return MODELS.find(m=>m.key===key)}
export function getModelDisplay(key:string){return getModel(key)?.shortDisplay??key}
export function getLLMs(){return MODELS.filter(m=>!m.isVision)}
export function getVLMs(){return MODELS.filter(m=>m.isVision)}

export interface ModelClassification {
  llms: string[];
  vlms: string[];
  unknown: string[];
}

/** 将 Rust 后端返回的模型 key 列表按注册表 isVision 分类，并标记注册表中缺失的模型。 */
export function classifyModelKeys(keys: string[]): ModelClassification {
  const llms: string[] = [];
  const vlms: string[] = [];
  const unknown: string[] = [];
  for (const k of keys) {
    const entry = getModel(k);
    if (!entry) { unknown.push(k); continue; }
    (entry.isVision ? vlms : llms).push(k);
  }
  return { llms, vlms, unknown };
}
