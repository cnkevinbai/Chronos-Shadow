// 无 State 依赖的 Tauri 命令集成测试
//
// 直接调用 #[tauri::command] 函数（而非底层引擎方法），验证命令层返回结构。
// 依赖 State<AppState> 的命令需 tauri::test harness，暂不在此覆盖。

use chronos_shadow_lib::agent::build_status;
use chronos_shadow_lib::agent::scheduling_engine;
use chronos_shadow_lib::vision;

#[test]
fn test_vision_privacy_model_status_command() {
    let status = vision::vision_privacy_model_status();
    // 当前 privacy_mask.onnx 是占位文件（128 字节）
    assert!(!status.available, "占位模型不应标记为 available");
    assert!(status.is_placeholder, "占位模型应标记 is_placeholder");
    assert!(!status.message.is_empty(), "应返回说明信息");
    assert_eq!(status.path, "resources/privacy_mask.onnx");
}

#[test]
fn test_analyze_task_command() {
    let result = scheduling_engine::analyze_task("fix the crash bug in login".to_string());
    assert!(result.confidence > 0.0, "意图置信度应为正");
    assert!(!result.recommended_model.is_empty(), "应推荐模型");
}

#[test]
fn test_get_build_status_command() {
    let result = build_status::get_build_status();
    assert!(result.is_ok(), "get_build_status 应返回 Ok");
}
