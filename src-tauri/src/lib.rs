// Chronos-Shadow 核心库入口
#![allow(deprecated)] // 旧 Router 将在 v0.2.0 彻底移除

pub mod agent;
pub mod vision;
mod state;

use state::AppState;

use tauri::Manager;
use tauri::WindowEvent;
use agent::session_db::{
    save_chat_session_chunk, load_chat_session_chunk,
    list_historical_meta_manifests, list_sessions_by_project,
    delete_chat_session, export_chat_session, rename_chat_session,
    import_chat_session,
};
use tracing_subscriber::fmt;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder};

// ─── 应用入口 ──────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // redline
            agent::redline::get_redline_status,
            agent::redline::validate_model_output,
            agent::redline::reset_fuse,
            // orchestrator
            agent::orchestrator::get_pipeline_stats,
            agent::orchestrator::orch_topological_sort,
            agent::orchestrator::orch_parallel_groups,
            agent::orchestrator::orch_executable_tasks,
            agent::orchestrator::orch_schedule_quality,
            agent::orchestrator::orch_smart_retry,
            agent::orchestrator::start_pipeline,
            agent::orchestrator::pause_pipeline,
            agent::orchestrator::resume_pipeline,
            agent::orchestrator::advance_pipeline,
            agent::orchestrator::create_task,
            agent::orchestrator::assign_task,
            agent::orchestrator::complete_task,
            agent::orchestrator::fail_task,
            // api client
            agent::api_client::chat_api,
            agent::api_client::chat_api_stream,
            agent::api_client::cancel_chat_stream,
            agent::api_client::get_cache_hit_stats,
            agent::api_client::check_dev_environment,
            agent::api_client::auto_install_deps,
            // sandbox
            agent::sandbox::init_sandbox,
            agent::sandbox::get_checkpoints,
            // mcp
            agent::mcp_client::mcp_connect_and_init,
            agent::mcp_client::mcp_fetch_tools,
            agent::mcp_client::mcp_disconnect,
            agent::mcp_client::mcp_cleanup_stale,
            agent::mcp_client::mcp_register_builtin_servers,
            // orchestrator management
            agent::orchestrator::prune_orchestrator_tasks,
            agent::orchestrator::flush_dead_letters,
            agent::orchestrator::get_event_metrics,
            // router
            agent::router::get_route_mode,
            agent::router::set_route_mode,
            agent::router::get_available_models,
            agent::router::set_model_api_key,
            agent::router::route_for_role,
            agent::router::get_model_endpoint,
            // hybrid router
            agent::router::hrouter_select_model,
            agent::router::hrouter_get_cluster_status,
            // local analytics
            agent::local_analytics::analytics_record,
            agent::local_analytics::analytics_snapshot,
            agent::local_analytics::analytics_window_metrics,
            agent::local_analytics::analytics_detect_anomalies,
            agent::local_analytics::analytics_correlation,
            agent::local_analytics::analytics_health_score,
            agent::local_analytics::analytics_change_point,
            // state manager
            agent::state_manager::state_save_all,
            agent::state_manager::state_health_report,
            // workbuddy engine
            agent::workbuddy_engine::wb_add_rule,
            agent::workbuddy_engine::wb_record_activity,
            agent::workbuddy_engine::wb_generate_report,
            agent::workbuddy_engine::wb_generate_suggestions,
            // system health
            agent::resilience::get_system_health,
            agent::resilience::get_circuit_breaker_status,
            // user profile
            agent::user_profile::get_user_profile,
            agent::user_profile::update_user_profile,
            agent::user_profile::get_greeting,
            agent::user_profile::get_personalization,
            agent::user_profile::get_heartbeat,
            agent::user_profile::get_achievements,
            agent::user_profile::touch_interaction,
            // security boundary
            agent::security_boundary::check_permission,
            agent::security_boundary::scan_llm_boundary,
            agent::security_boundary::get_security_report,
            // shadow
            agent::shadow::get_shadow_stats,
            agent::shadow::toggle_shadow,
            agent::shadow::dismiss_shadow_suggestion,
            agent::shadow::save_shadow_state,
            agent::shadow::load_shadow_state,
            // agent roster + live windows + evolution
            agent::router::get_agent_roster,
            agent::live_windows::list_live_windows,
            agent::indomitable_fetcher::indomitable_fetch_url,
            agent::indomitable_fetcher::extract_urls_from_text,
            agent::pptx_engine::pptx_generate,
            agent::pptx_engine::pptx_analyze_reference,
            agent::evolving::get_evolution_stats,
            agent::evolving::evo_validate_experience,
            agent::evolving::evo_intercept_context,
            // agent quality
            agent::evolving::agent_quality::get_agent_quality_scores,
            agent::evolving::agent_quality::record_agent_task_quality,
            agent::evolving::agent_quality::get_global_quality_report,
            // embedding engine
            agent::evolving::embedding::embedding_search,
            agent::evolving::embedding::embedding_add,
            agent::evolving::embedding::embedding_stats,
            // settings persistence
            agent::settings::load_settings,
            agent::settings::save_settings,
            // skill & mcp listing
            agent::skill_engine::list_skills,
            agent::mcp_client::list_mcp_servers,
            // remote cluster
            agent::remote_cluster::cluster_register_server,
            agent::remote_cluster::cluster_unregister_server,
            agent::remote_cluster::cluster_bind_project,
            agent::remote_cluster::cluster_edit_file,
            agent::remote_cluster::cluster_compile,
            agent::remote_cluster::cluster_ping,
            agent::remote_cluster::get_cluster_stats,
            // cvfs
            agent::sandbox::cvfs_create_project,
            agent::sandbox::cvfs_verify_scope,
            agent::sandbox::cvfs_read_file,
            agent::sandbox::cvfs_capture_checkpoint,
            agent::sandbox::cvfs_get_checkpoints,
            agent::sandbox::cvfs_get_projects,
            agent::sandbox::cvfs_delete_project,
            agent::sandbox::cvfs_list_project_files,
            agent::sandbox::cvfs_capture_checkpoint_v2,
            agent::sandbox::cvfs_restore_checkpoint,
            agent::sandbox::cvfs_delete_checkpoint,
            agent::sandbox::cvfs_get_project_health,
            // remote proxy
            agent::remote_proxy::remote_connect,
            agent::remote_proxy::remote_disconnect,
            agent::remote_proxy::remote_list_files,
            agent::remote_proxy::remote_read_file,
            agent::remote_proxy::remote_write_file,
            agent::remote_proxy::remote_compile,
            agent::remote_proxy::remote_snapshot,
            agent::remote_proxy::remote_rewind,
            agent::remote_proxy::get_remote_stats,
            // workbuddy
            agent::buddy_scan::get_buddy_scan_stats,
            agent::buddy_scan::run_buddy_scan,
            agent::buddy_scan::toggle_buddy_scan,
            agent::buddy_scan::get_buddy_saved_cost,
            agent::billing_engine::get_billing_stats,
            agent::billing_engine::get_billing_dashboard,
            agent::billing_engine::update_cost_cap,
            agent::billing_engine::get_model_recommendation,
            agent::billing_engine::check_context_health,
            agent::scheduling_engine::analyze_task,
            agent::hallucination_guard::audit_hallucination,
            agent::context_glue::get_context_glue_status,
            agent::context_glue::add_app_binding,
            agent::context_glue::remove_app_binding,
            agent::context_glue::get_app_bindings,
            agent::context_glue::toggle_context_glue,
            agent::context_glue::save_context_glue_bindings,
            agent::context_glue::load_context_glue_bindings,
            // general
            agent::sandbox::get_sandbox_status,
            agent::sandbox::sandbox_health_check,
            agent::sandbox::sandbox_audit_stats,
            agent::sandbox::sandbox_check_file_size,
            agent::sandbox::sandbox_cleanup_temp,
            agent::billing_engine::get_session_cost,
            agent::buddy_scan::get_saved_cost,
            agent::buddy_scan::get_saving_rate,
            // security vault
            agent::security_vault::get_vault_status,
            agent::security_vault::vault_api_key,
            agent::security_vault::fetch_api_key,
            agent::security_vault::delete_api_key,
            agent::detector::get_detector_stats,
            // vision privacy mask
            vision::vision_privacy_model_status,
            vision::vision_capture_frame,
            // lan health
            agent::router::check_lan_health,
            // worktree commands
            agent::worktree::create_worktree,
            agent::worktree::activate_worktree,
            agent::worktree::complete_worktree,
            agent::worktree::merge_worktree,
            agent::worktree::prune_worktree,
            agent::worktree::list_worktrees,
            agent::worktree::get_worktree_stats,
            // approval gate (第四红线)
            agent::approval_gate::submit_for_approval,
            agent::approval_gate::submit_for_approval_with_cost,
            agent::approval_gate::auditor_pre_screen_approval,
            agent::approval_gate::decide_approval,
            agent::approval_gate::list_pending_approvals,
            agent::approval_gate::get_approval_audit_log,
            agent::approval_gate::add_approval_rule,
            agent::approval_gate::remove_approval_rule,
            agent::approval_gate::get_approval_rules,
            agent::approval_gate::get_approval_suggestions,
            agent::approval_gate::expire_stale_approvals,
            agent::approval_gate::save_approval_state,
            agent::approval_gate::load_approval_state,
            // session persistence (chunked)
            save_chat_session_chunk,
            load_chat_session_chunk,
            list_historical_meta_manifests,
            list_sessions_by_project,
            delete_chat_session,
            export_chat_session,
            rename_chat_session,
            import_chat_session,
            // web intelligence
            agent::web_intelligence::web_intel_search,
            agent::web_intelligence::web_intel_fetch,
            agent::web_intelligence::web_intel_research,
            agent::web_intelligence::web_intel_add_domain,
            agent::web_intelligence::web_intel_remove_domain,
            agent::web_intelligence::web_intel_list_domains,
            agent::web_intelligence::web_intel_get_audit_log,
            agent::web_intelligence::web_intel_get_stats,
            agent::web_intelligence::web_intel_save_state,
            agent::web_intelligence::web_intel_load_state,
            // action dispatch engine
            agent::action_dispatch::execute_agent_action,
            agent::action_dispatch::extract_and_execute_actions,
            // collaboration engine
            agent::collaboration_engine::collab_get_model_ranking,
            agent::collaboration_engine::collab_recommend_model,
            agent::collaboration_engine::collab_record_execution,
            // task intelligence
            agent::task_intelligence::task_decompose,
            agent::task_intelligence::task_estimate_complexity,
            // predictive analytics
            agent::predictive_analytics::predictive_forecast_tokens,
            agent::predictive_analytics::predictive_detect_cost_anomaly,
            agent::predictive_analytics::predictive_optimize_budget,
            agent::predictive_analytics::predictive_analyze_enhanced,
            agent::scheduling_engine::scheduling_analyze_enhanced,
            // build status
            agent::build_status::get_build_status,
            // distillation evolution
            agent::web_intelligence::distill_evolution_report,
            agent::web_intelligence::distill_feedback,
            // evolution bus
            agent::evolution_bus::evobus_health_report,
            agent::evolution_bus::evobus_record_feedback,
            agent::evolution_bus::hallucination_feedback,
            // data flywheel
            agent::data_flywheel::flywheel_dashboard,
            agent::data_flywheel::flywheel_spin,
        ])
        .setup(|_app| {
            // 将 AppHandle 注入 Orchestrator，使其可以 emit 前端事件
            {
                let handle = _app.handle().clone();
                let state = _app.state::<AppState>();
                state.orchestrator.lock().unwrap().set_app_handle(handle);
            }

            // 自动恢复 Context Glue 跨应用绑定
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().context_glue.lock().unwrap().load_bindings(&dir);
                }
            }

            // 自动恢复 Shadow 影子记忆
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().shadow.lock().unwrap().load_state(&dir);
                }
            }

            // 自动恢复 Approval Gate 审批门禁状态
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().approval_gate.lock().unwrap().load_state(&dir);
                }
            }

            // 自动恢复进化引擎状态 (EvolutionBus + DataFlywheel + EvolutionEngine + Distillation)
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    if let Err(e) = _app.state::<AppState>().evolution_bus.lock().unwrap().load_state(&dir) {
                        tracing::warn!("[SETUP] evolution_bus load failed: {}", e);
                    }
                    if let Err(e) = _app.state::<AppState>().flywheel.lock().unwrap().load_state(&dir) {
                        tracing::warn!("[SETUP] flywheel load failed: {}", e);
                    }
                    let _ = std::fs::create_dir_all(&dir);
                    // EvolutionEngine 学习成果 (固化技能 + 记忆池) 恢复
                    {
                        let app_state = _app.state::<AppState>();
                        let mut evo = app_state.evolution.blocking_lock();
                        if let Err(e) = evo.load_state(&dir) {
                            tracing::warn!("[SETUP] evolution engine load failed: {}", e);
                        }
                    }
                    // Distillation + cache state restore
                    {
                        let app_state = _app.state::<AppState>();
                        let mut wi = app_state.web_intelligence.blocking_lock();
                        if let Err(e) = wi.distillation.load_state(&dir) {
                            tracing::warn!("[SETUP] distillation load failed: {}", e);
                        }
                        if let Err(e) = wi.cache.load_from_disk(&dir) {
                            tracing::warn!("[SETUP] cache load failed: {}", e);
                        }
                    }
                    tracing::info!("[SETUP] Evolution state restored");
                }
            }

            // 注册内置 MCP 服务器（真实 stdio 脚本）
            {
                let handle = _app.handle().clone();
                let state = _app.state::<AppState>();
                let mut mcp = state.mcp_client.blocking_lock();
                let _ = agent::mcp_client::register_builtin_servers(&mut mcp, &handle);
            }

            // 自动恢复 C-VFS 项目池
            {
                if let Ok(app_data) = _app.handle().path().app_data_dir() {
                    let state = _app.state::<AppState>();
                    // 使用 Tauri 的 async_runtime::block_on 确保正确阻塞
                    let result: Result<(), String> = tauri::async_runtime::block_on(async {
                        let cvfs = state.cvfs.lock().await;
                        cvfs.load_state(&app_data).await
                    });
                    match result {
                        Ok(()) => {
                            let count = tauri::async_runtime::block_on(async {
                                let cvfs = state.cvfs.lock().await;
                                cvfs.get_projects().await.len()
                            });
                            tracing::info!("[SETUP] C-VFS state loaded with {} projects", count);
                        }
                        Err(e) => tracing::warn!("[SETUP] C-VFS load_state failed: {}", e),
                    }
                }
            }

            // 自动加载 skills/ 目录下的所有技能清单
            {
                let state = _app.state::<AppState>();
                let mut skill_engine = state.skill_engine.lock().unwrap();
                let skills_dir = std::path::PathBuf::from(
                    env!("CARGO_MANIFEST_DIR")
                ).join("skills");
                if skills_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let manifest = path.join("skill.json");
                                if manifest.exists() {
                                    if let Ok(json) = std::fs::read_to_string(&manifest) {
                                        if let Err(e) = skill_engine.load_from_json(&json) {
                                            tracing::warn!("[SETUP] Failed to load skill {:?}: {}", path, e);
                                        } else {
                                            tracing::info!("[SETUP] Loaded skill: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // 激活所有已加载技能，使其可被 ExecuteSkill 动作执行 + 暴露给 Prompt
                skill_engine.activate_all();
            }

            // 注册统一持久化管理器 — 使用 Send+Sync 的 AppHandle
            {
                let handle = _app.handle().clone();
                let app_data = handle.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let state = _app.state::<AppState>();
                let mut sm = state.state_mgr.lock().unwrap();

                let h1 = handle.clone();
                let h2 = handle.clone();
                sm.register("shadow",
                    move |dir| { h1.state::<AppState>().shadow.lock().unwrap().save_state(dir).map_err(|e| e.to_string()) },
                    move |dir| { h2.state::<AppState>().shadow.lock().unwrap().load_state(dir).map_err(|e| e.to_string()) },
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("context_glue",
                    move |dir| h1.state::<AppState>().context_glue.lock().unwrap().save_bindings(dir),
                    move |dir| h2.state::<AppState>().context_glue.lock().unwrap().load_bindings(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("approval_gate",
                    move |dir| h1.state::<AppState>().approval_gate.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().approval_gate.lock().unwrap().load_state(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("embedding",
                    move |dir| h1.state::<AppState>().embedding.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().embedding.lock().unwrap().load_state(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("analytics",
                    move |dir| h1.state::<AppState>().analytics.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().analytics.lock().unwrap().load_state(dir),
                );
                let _ = sm.init(&app_data);
            }

            // 初始化 tracing 日志系统
            let app_dir = _app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let log_dir = app_dir.join("chronos_vault").join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let log_file = log_dir.join("chronos-shadow.log");
            let file_appender = tracing_appender::rolling::never(&log_dir, "chronos-shadow.log");
            fmt()
                .with_writer(file_appender)
                .with_target(false)
                .with_thread_ids(true)
                .init();
            tracing::info!("Chronos-Shadow v0.2.0 started — log at {:?}", log_file);

            // Seed billing_engine from saved settings (cost cap + legacy migration)
            {
                let state = _app.state::<AppState>();
                let settings = agent::settings::ensure_settings_loaded();
                state.billing_engine.set_cost_cap(settings.cost_cap);
                state.billing_engine.set_cost_cap_enabled(settings.cost_cap_enabled);
                if settings.accumulated_cost > 0.0 {
                    state.billing_engine.migrate_legacy_cost(settings.accumulated_cost);
                    tracing::info!("[BILLING] Migrated legacy cost ¥{:.2} to Budget tier", settings.accumulated_cost);
                }
            }

            // 系统托盘（可选 — 失败不阻断启动）
            let _ = (|| {
                let m = MenuBuilder::new(_app)
                    .item(&MenuItemBuilder::with_id("show", "显示窗口").build(_app).ok()?)
                    .separator()
                    .item(&MenuItemBuilder::with_id("quit", "退出 Chronos-Shadow").build(_app).ok()?)
                    .build().ok()?;
                let icon = _app.default_window_icon().cloned()?;
                let _tray = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("Chronos-Shadow | 时空之影")
                    .menu(&m)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => { if let Some(w) = app.get_webview_window("main") { w.show().ok(); w.set_focus().ok(); } }
                        "quit" => { app.exit(0); }
                        _ => {}
                    })
                    .build(_app).ok()?;
                tracing::info!("[SETUP] System tray icon created.");
                Some(())
            })();

            // 拦截窗口关闭事件 → 隐藏到托盘而非退出
            if let Some(window) = _app.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::CloseRequested { .. }) {
                        win.hide().ok();
                    }
                });
            }

            #[cfg(debug_assertions)]
            if let Some(window) = _app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Chronos-Shadow");
}
