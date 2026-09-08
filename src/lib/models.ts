// Chronos-Shadow 统一模型注册表 (Single Source of Truth)
// 能力矩阵核验状态（2026-09-08）：
//   ✅ 已按官方模型库核验：kimi-k3（1M 窗口/2.8T/原生视觉/缓存 $0.30 命中）、
//      kimi-k2.7-code(-highspeed)（256K/Context Caching）、glm-4.7（200K/缓存 $0.11/M）
//      —— 来源：platform.kimi.com/docs/api/models-overview、docs.bigmodel.cn、docs.z.ai
//   ⚠️ 项目自有型号（deepseek-v4-pro/flash、glm-5.2/5.1、glm-5v-turbo）在官方模型库
//      无对应页面，contextWindow 为项目声明值【待核验，无官方资料】——官方基线供参考：

export interface ModelEntry {
  key: string; display: string; shortDisplay: string;
  provider: "deepseek" | "kimi" | "glm" | "ollama"; isVision: boolean;
  contextWindow: number; supportsCache: boolean;
  costTier: "premium" | "standard" | "budget" | "free"; bestFor: string;
}
export const MODELS: ModelEntry[] = [
  { key:"deepseek-v4-pro",display:"DeepSeek V4-Pro (深度推理)",shortDisplay:"DeepSeek V4-Pro",provider:"deepseek",isVision:false,contextWindow:1000000,supportsCache:true,costTier:"premium",bestFor:"1M 上下文深度推理·1.6T MoE（官方 V4 系默认 1M 窗口）" },
  { key:"deepseek-v4-flash",display:"DeepSeek V4-Flash (代码生成)",shortDisplay:"DeepSeek V4-Flash",provider:"deepseek",isVision:false,contextWindow:1000000,supportsCache:true,costTier:"budget",bestFor:"1M 上下文默认写码审计·284B-A13B MoE·一折缓存命中（legacy deepseek-chat/reasoner 已退役路由至此）" },
  { key:"kimi-k3",display:"Kimi K3 (1M 超长上下文)",shortDisplay:"Kimi K3",provider:"kimi",isVision:false,contextWindow:1048576,supportsCache:true,costTier:"premium",bestFor:"1M 上下文超长项目分析·2.8T 参数·原生视觉理解（本项目用作文本主模型）" },
  { key:"kimi-k2.7-code",display:"Kimi K2.7-Code (代码专用)",shortDisplay:"Kimi K2.7-Code",provider:"kimi",isVision:false,contextWindow:262144,supportsCache:true,costTier:"standard",bestFor:"稳定写码" },
  { key:"kimi-k2.7-code-highspeed",display:"Kimi K2.7-Code-HS (极速编程)",shortDisplay:"Kimi K2.7-Code-HS",provider:"kimi",isVision:false,contextWindow:262144,supportsCache:true,costTier:"standard",bestFor:"紧急编译阻断极速写码" },
  // 项目声明型号【GLM-5.3-Flash 待官方核验】
  { key:"glm-5.3-flash",display:"GLM-5.3-Flash (1M 原生多模态)",shortDisplay:"GLM-5.3-Flash",provider:"glm",isVision:true,contextWindow:1048576,supportsCache:true,costTier:"standard",bestFor:"1M 上下文原生多模态（文/图/视频/文件）·320B-A18B MoE·混合稀疏+线性注意力" },
  { key:"glm-5.2",display:"GLM-5.2 (原生Agent规划)",shortDisplay:"GLM-5.2",provider:"glm",isVision:false,contextWindow:128000,supportsCache:false,costTier:"standard",bestFor:"原生大模型工具链极速编排" },
  // 项目声明型号【官方模型库暂无对应页面——官方视觉基线：GLM-4.6V 128K】
  { key:"glm-5v-turbo",display:"GLM-5V-Turbo (高精视觉)",shortDisplay:"GLM-5V-Turbo",provider:"glm",isVision:true,contextWindow:32768,supportsCache:false,costTier:"premium",bestFor:"视觉多模态全能走查" },
  // 项目声明型号【官方模型库暂无对应页面】
  { key:"glm-5.1",display:"GLM-5.1 (稳定推理)",shortDisplay:"GLM-5.1",provider:"glm",isVision:false,contextWindow:128000,supportsCache:false,costTier:"standard",bestFor:"稳定推理·生产环境" },
  { key:"glm-4.7",display:"GLM-4.7 (高性价比·200K)",shortDisplay:"GLM-4.7",provider:"glm",isVision:false,contextWindow:204800,supportsCache:true,costTier:"budget",bestFor:"高性价比·日常推理" },
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
