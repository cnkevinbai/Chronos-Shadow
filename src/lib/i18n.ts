// Chronos-Shadow 国际化语言包
// 支持：简体中文 (zh) / English (en)

export type Lang = "zh" | "en";

export interface LocaleDict {
  // Header
  app_title: string;
  workbench: string;
  settings: string;
  auto_rule: string;
  manual_control: string;
  text_llm: string;
  vision_vlm: string;
  console: string;
  sandbox_workspace: string;

  // FooterBar
  session_cost: string;
  mode: string;
  shield_saved: string;
  efficiency: string;
  cost_cap: string;
  healing_fuse: string;

  // SDLC Pipeline
  orchestrator: string;
  full_auto: string;
  step_debug: string;
  stage: string;
  active: string;
  tasks: string;
  fused: string;
  pipeline: string;
  running: string;
  paused: string;

  // Project Explorer
  workspace: string;
  new_project: string;
  files: string;
  chrono_trigger: string;
  protected: string;
  global_mounts: string;
  snapshots: string;
  rewind_point: string;

  // Redline Guard
  redline_guard: string;
  privacy: string;
  cv_mask_regions: string;
  schema_check: string;
  sandbox_whitelist: string;
  healing_counter: string;
  execution_timeline: string;
  events: string;
  attempts_used: string;

  // Skill/MCP
  ecosystem_hub: string;
  skills: string;
  mcp: string;
  import_skill: string;
  prompt_synced: string;
  not_synced: string;
  connected: string;
  disconnected: string;
  view_schema: string;

  // Security Shield
  security_radar: string;
  sandbox_isolation: string;
  ast_audit: string;
  secrets: string;
  gpl_risk: string;
  audit_logs: string;

  // Chat Panel
  omni_chat: string;
  agent_listening: string;
  thinking_process: string;
  view_thinking: string;
  pipeline_dispatching: string;
  syncing_blackboard: string;
  input_placeholder: string;
  execute: string;

  // Settings
  settings_title: string;
  api_keys: string;
  cost_control: string;
  lan_gateway: string;
  security_redlines: string;
  apply_changes: string;
  syncing: string;
  deepseek_key: string;
  kimi_key: string;
  glm_key: string;
  cost_cap_enabled: string;
  cost_cap_enabled_sub: string;
  cost_cap_amount: string;
  caching_priority: string;
  caching_priority_sub: string;
  ollama_endpoint: string;
  lan_model_name: string;
  lan_timeout: string;
  auto_fallback: string;
  auto_fallback_sub: string;
  max_healing: string;
  max_healing_sub: string;
  ast_audit_enabled: string;
  ast_audit_sub: string;
  block_gpl: string;
  block_gpl_sub: string;
  privacy_blur: string;
  privacy_blur_sub: string;

  // Language
  language: string;
  lang_zh: string;
  lang_en: string;

  // Diff
  diff_title: string;
  before: string;
  after: string;
  lines_count: string;
  added: string;
  removed: string;

  // Evolution Console
  evo_title: string;
  evo_error_log: string;
  evo_pending: string;
  evo_consolidated: string;
  evo_skill_tree: string;
  evo_aura_connected: string;
  evo_inspector: string;
  evo_inspect_node: string;
  evo_hallucination_trace: string;
  evo_correction_ledger: string;
  evo_commit_memory: string;
  evo_consolidated_to_db: string;
  evo_export_skill: string;
  evo_click_to_inspect: string;
  evo_source: string;

  // Floating Bubble
  bubble_shadow_pilot: string;
  bubble_idle: string;
  bubble_agent_working: string;
  bubble_saved: string;
  bubble_restore: string;

  // Quick Macros
  macros_title: string;

  // Timeline Slider
  timeline_title: string;
  timeline_preview: string;

  // WorkBuddy
  workbuddy_title: string;
  workbuddy_subtitle: string;
  app_glue_binder: string;
  context_glue_status: string;
  buddy_scan: string;
  buddy_saved: string;
  app_matrix: string;
  active_bindings: string;
  bytes_transferred: string;
  tokens_saved: string;
  latency: string;
  add_binding: string;
  remove_binding: string;
  no_bindings: string;
  clipboard_managed: string;
  dpi_correction: string;
  hallucination_blocked: string;
  scan_active: string;

  // WorkBuddy 扩展
  win32_bound_windows: string;
  active_links: string;
  handle_hijack_label: string;
  app_conn_matrix: string;
  ipc_bindings_count: string;
  streams_active: string;
  streaming_status: string;
  paused_status: string;
  linked_status: string;
  unlinked_status: string;
  context_processing: string;
  glue_monitor: string;
  current_route: string;
  buddy_scan_benefit: string;
  vlm_screenshot_saved: string;
  pixel_correction_saved: string;
  glue_data_topology: string;
  route_status: string;
  route_running: string;
  route_shutdown: string;
  data_schema: string;
  hijack_depth: string;
  hijack_method: string;
  memory_speed: string;
  memory_speed_active: string;
  memory_speed_idle: string;
  workbuddy_audit_log: string;
  click_stream_hint: string;
  buddy_scan_standby: string;
  data_link_suspended: string;

  // ─── Hardcoded string fixes ──────────────────────────────────────
  evolution_tab: string;
  mini_mode: string;
  restore_mode: string;
  saved_by_evolution: string;
  cost_cap_prompt: string;
  new_project_prompt: string;
  snapshot_type_manual: string;
  sandbox_active_label: string;
  privacy_on: string;
  privacy_off: string;
  right_tab_security: string;
  right_tab_skills: string;
  right_tab_workbuddy: string;
  live_badge: string;
  privacy_region_chat: string;
  privacy_region_password: string;
  privacy_region_banking: string;
  privacy_region_custom: string;
  add_mask_region: string;
  self_healing_triggered: string;
  mcp_connected_label: string;
  skills_active_label: string;
  view_cards: string;
  view_canvas: string;
  view_monitor: string;
  buddy_scan_on: string;
  stream_on: string;
  stream_off: string;
  diff_select_hint: string;
  chat_welcome_connected: string;
  chat_welcome_demo: string;
  chat_error_api: string;
  chat_error_network: string;
  chat_mock_reply: string;
  demo_badge: string;
  sandbox_label: string;
  path_interceptions_label: string;
  fuse_status_label: string;
  tools_label: string;
  status_label: string;
  transport_label: string;
  resources_label: string;
  evo_state_evaluating: string;
  evo_state_validated: string;
  evo_state_idle: string;
  evo_contract_hotcompile: string;
  evo_free_token: string;
  evo_intercepts: string;
  evo_compiled: string;
  evo_memory_pool: string;
  evo_export_success: string;
  settings_api_credentials: string;
  settings_cost_risk: string;
  settings_lan_gateway: string;
  settings_security: string;
  settings_language: string;
  settings_sync_button: string;
  settings_deepseek_key: string;
  settings_kimi_key: string;
  settings_glm_key: string;
  settings_cost_cap_label: string;
  settings_cost_cap_desc: string;
  settings_caching_label: string;
  settings_caching_desc: string;
  settings_ollama_url: string;
  settings_lan_model: string;
  settings_lan_timeout: string;
  settings_fallback_label: string;
  settings_fallback_desc: string;
  settings_healing_label: string;
  settings_healing_desc: string;
  settings_ast_label: string;
  settings_ast_desc: string;
  settings_gpl_label: string;
  settings_gpl_desc: string;
  settings_privacy_label: string;
  settings_privacy_desc: string;

  offline_status: string;
  blocked_label: string;
  ok_status: string;
  new_button: string;
  lock_badge: string;
  mini_mode_button: string;
  cost_amount_label: string;
  toast_pipeline_start: string;
  toast_pipeline_pause: string;
  toast_pipeline_resume: string;
  toast_pipeline_advance: string;
  toast_router_auto: string;
  toast_router_manual: string;
  toast_llm_changed: string;

  // ─── Web Intelligence Panel ─────────────────────────────────────
  wi_title: string;
  wi_search: string;
  wi_fetch: string;
  wi_research: string;
  wi_domains: string;
  wi_audit: string;
  wi_search_placeholder: string;
  wi_search_btn: string;
  wi_engine_label: string;
  wi_fetch_url_placeholder: string;
  wi_fetch_distill: string;
  wi_fetch_btn: string;
  wi_research_topic_placeholder: string;
  wi_research_btn: string;
  wi_add_domain_placeholder: string;
  wi_no_results: string;
  wi_no_audit: string;
  wi_search_results: string;
  wi_source: string;
  wi_relevance: string;
  wi_distilled: string;
  wi_key_points: string;
  wi_domain_whitelist: string;
  wi_security_notice: string;
  wi_stats_searches: string;
  wi_stats_fetches: string;
  wi_stats_research: string;
  wi_stats_domains: string;
  wi_stats_blocked: string;
  wi_stats_distilled: string;
  wi_stats_compression: string;
  wi_stats_traffic_saved: string;
  wi_stats_cache_hit: string;
  wi_stats_unified_hits: string;
  wi_stats_api_saved: string;
  wi_stats_unified_rate: string;

  // ─── Auto Routing Panel ─────────────────────────────────────────
  ar_title: string;
  ar_rules: string;
  ar_models: string;
  ar_matrix: string;
  ar_search_placeholder: string;
  ar_all: string;
  ar_pro: string;
  ar_flash: string;
  ar_rules_count: string;
  ar_agents_count: string;
  ar_category: string;
  ar_description: string;
  ar_route_model: string;
  ar_match_keywords: string;
  ar_online: string;
  ar_offline: string;
  ar_quality: string;
  ar_latency: string;
  ar_cost: string;
  ar_loading_models: string;
  ar_agent: string;
  ar_model: string;
  ar_bottom_bar: string;

  // Command Palette
  cmd_cat_navigate: string;
  cmd_cat_session: string;
  cmd_cat_actions: string;
  cmd_cat_settings: string;
  cmd_search_placeholder: string;
  cmd_no_results: string;
  cmd_hint_nav: string;
  cmd_hint_exec: string;
  cmd_hint_close: string;
  cmd_count: string;
  cmd_chat: string;
  cmd_chat_desc: string;
  cmd_pipeline: string;
  cmd_pipeline_desc: string;
  cmd_glue: string;
  cmd_glue_desc: string;
  cmd_skills: string;
  cmd_skills_desc: string;
  cmd_webintel: string;
  cmd_webintel_desc: string;
  cmd_autoroute: string;
  cmd_autoroute_desc: string;
  cmd_remote: string;
  cmd_remote_desc: string;
  cmd_explorer: string;
  cmd_explorer_desc: string;
  cmd_approval: string;
  cmd_approval_desc: string;
  cmd_sess_new: string;
  cmd_sess_new_desc: string;
  cmd_sess_save: string;
  cmd_sess_save_desc: string;
  cmd_sess_export: string;
  cmd_sess_export_desc: string;
  cmd_sess_clear: string;
  cmd_sess_clear_desc: string;
  cmd_act_toggle: string;
  cmd_act_toggle_desc: string;
  cmd_act_focus: string;
  cmd_act_focus_desc: string;
  cmd_act_shortcuts: string;
  cmd_act_shortcuts_desc: string;
  cmd_set_mode: string;
  cmd_set_mode_desc: string;
  cmd_set_open: string;
  cmd_set_open_desc: string;
}

const zh: LocaleDict = {
  app_title: "CHRONOS-SHADOW",
  workbench: "研发工作台",
  settings: "全局配置",
  auto_rule: "自动全局规则",
  manual_control: "手动控制",
  text_llm: "文本LLM:",
  vision_vlm: "视觉VLM:",
  console: "Console",
  sandbox_workspace: "Sandbox Workspace:",

  session_cost: "Session Cost:",
  mode: "Mode:",
  shield_saved: "Chronos Shield Saved:",
  efficiency: "效率提升",
  cost_cap: "Cost Cap:",
  healing_fuse: "自愈熔断",

  orchestrator: "Orchestrator",
  full_auto: "全自动巡航",
  step_debug: "单步调试",
  stage: "阶段",
  active: "活跃",
  tasks: "任务",
  fused: "熔断",
  pipeline: "流水线",
  running: "运行中",
  paused: "已暂停",

  workspace: "Workspace",
  new_project: "新建项目",
  files: "文件",
  chrono_trigger: "时光机",
  protected: "已保护",
  global_mounts: "全局挂载 [RO]",
  snapshots: "个快照",
  rewind_point: "回滚到此节点",

  redline_guard: "红线防护",
  privacy: "隐私",
  cv_mask_regions: "CV 隐私遮罩区域",
  schema_check: "Schema 强校验",
  sandbox_whitelist: "文件沙盒白名单",
  healing_counter: "自愈熔断计数器",
  execution_timeline: "执行时间轴",
  events: "事件",
  attempts_used: "次尝试",

  ecosystem_hub: "生态中心",
  skills: "技能广场",
  mcp: "MCP 控制台",
  import_skill: "导入 Skill (JSON + 脚本)",
  prompt_synced: "Prompt 已同步",
  not_synced: "未同步",
  connected: "已连接",
  disconnected: "未连接",
  view_schema: "查看 JSON Schema",

  security_radar: "安全合规雷达",
  sandbox_isolation: "沙盒隔离",
  ast_audit: "AST 审计",
  secrets: "密钥扫描",
  gpl_risk: "GPL 风险",
  audit_logs: "审计日志",

  omni_chat: "全息对话控制台",
  agent_listening: "异步多智能体心流监听中...",
  thinking_process: "查阅智能体本地心流独白",
  view_thinking: "思考过程",
  pipeline_dispatching: "流水线正在调度多智能体推演...",
  syncing_blackboard: "智能体正在黑板中枢同步写入状态...",
  input_placeholder: "下达产品重构或 Quest 任务闭环指令...",
  execute: "执行",

  settings_title: "系统设置中枢",
  api_keys: "🔑 API 密钥凭据",
  cost_control: "📊 成本与资费风控",
  lan_gateway: "🌐 局域网离线网关",
  security_redlines: "🛡️ 安全红线与沙盒",
  apply_changes: "应用配置",
  syncing: "同步中...",
  deepseek_key: "DeepSeek API Key",
  kimi_key: "Kimi (Moonshot) API Key",
  glm_key: "智谱 GLM API Key",
  cost_cap_enabled: "激活开销上限硬熔断 (Cost Cap)",
  cost_cap_enabled_sub: "会话资费达阈值时自动挂起 Rust 线程并红屏报警",
  cost_cap_amount: "单次 Quest 最高开销 (RMB)",
  caching_priority: "压榨 DeepSeek Context Caching",
  caching_priority_sub: "Payload 头部强制注入静态契约，锁定一折缓存",
  ollama_endpoint: "Ollama / Llama.cpp 服务端点",
  lan_model_name: "离线热备模型名称",
  lan_timeout: "云端超时熔断阈值 (ms)",
  auto_fallback: "自动降级热切换",
  auto_fallback_sub: "云端不可用时自动切换至本地模型",
  max_healing: "最大容忍自愈循环阈值",
  max_healing_sub: "连续自愈失败超过此次数时强制熔断，严防死循环空耗",
  ast_audit_enabled: "端侧 AST 静态安全审查",
  ast_audit_sub: "写码后编译前静默审计密码泄露与 SQL 注入",
  block_gpl: "拦截 GPL 传染性协议",
  block_gpl_sub: "扫描依赖树，阻断可能导致企业源码被开源的协议",
  privacy_blur: "CV 隐私脱敏遮罩",
  privacy_blur_sub: "端侧轻量模型对敏感区域像素打码",

  language: "🌐 界面语言",
  lang_zh: "简体中文",
  lang_en: "English",

  diff_title: "差异对比",
  before: "变更前",
  after: "变更后",
  lines_count: "行",
  added: "新增",
  removed: "删除",

  evo_title: "📊 技能树矩阵进化图",
  evo_error_log: "📝 影子增量记忆错题本",
  evo_pending: "待固化",
  evo_consolidated: "已向量固化",
  evo_skill_tree: "技能树矩阵进化图",
  evo_aura_connected: "Evolution Aura: Connected",
  evo_inspector: "🛡️ 增量记忆解构审查",
  evo_inspect_node: "审查节点",
  evo_hallucination_trace: "⚠️ 幻觉/报错触发轨迹",
  evo_correction_ledger: "✅ 端侧自愈纠错对账单",
  evo_commit_memory: "⚡ 执行向量固化 (Commit Memory)",
  evo_consolidated_to_db: "✓ 已固化至本地 SQLite 飞轮",
  evo_export_skill: "导出为独立 Skill 工具包",
  evo_click_to_inspect: "点击左侧增量记忆进行解构审查",
  evo_source: "来源",

  bubble_shadow_pilot: "Shadow Pilot",
  bubble_idle: "☕ 后台随航静默中...",
  bubble_agent_working: "🤖 影子 Agent [{agent}] 正在推演",
  bubble_saved: "已省资费",
  bubble_restore: "唤醒大窗",

  macros_title: "💡 一键宏指令 · 智能推荐",

  timeline_title: "⏪ 时光拖拽回滚",
  timeline_preview: "预览",

  // WorkBuddy
  workbuddy_title: "🔗 跨软件随航总控台",
  workbuddy_subtitle: "WorkBuddy · 泛办公协同底座",
  app_glue_binder: "应用边界绑定器",
  context_glue_status: "Context Glue Status",
  buddy_scan: "视觉走查器",
  buddy_saved: "WorkBuddy 视觉纠偏省下",
  app_matrix: "应用连接矩阵",
  active_bindings: "活跃绑定",
  bytes_transferred: "已传输",
  tokens_saved: "Token 节省",
  latency: "延迟",
  add_binding: "+ 添加绑定",
  remove_binding: "移除",
  no_bindings: "暂无应用绑定 — 点击 + 创建",
  clipboard_managed: "剪贴板托管",
  dpi_correction: "DPI 纠偏",
  hallucination_blocked: "幻觉拦截",
  scan_active: "扫描引擎",

  // WorkBuddy 扩展
  win32_bound_windows: "🪟 Win32 活动窗口边界挂载",
  active_links: "Active Links",
  handle_hijack_label: "托管上下文与剪贴板",
  app_conn_matrix: "🔗 Application Connection Matrix (跨窗口粘合画布)",
  ipc_bindings_count: "IPC bindings",
  streams_active: "streams active",
  streaming_status: "Streaming (0ms)",
  paused_status: "Paused",
  linked_status: "LINKED",
  unlinked_status: "UNLINKED",
  context_processing: "● Context Processing",
  glue_monitor: "📊 Context Glue Monitor (粘合数据监视器)",
  current_route: "当前路由连线",
  buddy_scan_benefit: "Buddy Scan Benefit",
  vlm_screenshot_saved: "免除云端 VLM 全屏截图开销",
  pixel_correction_saved: "局部像素纠偏已省",
  glue_data_topology: "粘合数据拓扑流结构",
  route_status: "Route Status",
  route_running: "RUNNING (内存安全网桥)",
  route_shutdown: "SHUTDOWN",
  data_schema: "Data Schema",
  hijack_depth: "Hijack Depth",
  hijack_method: "Win32 WM_GETTEXT Hooking",
  memory_speed: "Memory Speed",
  memory_speed_active: "1.2 MB/s (Real-Time)",
  memory_speed_idle: "0 B/s",
  workbuddy_audit_log: "WorkBuddy 随航走查日志",
  click_stream_hint: "请点击画布中的连线流查看实时粘合日志",
  buddy_scan_standby: "Buddy-Scan: Standby",
  data_link_suspended: "数据连线已挂起",

  // ─── Hardcoded string fixes ──────────────────────────────────────
  evolution_tab: "🧬 进化",
  mini_mode: "⊟ 迷你",
  restore_mode: "⊞ 还原",
  saved_by_evolution: "🧬 Saved by Evolution",
  cost_cap_prompt: "请输入单次任务熔断上限金额 (¥):",
  new_project_prompt: "新项目名称:",
  snapshot_type_manual: "Manual",
  sandbox_active_label: "Sandbox: Active",
  privacy_on: "Privacy: ON",
  privacy_off: "Privacy: OFF",
  right_tab_security: "🛡️ 安全防线",
  right_tab_skills: "🧩 技能中枢",
  right_tab_workbuddy: "🔗 WorkBuddy",
  live_badge: "Live",
  privacy_region_chat: "聊天窗口",
  privacy_region_password: "密码输入框",
  privacy_region_banking: "网银/支付",
  privacy_region_custom: "自定义黑名单",
  add_mask_region: "+ 划定区域",
  self_healing_triggered: "Self-Healing 已触发",
  mcp_connected_label: "MCP connected",
  skills_active_label: "Skills active",
  view_cards: "窗口",
  view_canvas: "拓扑",
  view_monitor: "日志",
  buddy_scan_on: "Buddy-Scan ON",
  stream_on: "ON",
  stream_off: "OFF",
  diff_select_hint: "选择两个快照进行比较",
  chat_welcome_connected: "⚡ Chronos-Shadow 已连接云端 API。项目沙盒隔离锁 [ACTIVE]。输入自然语言指令启动全自动 SDLC 研发航道。",
  chat_welcome_demo: "⚡ Chronos-Shadow 守护引擎加载成功。⚠️ 未配置 API Key，当前为演示模式。请在「全局配置」中填入 API 密钥以启用真实对话。",
  chat_error_api: "❌ API 调用失败",
  chat_error_network: "❌ 网络错误",
  chat_mock_reply: "[演示模式] 收到:",
  demo_badge: "(Demo)",
  sandbox_label: "沙盒隔离",
  path_interceptions_label: "路径拦截",
  fuse_status_label: "熔断状态",
  tools_label: "Tools",
  status_label: "Status",
  transport_label: "Transport",
  resources_label: "Resources",
  evo_state_evaluating: "Evaluating",
  evo_state_validated: "Validated",
  evo_state_idle: "Idle",
  evo_contract_hotcompile: "📜 CLAUDE.md 契约热编译",
  evo_free_token: "[100%免Token]",
  evo_intercepts: "拦截",
  evo_compiled: "编译",
  evo_memory_pool: "记忆池",
  evo_export_success: "📦 已成功将此项本地进化工作流提取并编译封装为标准的独立 skill.json 扩展包。",
  settings_api_credentials: "🔑 API 密钥凭据",
  settings_cost_risk: "📊 成本与资费风控",
  settings_lan_gateway: "🌐 局域网离线网关",
  settings_security: "🛡️ 安全红线与沙盒",
  settings_language: "🌐 界面语言",
  settings_sync_button: "APPLY CHANGES",
  settings_deepseek_key: "DeepSeek API Key",
  settings_kimi_key: "Kimi (Moonshot) API Key",
  settings_glm_key: "智谱 GLM API Key",
  settings_cost_cap_label: "激活开销上限硬熔断 (Cost Cap)",
  settings_cost_cap_desc: "会话资费达阈值时自动挂起 Rust 线程并红屏报警",
  settings_caching_label: "压榨 DeepSeek Context Caching",
  settings_caching_desc: "Payload 头部强制注入静态契约，锁定一折缓存",
  settings_ollama_url: "Ollama / Llama.cpp 服务端点",
  settings_lan_model: "离线热备模型名称",
  settings_lan_timeout: "云端超时熔断阈值 (ms)",
  settings_fallback_label: "自动降级热切换",
  settings_fallback_desc: "云端不可用时自动切换至本地模型",
  settings_healing_label: "最大容忍自愈循环阈值",
  settings_healing_desc: "连续自愈失败超过此次数时强制熔断，严防死循环空耗",
  settings_ast_label: "端侧 AST 静态安全审查",
  settings_ast_desc: "写码后编译前静默审计密码泄露与 SQL 注入",
  settings_gpl_label: "拦截 GPL 传染性协议",
  settings_gpl_desc: "扫描依赖树，阻断可能导致企业源码被开源的协议",
  settings_privacy_label: "CV 隐私脱敏遮罩",
  settings_privacy_desc: "端侧轻量模型对敏感区域像素打码",

  offline_status: "Offline",
  blocked_label: "blocked",
  ok_status: "OK",
  new_button: "NEW",
  lock_badge: "🛡️ LOCK",
  mini_mode_button: "🎈 无感随航",
  cost_amount_label: "单次 Quest 最高开销 (RMB)",
  toast_pipeline_start: "全自动 SDLC 流水线已启动。",
  toast_pipeline_pause: "流水线已暂停。点击恢复继续执行。",
  toast_pipeline_resume: "流水线已恢复运行。",
  toast_pipeline_advance: "已推进至下一阶段。",
  toast_router_auto: "已切回自动全局能效规则，混合网关接管。",
  toast_router_manual: "手动覆盖模式激活。云端自适应评估已挂起。",
  toast_llm_changed: "文本链路已重定向至",

  // Web Intelligence
  wi_title: "🌐 Web 智能搜索",
  wi_search: "搜索",
  wi_fetch: "抓取",
  wi_research: "研究",
  wi_domains: "域名",
  wi_audit: "审计",
  wi_search_placeholder: "输入搜索关键词...",
  wi_search_btn: "搜索",
  wi_engine_label: "引擎",
  wi_fetch_url_placeholder: "输入 HTTPS URL...",
  wi_fetch_distill: "蒸馏",
  wi_fetch_btn: "抓取",
  wi_research_topic_placeholder: "输入研究主题...",
  wi_research_btn: "开始研究",
  wi_add_domain_placeholder: "添加域名...",
  wi_no_results: "暂无搜索结果",
  wi_no_audit: "暂无审计记录",
  wi_search_results: "个结果",
  wi_source: "来源",
  wi_relevance: "相关度",
  wi_distilled: "已蒸馏",
  wi_key_points: "关键点",
  wi_domain_whitelist: "域名白名单",
  wi_security_notice: "仅白名单内域名可被访问 · 首次访问需审批 · 全量审计",
  wi_stats_searches: "搜索",
  wi_stats_fetches: "抓取",
  wi_stats_research: "研究",
  wi_stats_domains: "白名单域名",
  wi_stats_blocked: "已拦截",
  wi_stats_distilled: "蒸馏次数",
  wi_stats_compression: "压缩率",
  wi_stats_traffic_saved: "节省流量",
  wi_stats_cache_hit: "蒸馏缓存",
  wi_stats_unified_hits: "统一缓存命中",
  wi_stats_api_saved: "API调用节省",
  wi_stats_unified_rate: "统一命中率",

  // Auto Routing
  ar_title: "自动路由中枢",
  ar_rules: "路由规则",
  ar_models: "模型矩阵",
  ar_matrix: "Agent映射",
  ar_search_placeholder: "搜索关键词/Agent...",
  ar_all: "全部",
  ar_pro: "Pro深度推理",
  ar_flash: "Flash快速",
  ar_rules_count: "条规则",
  ar_agents_count: "个Agent",
  ar_category: "分类",
  ar_description: "说明",
  ar_route_model: "路由模型",
  ar_match_keywords: "匹配关键词",
  ar_online: "在线",
  ar_offline: "离线",
  ar_quality: "质量",
  ar_latency: "延迟",
  ar_cost: "成本",
  ar_loading_models: "正在加载模型能力数据...",
  ar_agent: "Agent",
  ar_model: "模型",
  ar_bottom_bar: "自动按关键词匹配最优Agent+模型 · 27条路由规则",

  // Command Palette
  cmd_cat_navigate: "导航",
  cmd_cat_session: "会话",
  cmd_cat_actions: "操作",
  cmd_cat_settings: "设置",
  cmd_search_placeholder: "输入命令名称搜索…",
  cmd_no_results: "未找到匹配命令",
  cmd_hint_nav: "↑↓ 导航",
  cmd_hint_exec: "↵ 执行",
  cmd_hint_close: "ESC 关闭",
  cmd_count: "条命令",
  cmd_chat: "全局对话",
  cmd_chat_desc: "AI 对话面板",
  cmd_pipeline: "调度流水线",
  cmd_pipeline_desc: "7-Agent SDLC",
  cmd_glue: "跨软件粘合",
  cmd_glue_desc: "WorkBuddy 窗口绑定",
  cmd_skills: "技能中枢",
  cmd_skills_desc: "技能与MCP管理",
  cmd_webintel: "Web智能搜索",
  cmd_webintel_desc: "搜索/抓取/研究",
  cmd_autoroute: "自动路由",
  cmd_autoroute_desc: "关键词→Agent路由",
  cmd_remote: "远程服务器",
  cmd_remote_desc: "SSH编译管理",
  cmd_explorer: "项目沙盒",
  cmd_explorer_desc: "文件树/检查点",
  cmd_approval: "审批门禁",
  cmd_approval_desc: "第四红线安全审批",
  cmd_sess_new: "新建会话",
  cmd_sess_new_desc: "开启空白研发航道",
  cmd_sess_save: "保存会话",
  cmd_sess_save_desc: "固化当前对话到磁盘",
  cmd_sess_export: "导出会话JSON",
  cmd_sess_export_desc: "导出当前会话",
  cmd_sess_clear: "清空全部会话",
  cmd_sess_clear_desc: "删除所有历史档案",
  cmd_act_toggle: "切换侧栏",
  cmd_act_toggle_desc: "展开/收起历史会话",
  cmd_act_focus: "聚焦输入框",
  cmd_act_focus_desc: "光标移动到输入框",
  cmd_act_shortcuts: "快捷键帮助",
  cmd_act_shortcuts_desc: "查看全部快捷键",
  cmd_set_mode: "切换路由模式",
  cmd_set_mode_desc: "自动/手动路由",
  cmd_set_open: "全局配置",
  cmd_set_open_desc: "API密钥/成本风控",
};

const en: LocaleDict = {
  app_title: "CHRONOS-SHADOW",
  workbench: "Workbench",
  settings: "Settings",
  auto_rule: "Auto-Matrix",
  manual_control: "Manual",
  text_llm: "Text LLM:",
  vision_vlm: "Vision VLM:",
  console: "Console",
  sandbox_workspace: "Sandbox Workspace:",

  session_cost: "Session Cost:",
  mode: "Mode:",
  shield_saved: "Chronos Shield Saved:",
  efficiency: "efficiency gain",
  cost_cap: "Cost Cap:",
  healing_fuse: "Healing Fuse",

  orchestrator: "Orchestrator",
  full_auto: "Full-Auto Cruise",
  step_debug: "Step Debug",
  stage: "Stage",
  active: "Active",
  tasks: "Tasks",
  fused: "Fused",
  pipeline: "Pipeline",
  running: "Running",
  paused: "Paused",

  workspace: "Workspace",
  new_project: "New Project",
  files: "Files",
  chrono_trigger: "Chrono-Trigger",
  protected: "Protected",
  global_mounts: "Global Mounts [RO]",
  snapshots: "snapshots",
  rewind_point: "Rewind to This Point",

  redline_guard: "Redline Guard",
  privacy: "Privacy",
  cv_mask_regions: "CV Privacy Mask Regions",
  schema_check: "Schema Validation",
  sandbox_whitelist: "Sandbox Whitelist",
  healing_counter: "Healing Counter",
  execution_timeline: "Execution Timeline",
  events: "events",
  attempts_used: "attempts used",

  ecosystem_hub: "Ecosystem Hub",
  skills: "Skills",
  mcp: "MCP Console",
  import_skill: "Import Skill (JSON + Script)",
  prompt_synced: "Prompt Synced",
  not_synced: "Not Synced",
  connected: "Connected",
  disconnected: "Disconnected",
  view_schema: "View JSON Schema",

  security_radar: "Security Radar",
  sandbox_isolation: "Sandbox Isolation",
  ast_audit: "AST Audit",
  secrets: "Secrets",
  gpl_risk: "GPL Risk",
  audit_logs: "Audit Logs",

  omni_chat: "Omni-Chat Console",
  agent_listening: "Async multi-agent flow monitoring...",
  thinking_process: "View Agent Local Thinking Process",
  view_thinking: "Thinking Process",
  pipeline_dispatching: "Pipeline dispatching multi-agent inference...",
  syncing_blackboard: "Agents syncing state to blackboard hub...",
  input_placeholder: "Enter product refactor or Quest task command...",
  execute: "EXECUTE",

  settings_title: "System Settings Hub",
  api_keys: "🔑 API Credentials",
  cost_control: "📊 Cost & Risk Control",
  lan_gateway: "🌐 LAN Offline Gateway",
  security_redlines: "🛡️ Security Redlines & Sandbox",
  apply_changes: "APPLY CHANGES",
  syncing: "Syncing...",
  deepseek_key: "DeepSeek API Key",
  kimi_key: "Kimi (Moonshot) API Key",
  glm_key: "Zhipu GLM API Key",
  cost_cap_enabled: "Enable Cost Cap Circuit Breaker",
  cost_cap_enabled_sub: "Auto-suspend Rust threads and red-screen alert when session cost reaches threshold",
  cost_cap_amount: "Max Quest Cost (CNY)",
  caching_priority: "Force DeepSeek Context Caching",
  caching_priority_sub: "Inject static contract at payload head for 90% cache discount",
  ollama_endpoint: "Ollama / Llama.cpp API Endpoint",
  lan_model_name: "Fallback Model Name",
  lan_timeout: "Cloud Timeout Threshold (ms)",
  auto_fallback: "Auto Fallback Hot-Swap",
  auto_fallback_sub: "Seamlessly switch to local model when cloud is unavailable",
  max_healing: "Max Self-Healing Loop Threshold",
  max_healing_sub: "Force circuit break after consecutive failures exceed this count",
  ast_audit_enabled: "Local AST Static Security Audit",
  ast_audit_sub: "Silently audit for credential leaks & SQL injection before compilation",
  block_gpl: "Block GPL Copyleft Licenses",
  block_gpl_sub: "Scan dependency tree; prevent infectious open-source licenses",
  privacy_blur: "CV Privacy Blur Masking",
  privacy_blur_sub: "Lightweight on-device model pixelates sensitive regions",

  language: "🌐 Language",
  lang_zh: "简体中文",
  lang_en: "English",

  diff_title: "Visual Diff",
  before: "Before",
  after: "After",
  lines_count: "lines",
  added: "added",
  removed: "removed",

  evo_title: "📊 Skill Tree Matrix",
  evo_error_log: "📝 Shadow Delta Error Log",
  evo_pending: "pending",
  evo_consolidated: "Consolidated",
  evo_skill_tree: "Skill Tree Evolution Matrix",
  evo_aura_connected: "Evolution Aura: Connected",
  evo_inspector: "🛡️ Memory Inspector",
  evo_inspect_node: "Inspect Node",
  evo_hallucination_trace: "⚠️ Hallucination/Error Trigger Trace",
  evo_correction_ledger: "✅ On-Device Self-Healing Ledger",
  evo_commit_memory: "⚡ Commit Memory (Vectorize)",
  evo_consolidated_to_db: "✓ Consolidated to Local SQLite",
  evo_export_skill: "Export as Skill Package",
  evo_click_to_inspect: "Click a delta log to inspect",
  evo_source: "Source",

  bubble_shadow_pilot: "Shadow Pilot",
  bubble_idle: "☕ Idle — shadow monitoring...",
  bubble_agent_working: "🤖 Shadow Agent [{agent}] is inferring",
  bubble_saved: "Saved",
  bubble_restore: "Restore",

  macros_title: "💡 One-Click Macros · Smart Suggestions",

  timeline_title: "⏪ Timeline Rewind",
  timeline_preview: "Preview",

  // WorkBuddy
  workbuddy_title: "🔗 App Glue Binder",
  workbuddy_subtitle: "WorkBuddy · Office Co-Base",
  app_glue_binder: "App Connection Matrix",
  context_glue_status: "Context Glue Status",
  buddy_scan: "Buddy Scan",
  buddy_saved: "Saved by Buddy Scan",
  app_matrix: "App Connection Matrix",
  active_bindings: "Active Bindings",
  bytes_transferred: "Transferred",
  tokens_saved: "Tokens Saved",
  latency: "Latency",
  add_binding: "+ Add Binding",
  remove_binding: "Remove",
  no_bindings: "No bindings — click + to create",
  clipboard_managed: "Clipboard Managed",
  dpi_correction: "DPI Correction",
  hallucination_blocked: "Hallucination Blocked",
  scan_active: "Scan Engine",

  // WorkBuddy extensions
  win32_bound_windows: "🪟 Win32 Window Boundary Mounts",
  active_links: "Active Links",
  handle_hijack_label: "Hook Context & Clipboard",
  app_conn_matrix: "🔗 Application Connection Matrix",
  ipc_bindings_count: "IPC bindings",
  streams_active: "streams active",
  streaming_status: "Streaming (0ms)",
  paused_status: "Paused",
  linked_status: "LINKED",
  unlinked_status: "UNLINKED",
  context_processing: "● Context Processing",
  glue_monitor: "📊 Context Glue Monitor",
  current_route: "Current Route",
  buddy_scan_benefit: "Buddy Scan Benefit",
  vlm_screenshot_saved: "Eliminated VLM screenshot overhead",
  pixel_correction_saved: "Saved by pixel correction",
  glue_data_topology: "Glue Data Topology",
  route_status: "Route Status",
  route_running: "RUNNING (Memory-safe bridge)",
  route_shutdown: "SHUTDOWN",
  data_schema: "Data Schema",
  hijack_depth: "Hijack Depth",
  hijack_method: "Win32 WM_GETTEXT Hooking",
  memory_speed: "Memory Speed",
  memory_speed_active: "1.2 MB/s (Real-Time)",
  memory_speed_idle: "0 B/s",
  workbuddy_audit_log: "WorkBuddy Audit Trail",
  click_stream_hint: "Click a stream line in the canvas to view real-time glue logs",
  buddy_scan_standby: "Buddy-Scan: Standby",
  data_link_suspended: "Data link suspended",

  // ─── Hardcoded string fixes ──────────────────────────────────────
  evolution_tab: "🧬 Evolution",
  mini_mode: "⊟ Mini",
  restore_mode: "⊞ Restore",
  saved_by_evolution: "🧬 Saved by Evolution",
  cost_cap_prompt: "Enter cost cap limit (¥):",
  new_project_prompt: "New project name:",
  snapshot_type_manual: "Manual",
  sandbox_active_label: "Sandbox: Active",
  privacy_on: "Privacy: ON",
  privacy_off: "Privacy: OFF",
  right_tab_security: "🛡️ Security",
  right_tab_skills: "🧩 Skills Hub",
  right_tab_workbuddy: "🔗 WorkBuddy",
  live_badge: "Live",
  privacy_region_chat: "Chat Window",
  privacy_region_password: "Password Field",
  privacy_region_banking: "Banking UI",
  privacy_region_custom: "Custom Blocklist",
  add_mask_region: "+ Add Region",
  self_healing_triggered: "Self-Healing triggered",
  mcp_connected_label: "MCP connected",
  skills_active_label: "Skills active",
  view_cards: "Windows",
  view_canvas: "Canvas",
  view_monitor: "Logs",
  buddy_scan_on: "Buddy-Scan ON",
  stream_on: "ON",
  stream_off: "OFF",
  diff_select_hint: "Select two snapshots to compare",
  chat_welcome_connected: "⚡ Chronos-Shadow connected to cloud API. Sandbox isolation [ACTIVE]. Enter natural language to start full-auto SDLC pipeline.",
  chat_welcome_demo: "⚡ Chronos-Shadow engine loaded. ⚠️ No API Key configured — demo mode. Go to Settings to enter API credentials for live AI chat.",
  chat_error_api: "❌ API call failed",
  chat_error_network: "❌ Network error",
  chat_mock_reply: "[Demo mode] Received:",
  demo_badge: "(Demo)",
  sandbox_label: "Sandbox Isolation",
  path_interceptions_label: "Path Interceptions",
  fuse_status_label: "Fuse Status",
  tools_label: "Tools",
  status_label: "Status",
  transport_label: "Transport",
  resources_label: "Resources",
  evo_state_evaluating: "Evaluating",
  evo_state_validated: "Validated",
  evo_state_idle: "Idle",
  evo_contract_hotcompile: "📜 CLAUDE.md Contract Hot-Compile",
  evo_free_token: "[100% Token-Free]",
  evo_intercepts: "Intercepts",
  evo_compiled: "Compiled",
  evo_memory_pool: "Memory Pool",
  evo_export_success: "📦 Successfully extracted and compiled this evolution workflow into a standalone skill.json package.",
  settings_api_credentials: "🔑 API Credentials",
  settings_cost_risk: "📊 Cost & Risk Control",
  settings_lan_gateway: "🌐 LAN Gateway",
  settings_security: "🛡️ Security & Sandbox",
  settings_language: "🌐 Language",
  settings_sync_button: "APPLY CHANGES",
  settings_deepseek_key: "DeepSeek API Key",
  settings_kimi_key: "Kimi (Moonshot) API Key",
  settings_glm_key: "Zhipu GLM API Key",
  settings_cost_cap_label: "Activate Cost Cap Breaker",
  settings_cost_cap_desc: "Auto-suspend Rust thread and red-screen alert when session cost hits threshold",
  settings_caching_label: "Maximize DeepSeek Context Caching",
  settings_caching_desc: "Force-inject static contract into payload header for 1-fold cache discount",
  settings_ollama_url: "Ollama / Llama.cpp Endpoint",
  settings_lan_model: "Offline Hot-Standby Model Name",
  settings_lan_timeout: "Cloud Timeout Threshold (ms)",
  settings_fallback_label: "Auto LAN Fallback",
  settings_fallback_desc: "Seamlessly switch to local model when cloud is unreachable",
  settings_healing_label: "Max Self-Healing Loop Threshold",
  settings_healing_desc: "Force circuit-break after exceeding this count to prevent infinite loops",
  settings_ast_label: "On-Device AST Security Audit",
  settings_ast_desc: "Silently audit credential leaks & SQL injection before compilation",
  settings_gpl_label: "Block GPL Infectious Licenses",
  settings_gpl_desc: "Scan dependency tree; block licenses that may force open-source exposure",
  settings_privacy_label: "CV Privacy Blur Mask",
  settings_privacy_desc: "Lightweight on-device pixel masking for sensitive regions",

  offline_status: "Offline",
  blocked_label: "blocked",
  ok_status: "OK",
  new_button: "NEW",
  lock_badge: "🛡️ LOCK",
  mini_mode_button: "🎈 Shadow Mode",
  cost_amount_label: "Max Quest Cost (RMB)",
  toast_pipeline_start: "Full-auto SDLC pipeline started.",
  toast_pipeline_pause: "Pipeline paused. Click resume to continue.",
  toast_pipeline_resume: "Pipeline resumed.",
  toast_pipeline_advance: "Advanced to next stage.",
  toast_router_auto: "Switched to auto global efficiency rules. Hybrid gateway接管.",
  toast_router_manual: "Manual override mode active. Cloud adaptive evaluation suspended.",
  toast_llm_changed: "Text link redirected to",

  // Web Intelligence
  wi_title: "🌐 Web Intelligence",
  wi_search: "Search",
  wi_fetch: "Fetch",
  wi_research: "Research",
  wi_domains: "Domains",
  wi_audit: "Audit",
  wi_search_placeholder: "Enter search keywords...",
  wi_search_btn: "Search",
  wi_engine_label: "Engine",
  wi_fetch_url_placeholder: "Enter HTTPS URL...",
  wi_fetch_distill: "Distill",
  wi_fetch_btn: "Fetch",
  wi_research_topic_placeholder: "Enter research topic...",
  wi_research_btn: "Research",
  wi_add_domain_placeholder: "Add domain...",
  wi_no_results: "No search results",
  wi_no_audit: "No audit records",
  wi_search_results: "results",
  wi_source: "Source",
  wi_relevance: "Relevance",
  wi_distilled: "Distilled",
  wi_key_points: "Key Points",
  wi_domain_whitelist: "Domain Whitelist",
  wi_security_notice: "Only whitelisted domains accessible · First access requires approval · Full audit",
  wi_stats_searches: "Searches",
  wi_stats_fetches: "Fetches",
  wi_stats_research: "Research",
  wi_stats_domains: "Domains",
  wi_stats_blocked: "Blocked",
  wi_stats_distilled: "Distilled",
  wi_stats_compression: "Compression",
  wi_stats_traffic_saved: "Traffic Saved",
  wi_stats_cache_hit: "Distill Cache",
  wi_stats_unified_hits: "Cache Hits",
  wi_stats_api_saved: "API Saved",
  wi_stats_unified_rate: "Hit Rate",

  // Auto Routing
  ar_title: "Auto-Routing Hub",
  ar_rules: "Route Rules",
  ar_models: "Model Matrix",
  ar_matrix: "Agent Map",
  ar_search_placeholder: "Search keyword/Agent...",
  ar_all: "All",
  ar_pro: "Pro Deep",
  ar_flash: "Flash Fast",
  ar_rules_count: "rules",
  ar_agents_count: "agents",
  ar_category: "Category",
  ar_description: "Description",
  ar_route_model: "Route Model",
  ar_match_keywords: "Match Keywords",
  ar_online: "Online",
  ar_offline: "Offline",
  ar_quality: "Quality",
  ar_latency: "Latency",
  ar_cost: "Cost",
  ar_loading_models: "Loading model data...",
  ar_agent: "Agent",
  ar_model: "Model",
  ar_bottom_bar: "Auto-match optimal Agent+Model by keyword · 27 routing rules",

  // Command Palette
  cmd_cat_navigate: "Navigate",
  cmd_cat_session: "Session",
  cmd_cat_actions: "Actions",
  cmd_cat_settings: "Settings",
  cmd_search_placeholder: "Search commands…",
  cmd_no_results: "No matching commands",
  cmd_hint_nav: "↑↓ Navigate",
  cmd_hint_exec: "↵ Execute",
  cmd_hint_close: "ESC Close",
  cmd_count: "commands",
  cmd_chat: "Global Chat",
  cmd_chat_desc: "AI chat panel",
  cmd_pipeline: "Pipeline",
  cmd_pipeline_desc: "7-Agent SDLC",
  cmd_glue: "App Glue",
  cmd_glue_desc: "WorkBuddy window binding",
  cmd_skills: "Skill Hub",
  cmd_skills_desc: "Skills & MCP management",
  cmd_webintel: "Web Intelligence",
  cmd_webintel_desc: "Search / fetch / research",
  cmd_autoroute: "Auto Routing",
  cmd_autoroute_desc: "Keyword → Agent routing",
  cmd_remote: "Remote Server",
  cmd_remote_desc: "SSH compile management",
  cmd_explorer: "Project Sandbox",
  cmd_explorer_desc: "File tree / checkpoints",
  cmd_approval: "Approval Gate",
  cmd_approval_desc: "Fourth redline approval",
  cmd_sess_new: "New Session",
  cmd_sess_new_desc: "Start a blank session",
  cmd_sess_save: "Save Session",
  cmd_sess_save_desc: "Persist current chat to disk",
  cmd_sess_export: "Export Session JSON",
  cmd_sess_export_desc: "Export current session",
  cmd_sess_clear: "Clear All",
  cmd_sess_clear_desc: "Delete all history",
  cmd_act_toggle: "Toggle Sidebar",
  cmd_act_toggle_desc: "Expand / collapse history",
  cmd_act_focus: "Focus Input",
  cmd_act_focus_desc: "Move cursor to input",
  cmd_act_shortcuts: "Shortcuts",
  cmd_act_shortcuts_desc: "View all shortcuts",
  cmd_set_mode: "Toggle Route Mode",
  cmd_set_mode_desc: "Auto / manual routing",
  cmd_set_open: "Settings",
  cmd_set_open_desc: "API keys / cost control",
};

export const locales: Record<Lang, LocaleDict> = { zh, en };

export function getLocale(lang: Lang): LocaleDict {
  return locales[lang] ?? en;
}
