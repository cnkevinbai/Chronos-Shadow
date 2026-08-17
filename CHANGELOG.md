# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.0] — 2026-08-17

### Added — 运行模式 + 推理深度
- 运行模式选择：plan / review / auto / yolo，映射到「四红线 + 审批门禁」自主级别（Plan 只计划、Review 每步审批、Auto 低风险自动、Yolo 跳过全部校验）
- 推理深度切换：low / medium / high，映射到真实 `max_tokens` + `temperature`（2048/0.7、4096/0.3、8192/0.1）

### Changed — 引擎能力去桩（升级评估落地）
- `win_hooks` 事件转发：原子计数 + mpsc 通道 + GetMessageW 消息泵（原 trace 空转）
- `subagents` scout 真实化：文件遍历 + 正则符号提取 / HTTP 抓取 + HTML→Markdown（原 mock 假数据）
- `consolidator` 真实 SQLite：rusqlite(bundled) 持久化，替换 JSON 全量读写
- `buddy_scan` OCR 脚手架 + 诚实 fail-closed 回退（原像素哈希假 OCR）
- `vision` ONNX 格式校验 `is_valid_onnx`（原尺寸启发式）

### Added — 其他
- i18n 浏览器语言自动探测 + CommandPalette 全量本地化
- 前端测试扩充至 29 个（ToastProvider / DiffViewer）

## [0.3.0] — 2026-08-14

### Changed — lib.rs 命令迁移重构
- 53 个 Tauri 命令从 ~1976 行的 `lib.rs` 全部分散到 30+ 个 `agent/*.rs` 模块，`lib.rs` 仅剩模块声明 + `run()` 入口（~470 行）
- 命令名与前端 `invoke()` 调用点不变，行为零改动
- 清理迁移过程中产生的全部死 `use` 导入，编译 0 warning

### Added — v0.3.0 待办推进
- 进化引擎持久化：`EvolutionEngine`/`LocalConsolidator`/`Consolidator` 新增 `save_state`/`load_state`，记忆池、固化技能、嵌入、调节器统计重启保留
- 动态指标采集：`analyze_task_enhanced` 从硬编码占位改为动态估算 token/复杂度/成本/时长/风险 + 模型推荐（对齐 `billing.rs` 官方定价）
- MCP 真实脚本集成：3 个真实 Node.js MCP 服务器（audit-vault / win32-registry / local-vector-glue）+ Rust 侧配置加载 + 启动自动注册
- ONNX 隐私遮罩集成：`is_valid_onnx` 真实 ModelProto 头校验（替代尺寸启发式）+ `detect_sensitive_regions` 模型驱动检测脚手架（真推理待真实模型 + tract/ort）
- 前端测试：Vitest + React Testing Library 基建 + 29 个测试（`utils`/`models`/`i18n`/`Modal`/`ErrorBoundary`/`ToastProvider`/`DiffViewer`）+ CI `npm test` 步骤

### Security — 安全审计加固 (2026-08-15)
- 路径穿越 ×2：单动作 `file_read/edit`（词法 `starts_with` 被 `../` 绕过）+ 代码块自动保存（Markdown filename hint 无校验）
- 终端命令黑名单 → 白名单：拒绝 shell 元字符 + 程序名白名单，`cmd`/`powershell`/`bash` 等解释器不入名单
- `web_fetch` SSRF：拦截内网/环回/链路本地/云元数据字面地址
- `cvfs_read_file` 任意文件读：复用写保护 Scope 过滤器（canonicalize + 拒绝 `..`）
- API Key 明文落盘：移除 base64 `.chronos_keys`，仅驻留内存 + Windows 凭据管理器
- 会话导入/导出加解密一致性：导入改加密、导出改解密
- GCM nonce 复用：时间戳+固定字节 → `Aes256Gcm::generate_nonce(OsRng)`
- 审批门禁接入 LLM 动作（`web_search`/`web_fetch`/`mcp_call`）+ `parse_action_type` 死规则修复
- 安全边界误伤：仅扫描 `Terminal` 命令，不再拦截含 SQL/shell 关键词的正常代码生成

### Added — 功能补齐 (2026-08-15)
- MCP HTTP+SSE 传输：`endpoint` 事件握手 + POST 通道 + 按 `id` 分发响应（此前 `"SSE transport not yet implemented"`）
- `ExecuteSkill` 动作：从桩接到真实 `SkillEngine::execute`（此前永远返回 "requires local filesystem access"）
- 视觉感知哈希：采样 64 字节 → 真实 DCT pHash；高斯模糊可分离化（O(25n) → O(10n)）
- 特征哈希嵌入：替代 `DefaultHasher` 伪随机向量（`mock_embed`，余弦无意义）

### Known Limitations
- ONNX 隐私遮罩：真实像素级高斯打码（可分离优化）+ `is_valid_onnx` 格式校验已实现；真 ONNX 推理仍待真实模型（`privacy_mask.onnx` 现为占位）+ `ort`/`tract` 推理库
- macOS / Linux 支持未做（需跨平台 CI + 条件编译）
- Rust Tauri 命令集成测试：无 State 命令已直测（`tests/commands.rs`）+ 引擎单测；State 依赖命令待 `tauri::test` harness（`AppState` 私有 + 沙箱无法运行 `cargo test`）
- MCP SSE 端到端：实现已 `cargo check --all-targets` 通过，完整链路需连真实 SSE 服务器验证

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