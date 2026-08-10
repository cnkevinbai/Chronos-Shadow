# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-08-10

### Added — 新增 7 大核心引擎

#### 🌐 Web 智能搜索分析引擎 (`web_intelligence.rs`, 1459行)
- 域名白名单管理：预置30+官方文档+技术社区域名，支持增删改查
- Web 搜索引擎查询：Bing / DuckDuckGo 双引擎，结果缓存(10min TTL)
- 网页抓取 + HTML→Markdown 自动转换 + 内容安全脱敏
- 多源聚合研究：搜索→抓取→蒸馏→总结全自动管道
- 全量审计日志：所有外网请求可追溯，域名白名单强制校验

#### 🧪 多级语义蒸馏引擎 (`distillation_engine.rs`, 1503行)
- 三级蒸馏：Light(结构保持) / Medium(语义提取) / Deep(知识压缩)
- 7维强化学习权重自动进化 (lr=0.05, reward函数)
- 12种实体自动提取：版本/日期/Crate/包名/仓库/许可证/废弃/破坏性变更
- 自适应策略表：按内容类型(code/doc/blog/mixed)自动调参
- 内容类型自动检测 + LRU缓存
- 质量反馈闭环 + EMA更新策略

#### 🔄 统一缓存引擎 (`cache_engine.rs`, 626行)
- 5分类独立TTL：搜索(10min)/抓取(1h)/蒸馏(1d)/LLM响应(1h)/通用(5min)
- 自适应TTL：命中率>80%→TTL×1.5，命中率<20%→TTL×0.7
- LRU淘汰 + 128MB内存上限 + 10K条目限制
- 磁盘持久化 + 全维度统计

#### 🤖 多模型协作引擎 (`collaboration_engine.rs`, 425行)
- 4种协作模式：Single/Parallel/Voting/Cascade/DivideAndConquer
- 5模型能力画像：deepseek-v4-pro/flash, kimi-k3/k2.7-code, glm-5.2
- 分任务类型质量评分 + EMA质量反馈
- 自动降级切换 + 成本优化选择

#### 📊 任务智能分解引擎 (`task_intelligence.rs`, 470行)
- 7种任务模板：代码实现/修复/设计/重构/测试/调研/安全审计
- 5级复杂度估算(4维特征评分) + 拓扑并行组检测
- 智能分解：依赖图 + 自动Agent匹配 + 成本/时间预估

#### 🔮 预测分析引擎 (`predictive_analytics.rs`, 747行)
- Holt-Winters 季节性Token用量预测
- SPC统计过程控制成本异常检测
- 贝叶斯预算优化 + K-means使用模式聚类
- EMA指数平滑 + 简单预测API

#### 🧬 端侧进化总线 (`evolution_bus.rs`, 480行)
- 9引擎统一注册表 + 进化事件日志(1000条)
- 反馈环路：自动调参 + 安全clamp保护(±15%/cycle)
- 跨引擎知识迁移 + 每小时自动评估先进性
- 持久化 + 健康报告

### Enhanced — 5 个核心模块增强

#### 调度引擎 v3 (`scheduling_engine.rs`, +284行)
- TF-IDF 加权意图分类(13类 × 100+关键词)
- 贝叶斯置信度更新(Beta分布平滑)
- Bigram/Trigram精确模式匹配(6种意图)
- 紧急度估算(15种信号)

#### 安全边界 (`security_boundary.rs`, +97行)
- 6个新操作类型：WebSearch/WebFetchReadonly/ApiCallReadonly(需审批)
- SocialPostWrite/DataUploadExternal(永远禁止)
- LLM输出检测逻辑细化：区分合法搜索与恶意外泄

#### 审批门禁 (`approval_gate.rs`, +56行)
- 3个新审批规则：WebSearch/WebFetch/ApiCall
- 低风险自动放行(阈值5/4/3)
- 四维风险画像(影响范围1/可逆性10/资费2/合规2-3)

#### Redline Schema (`redline.rs`, +74行)
- WebSearch/WebFetch 操作类型 + URL安全校验
- HTTPS强制 + SQL注入检测

#### 防幻觉引擎 (`hallucination_guard.rs`, +126行)
- 10维自适应灵敏度调节
- 误报率反馈学习(目标15%)
- EvolutionBus联动

### Added — 前端

- **WebIntelligencePanel**: 5Tab(搜索/抓取/研究/域名/审计) + 12项统计指标
- **AutoRoutingPanel**: 3Tab(路由规则/模型矩阵/Agent映射) + 搜索过滤
- **EvolutionConsole**: 进化总线健康面板(5引擎实时评分)
- ChatPanel: 行动调度引擎(auto-detect→execute→follow-up)
- i18n扩展: 333个翻译键，中英文全覆盖
- 前端打包: 1815 modules, 469KB JS, 84KB CSS

### Fixed — 编译与运行时修复
- `regex-lite` crate 添加到 Cargo.toml
- `DistillationLevel` 缺少 `Hash` derive
- `scheduling_engine.rs` impl块括号错位
- `cache_engine.rs` 借用冲突 → 重构返回类型为 `Option<String>`
- `distillation_engine.rs` move/clone问题 → 预保存长度
- `lib.rs` MutexGuard Send问题 → scope drop模式
- `predictive_analytics.rs` 5个类型错误
- `agent_evolution.rs` rand::random → chrono时间戳ID
- `mcp_client.rs` 测试 mut 关键字缺失
- EXE启动崩溃：`tokio::spawn` → `tauri::async_runtime::spawn`
- 界面显示dist目录索引：`frontendDist`绝对路径 → 相对路径 `"../dist"`

### Security
- 域名白名单 Deny-by-Default：数据外泄/社交发布/外网上传永远禁止
- Web搜索/抓取默认需审批，低风险自动放行
- HTTPS强制 + URL格式校验
- 请求内容自动脱敏(API Keys/文件路径/个人信息)
- 响应端侧蒸馏：仅喂结论给LLM，原始网页不进上下文

## [0.1.1] — 2026-08-07

- Tauri v2 + React 19 + Tailwind CSS 4 框架
- 多模型混合路由 (DeepSeek / Kimi / GLM / Ollama)
- 7-Agent SDLC 流水线编排
- 三红线防幻觉拦截器
- AES-256-GCM 会话加密框架
- Windows Credential Manager 原生 FFI
- 三层并行计费引擎
- 远程 SSH 代理 + 集群管理
- MCP JSON-RPC 2.0 协议客户端
- GitHub Actions CI/CD (Windows MSI)
- 9 个前端面板 + 11 个组件

[Unreleased]: https://github.com/cnkevinbai/Chronos-Shadow/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.2.0
[0.1.1]: https://github.com/cnkevinbai/Chronos-Shadow/releases/tag/v0.1.1