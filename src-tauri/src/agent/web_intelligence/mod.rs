// 对外信息搜索抓取与分析能力模块 (Web Intelligence Module)
//
// 核心功能：
// - 域名白名单管理：预置技术文档站点 + 用户自定义
// - Web Search：搜索引擎查询 (Bing API / 可配置后端)
// - Web Fetch：指定 URL 内容抓取 + HTML→Markdown 转换
// - Web Research：多源聚合分析 (搜索→抓取→蒸馏→总结)
// - Office Connect：办公系统连接器 (邮件/日历/任务只读)
//
// 安全约束（与 SecurityBoundary + ApprovalGate 联动）：
// - 域名白名单强制校验 — 非白名单域名拒绝请求
// - 所有外网操作通过第四红线审批门禁
// - 请求内容自动脱敏 (API Keys / 文件路径 / 个人信息)
// - 响应内容端侧蒸馏 — 仅喂结论给大模型，原始内容不入上下文
// - 全量审计日志 — 所有外网请求可追溯
//
// 设计原则：
//   1. 只读优先 (Read-Only by Default) — 绝不主动写入外网
//   2. 白名单约束 (Allowlist-Only) — 仅访问已批准的域名
//   3. 蒸馏优先 (Distill-before-LLM) — 外部数据经处理后才喂给大模型
//   4. 用户决策 (User-in-the-Loop) — 搜索目标、白名单、策略由用户控制

mod types;
mod core;
mod markdown;
mod commands;

pub use types::*;
pub use commands::*;

