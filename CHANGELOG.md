# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.5.1] — 2026-09-07

### Added — 任务编排面板
- **新面板 `OrchestrationPanel`（dock 第 10 项「任务编排」）**：将此前无 UI 入口的编排引擎能力可视化——`analyze_task` 意图分析器（意图分类 + 置信度 + 推荐 Agent/模型 + 优化建议 + 次要意图）、`orch_parallel_groups` 并行组泳道、`orch_topological_sort` + `orch_executable_tasks` 执行顺序视图、`orch_schedule_quality` 质量分常驻卡片、`orch_smart_retry` 智能重试；Ctrl+K 命令面板同步新增 `nav-orchestrator` 入口；3 个冒烟测试

### Changed — 工程卫生与可维护性
- **千行模块拆分**：`distillation_engine.rs`（1656 行）拆为 `distillation_engine/{mod,types,engine,helpers,entities}.rs`；`web_intelligence.rs`（1650 行）拆为 `web_intelligence/{mod,types,core,markdown,commands}.rs`；外部 API 路径不变，行为零改动
- **oxlint 16 条警告清零**：修复 4 处未使用变量；移除 `if (true)` 死代码分支（Mock 演示模式不可达代码）；React hooks 依赖以稳定 ref 模式修复（含 `persistCurrentSession` 补 `currentProject` 依赖的真实 stale-closure 缺陷）；`ToastContext`/`useToast`/`buildPaletteCommands` 拆分独立文件满足 fast-refresh 单组件导出约束

### Added — 测试与 CI
- 前端首个 views 层测试：`SettingsPanel.test.tsx` 5 用例（Tab 渲染 / 设置加载 / key 状态回调 / 保存链路 / 凭据入库回调），全仓 34/34 通过
- CI 新增 `v*` tag 触发发布流水线；补打 `v0.3.0` / `v0.4.0` / `v0.5.0` 标签

### Changed — P0/P1 可用性治理（布局科学性）
- **微字号治理**：全局 264 处 ≤9px 字号（6/7/8/9px）统一提升至 10px 起步档——消除 83% 不可读微字号，10px 成为全应用最低可读档
- **SettingsPanel 配置安全**：costCap 输入校验（拒 NaN/负值，min=0.1，保存端 Math.max 兜底）；**未保存更改检测**（快照对比 + beforeunload 拦截 + 保存后刷新快照）
- **设置恢复默认**：一键恢复全部设置为出厂默认值（前端默认值常量化 + 确认弹窗 + 自动保存 + Toast 反馈）
- **安全侧栏可折叠**：280px 安全风控面板支持一键折叠/展开（ShieldHalf/ShieldCheck 图标，aria-label），**<1200px 窗口自动折叠**——缓解三层侧栏水平空间碎片化

### Added — P2 可用性增强
- **审批请求全局角标**：App 层 10s 轮询 `list_pending_approvals`，dock 审批按钮红点角标 + Footer 常驻「审批门禁 · N」提示——待审批不再需要主动进面板才能发现
- **拖拽上传**：Tauri 原生 `onDragDropEvent` 监听，文件拖入窗口自动进入多模态挂载缓冲区（按扩展名分流 doc/image 类型）

### Fixed — 缺陷修复（缺陷审查批次）
- **RemoteHub 连接参数错误**：点击"连接"已注册服务器时误用"添加表单"的当前值（空/默认端口 22/root），必然连接失败——改为注册时记录配置（serverConfigsRef 内存映射），连接时按 server_id 取回真实端口/用户/根路径
- **消息搜索 O(n²)**：搜索高亮在 map 内对每条消息反复 `findIndex`——改为预建 id→索引映射 + Set 查表（O(n)）
- 已排查非缺陷：会话切换时流式输出静默丢弃（按 streamMsgId 匹配，无数据损坏）；handleNewSession 无误持久化

### Changed — EMOJI 全面清零（全局图标规范落地）
- **文案/模板/数据名称中的 emoji 全部移除**（24 文件，计数 290 → 15）：toast 文案、系统消息模板、宏/技能/MCP 名称、欢迎语、i18n 字典——UI 图标全部使用 lucide/SvgIcons 矢量（前批完成）
- 保留 15 处**功能性数据**：用户头像默认值与 12 个可选头像、成就字段引用（a.emoji）——属用户可选数据而非图标占位
- 功能性符号（✕ 关闭/✓ 已配置/● 动画点/↵ 回车）不受影响

### Fixed — 主界面布局散架修复（聊天输入框不贴底）
- **根因**：MessageList 拆分时容器闭合错位——`</div>` 多放了一处在 MessageList 调用后，导致主栏 `flex-col` 容器提前闭合，Composer/成品面板/状态栏全部散落到容器外（聊天输入框不贴底、元素散离）
- 修复：移除多余闭合、恢复主栏层级（Header → 搜索 → 消息+文件行 → Composer → 成品面板 → 状态栏 → 快捷提示），层级扫描验证 depth 平衡；同时补回文件树 lucide 图标与契约徽章（git checkout 误回滚的部分）

### Fixed — 原生对话框系统性缺陷（交互逻辑审查发现）
- **Tauri v2 WebView2 默认禁用 window.alert/confirm/prompt**——8 个文件 46 处调用在桌面端静默失效：删除/回滚确认返回 undefined（危险操作被静默跳过）、错误提示不显示、ApprovalPanel 添加规则（4 连 prompt）完全不可用
- 修复：新建统一对话框工具 `src/lib/dialogs.ts`（Tauri 下走 plugin-dialog 的 confirm/message——capabilities 已含 dialog:default；浏览器模式降级原生对话框），**46 处调用全部迁移**，涉及函数按需 async 化
- ApprovalPanel「添加规则」由 4 连 prompt 重写为**内联表单**（操作类型下拉 + 风险/阈值/描述输入 + aria-label）

### Changed — 模型能力矩阵按官方模型库核验更新
- **数据源**（搜索引擎读取官方文档）：DeepSeek api-docs.deepseek.com/news/news260424 + Models & Pricing；Kimi platform.kimi.com/docs/api/models-overview；智谱 docs.bigmodel.cn / docs.z.ai（GLM-5.3-Flash 官方页）
- **DeepSeek V4 系修正**：官方 2026-04-24 预览、07-31 正式——`deepseek-v4-flash`/`v4-pro` 上下文 **1M 为官方服务默认**（注册表原 65536/131072 均严重低估）；284B-A13B / 1.6T-A49B MoE；legacy deepseek-chat/reasoner 已退役路由至 V4-Flash
- **Kimi K3 修正**：官方模型参数表确认 **1M 上下文**（2.8T 参数、原生视觉理解、缓存命中 $0.30）——注册表原 65536 严重低估；k2.7-code/highspeed 官方 **256K** + Context Caching
- **GLM-4.7 修正**：官方 **200K**（204800）+ 上下文缓存——注册表原 32768 低估
- **新增 glm-5.3-flash**：官方 2026-08 发布的原生多模态 VLM（文/图/视频/文件），**上下文 1M**、max output 128K、320B-A18B MoE（混合稀疏+线性注意力）、context caching——注册表此前无此模型
- glm-5.2/5.1/5v-turbo 官方模型库暂无对应页面，标注【待核验】保留项目声明值
- models.test.ts 断言同步（1M 窗口）

### Added — 泄压阈值可视化 + 用量分析面板（第三批）
- **新组件 `chat/ContextPressureMeter.tsx`**：点击状态栏「上下文 N%」展开——
  - **泄压阈值可视化**：三段分段计量条（0-80% 绿 / 80-95% 琥珀 / 95-100% 红）+ 当前压力填充 + 80% 触发线指针 + 大数字与区域标签
  - **泄压统计**：最近一次泄压的阶段（已剪枝/已截断/升级）与前后 token 对比、工具输出剪枝数、消息删除数、字符节省
  - **用量分析**：压力趋势迷你柱状图（最近 12 次发送，颜色按阈值分级）+ 消息 token 占用 Top 6 横向条形图（滑窗截断时的首要剪枝对象一目了然）
  - 红线状态（≥95%）显示「新建会话」逃生按钮
- 状态栏压力文字升级为可点击按钮（aria-expanded/aria-label），面板 absolute 悬浮于状态栏上方
- 前端 token 估算与 Rust estimate_tokens 同权重（ascii 0.25 / 其他 0.6）

### Added — 上下文管理设置分区 + 压力红线联动建议（第二批）
- **SettingsPanel 新增「上下文管理」Tab**（dock 第 8 项）：compact_ratio 滑条（30-95%，实时显示触发阈值 token 数）、默认上下文窗口输入、**按模型窗口覆盖列表**（MODELS 注册表预填 contextWindow，添加/删除）；配置经 saveSettings 持久化（serde default 向后兼容旧 config.json）
- `context_prune_apply` 改为从 AppSettings 读取配置并按 `selected_model` 应用窗口覆盖（前端不再传 config）
- **压力红线联动自动新建会话建议条**：PressureEscalation 时消息区顶部出现红色建议条（双语 + 「新建会话」一键操作 + 可关闭），正常泄压自动清除

### Added — 历史会话上下文管理机制（工具输出剪枝 + 滑窗截断 + 压力红线泄压）
- **新后端模块 `agent/context_pruning.rs`**：发送给 LLM 前的 chatMessages 自动泄压——
  - **工具输出剪枝（Tool Result Pruning）**：超长工具/命令输出（>8192 字符）压缩为 head 4096 + 省略标记（含被剪字符数）+ tail 1024，完整内容仍随会话分块归档于 Vault
  - **滑动窗口截断（History Truncation）**：超压时删除最旧前缀，原样保留最新 16% 预算；删除边界对齐 user/assistant 成对（assistant 永不孤立截断）；首条 System 开场永不删除
  - **压力红线阀**：`trigger = floor(context_window × compact_ratio)`（默认 64K×0.80），两级泄压最多 2 轮，仍超压上报 `PressureEscalation`（前端 Toast 建议新建会话）
- **发送链路接入**：`handleSend` 构造 chatMessages 后调用 `context_prune_apply`（失败降级原始消息），实际请求使用泄压后的 `outboundMessages`
- **压力可视化**：状态栏实时显示「上下文 N%」（<80% 灰 / ≥80% 琥珀 / ≥95% 红）
- 9 个单元测试（压力触发/成对保护/幂等/升级路径/配置钳制）；配置参数（compact_ratio 0.30-0.95、窗口、head/tail）支持前端按模型覆盖

### Changed — 长尾收尾：运行时提示消息双语化
- **RemoteHub / ProjectExplorer 的 alert/confirm/prompt 全部双语化**（23 处）：错误提示、危险操作确认、审批拦截提示均按界面语言输出——英文用户不再收到纯中文系统弹窗

### Refactor — ChatPanel 组件拆分（第四批，最终完成）
- 消息流渲染拆分为 `src/views/chat/MessageList.tsx`（216 行）：消息卡片（Cache-Aligned 徽章/附件胶囊/thinking 折叠/复制/重试）+ Markdown 渲染辅助（renderMdNode/MarkdownContent）整体迁移；滚动定位 refs（msgContainerRef/chatEndRef/isNearBottomRef）经 props 共享；**ChatPanel 2029 → 1398 行（累计 -31%）**，四子组件合计 834 行

### Refactor — ChatPanel 组件拆分（第三批，完成）
- 输入区拆分为 `src/views/chat/Composer.tsx`（210 行）：斜杠宏/@特种兵弹窗、多模态挂载看板、发送/停止按钮；附件挂载逻辑（dialogOpen 依赖）提取为父组件 `handleAttachDoc/handleAttachImage` 回调；`Attachment` 类型上移至 `@/lib/types` 共享；**ChatPanel 2029 → 1575 行（累计 -22%）**，行为零改动

### Refactor — ChatPanel 组件拆分（第二批）
- 会话历史侧栏拆分为 `src/views/chat/SessionSidebar.tsx`：内联重命名状态（editingSessionId/editTitle）、会话搜索（sessionFilter/filteredManifests）、重命名 handlers 全部内聚到子组件；JSON 导入因 dialogOpen 模块级依赖经 `onImport` 回调保留在父组件；ChatPanel 1981 → 1742 行

### Refactor — ChatPanel 组件拆分（第一批）
- 成品文件面板 + 文件编辑模态拆分为 `src/views/chat/ArtifactPanel.tsx`（独立组件 + 自含 extColor 辅助函数 + 显式 props 契约），ChatPanel 2029 → 1981 行；行为零改动

### Changed — 易用性与可访问性
- **ApprovalPanel / ProjectExplorer 全面 i18n 化**：新增 62 个 `ap_*`/`pe_*` 字典键（zh/en），Tab/表单/按钮/空态/健康面板/Worktree 状态全部走字典；JSX 硬编码中文 135 → ~75（本三批累计 162 → ~75）
- **RemoteHub 全面 i18n 化**：37 个 `rh_*` 字典键（zh/en），面板全部 placeholder/按钮/状态/空态引导文案不再硬编码中文；修复回滚输入框 `querySelector` 对中文 placeholder 的耦合（改用 `data-rewind-input` 属性锚点）；本批后 JSX 硬编码中文 162 → ~135
- **ChatPanel 结构性 emoji → lucide SVG（第一批 17 处 + 第二批 9 处）**：侧栏统计条、清空/导出/固化按钮、搜索图标与 placeholder、思考折叠、宏指令、重试、编辑、成品面板标题；emoji 计数 367 → 350（文案模板/头像数据中的 emoji 属内容，保留）
- **流式对话可中止**：前端接入后端 `cancel_chat_stream`（此前从未暴露），`isThinking` 时发送按钮切换为红色"停止"按钮，后端流循环检查取消标志提前返回
- **DockButton 可访问性**：9 个导航按钮补齐 `aria-label` + `aria-current`（全应用首批 aria 属性）
- **dock tooltip 全部入 i18n 字典**：新增 `dock_*` 9 键 + `stream_stop`（zh/en 双语），消除导航层硬编码中文

## [0.5.0] — 2026-08-17

### Changed — 七核心引擎 v2 升级（科学化/系统化/创新化）
- **任务智能引擎**：PERT 三点工作量估算 + 多维风险识别 + 关键路径分析
- **多模型协作引擎**：UCB1 探索-利用模型选择（多臂老虎机）
- **预测分析引擎**：CUSUM 变化点检测（累计和控制图）
- **端侧进化总线**：系统化健康自评估（A-F 分级 + 退化引擎识别）
- **多级语义蒸馏引擎**：实体关系图提取（共现→关系推断）
- **Web 智能搜索**：相关性重排序（查询词命中密度 + 来源权威加权）
- **调度引擎**：上下文感知路由（对话历史领域信号修正）

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