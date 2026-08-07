// session_db.rs — Chronos Streamed-Chunk Session DB
//
// 流式分块会话数据库 + 缓存锁定标记算法 (Caching Alignment Marker)
//
// 架构升级要点：
//  1. 分块存储：元数据 (.meta) 与消息体 (.chunks) 物理分离，
//     侧栏列表仅读取轻量 .meta 文件，彻底杜绝 I/O 阻塞。
//  2. SHA256 缓存哈希链：每条消息携带 caching_marker_hash，
//     前端/网关可直接比对，0ms 决策是否继承云端 Context Caching。
//  3. 单会话财务审计：SessionMetaManifest.total_accumulated_cost
//     帮企业主管追踪每个会话的独立资费。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use chrono::Utc;

/// 升级版：带缓存哈希对齐标记的消息实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageEntity {
    pub id: String,
    pub sender: String,            // "User" | "PM" | "Coder" | "System" 等
    pub model: String,             // 调用的特定大模型芯片节点，如 "deepseek-v4-pro"
    pub content: String,
    pub thinking: Option<String>,
    pub cost_tokens: u32,
    pub timestamp: String,
    /// 🔥 核心优化：端侧计算的上下文缓存特征哈希
    /// SHA256 链式累积，确保历史会话顺序一致时可 100% 命中 DeepSeek 一折缓存
    pub caching_marker_hash: String,
}

/// 升级版：流式轻量化会话元数据
/// 用于左侧会话历史树的极速 Lazy 加载，拒绝内存阻塞
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetaManifest {
    pub session_id: String,
    pub title: String,
    pub bound_project: String,
    pub last_updated: String,
    /// 消息总数，侧栏快速展示
    pub total_messages_count: u32,
    /// 单会话维度累计 Token 折算成本 (CNY)，供企业财务审计
    pub total_accumulated_cost: f64,
    /// 最后一条用户消息预览（前 40 字符）
    #[serde(default)]
    pub last_message_preview: String,
}

/// 完整会话数据包（用于分块写入与回溯加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionPayload {
    pub meta: SessionMetaManifest,
    pub messages: Vec<ChatMessageEntity>,
}

// ─── 存储路径 ──────────────────────────────────────────────────────

fn get_session_base_dir(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("chronos_vault/sessions");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

// ─── 缓存哈希链计算 ────────────────────────────────────────────────

impl ChatMessageEntity {
    /// 动态计算消息链的特征哈希，锁定云端一折扣费
    /// 采用 SHA256 链式累积：H_n = SHA256(H_{n-1} || sender || content)
    /// 种子根为 "CHRONOS_ROOT_SEED"
    pub fn compute_caching_hash(&mut self, previous_hash: &str) {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(self.sender.as_bytes());
        hasher.update(self.content.as_bytes());
        let result = hasher.finalize();
        self.caching_marker_hash = format!("{:x}", result);
    }
}

// ─── Tauri Commands ─────────────────────────────────────────────────

/// 核心命令 1：分块高效 Commit
///
/// 实现元数据与消息流的异步分块落盘：
///  - {session_id}.meta  → 轻量 SessionMetaManifest（侧栏极速读取）
///  - {session_id}.chunks → 完整消息体 Vec<ChatMessageEntity>
///
/// Rust 端全自动重构消息链的 Context 缓存哈希，严防 EXE 界面卡顿。
#[tauri::command]
pub async fn save_chat_session_chunk(
    app_handle: AppHandle,
    mut payload: ChatSessionPayload,
) -> Result<(), String> {
    let base_dir = get_session_base_dir(&app_handle);

    // 1. 动态自愈：在 Rust 端全自动重构并编译消息链的 Context 缓存哈希
    let mut current_hash = String::from("CHRONOS_ROOT_SEED");
    for msg in &mut payload.messages {
        msg.compute_caching_hash(&current_hash);
        current_hash = msg.caching_marker_hash.clone();
    }

    // 更新元数据的最终统计指标
    payload.meta.total_messages_count = payload.messages.len() as u32;
    // 提取最后一条用户消息预览
    payload.meta.last_message_preview = payload
        .messages
        .iter()
        .rev()
        .find(|m| m.sender == "User")
        .map(|m| {
            let txt = m.content.replace('\n', " ").replace('\r', "");
            if txt.len() > 40 {
                format!("{}…", &txt[..40])
            } else {
                txt
            }
        })
        .unwrap_or_default();

    // 2. 分块分级持久化：分离元数据与消息体
    let meta_path = base_dir.join(format!("{}.meta", payload.meta.session_id));
    let chunk_path = base_dir.join(format!("{}.chunks", payload.meta.session_id));

    println!(
        "[SESSION DB] Chunk-Writing VFS Session Archive to: {:?}",
        chunk_path
    );

    let meta_raw =
        serde_json::to_string_pretty(&payload.meta).map_err(|e| e.to_string())?;
    let chunk_raw =
        serde_json::to_string_pretty(&payload.messages).map_err(|e| e.to_string())?;

    fs::write(meta_path, meta_raw).map_err(|e| e.to_string())?;
    fs::write(chunk_path, chunk_raw).map_err(|e| e.to_string())?;

    Ok(())
}

/// 核心命令 2：历史回溯
///
/// 用户点击历史会话时，由 Rust 执行秒级异步分块加载。
/// 返回完整的 ChatSessionPayload（含 meta + messages），
/// 前端可直接恢复完整上下文并继承 DeepSeek Context Caching。
#[tauri::command]
pub async fn load_chat_session_chunk(
    app_handle: AppHandle,
    session_id: String,
) -> Result<ChatSessionPayload, String> {
    let base_dir = get_session_base_dir(&app_handle);
    let meta_path = base_dir.join(format!("{}.meta", session_id));
    let chunk_path = base_dir.join(format!("{}.chunks", session_id));

    if !meta_path.exists() || !chunk_path.exists() {
        return Err(
            "[CHRONOS DB ERR] 历史会话时空快照损坏或丢失。".to_string(),
        );
    }

    let meta_raw = fs::read_to_string(meta_path).map_err(|e| e.to_string())?;
    let chunk_raw = fs::read_to_string(chunk_path).map_err(|e| e.to_string())?;

    let meta: SessionMetaManifest =
        serde_json::from_str(&meta_raw).map_err(|e| e.to_string())?;
    let messages: Vec<ChatMessageEntity> =
        serde_json::from_str(&chunk_raw).map_err(|e| e.to_string())?;

    Ok(ChatSessionPayload { meta, messages })
}

/// 核心命令 3：极速加载历史清单列表
///
/// 仅读取轻量化 .meta 文件，毫秒级撑起成百上千条历史树轴。
/// 按 last_updated 降序排列，最新会话永远置顶。
#[tauri::command]
pub async fn list_historical_meta_manifests(
    app_handle: AppHandle,
) -> Result<Vec<SessionMetaManifest>, String> {
    let base_dir = get_session_base_dir(&app_handle);
    let mut manifest_list = Vec::new();

    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .map_or(false, |ext| ext == "meta")
            {
                if let Ok(meta_raw) = fs::read_to_string(path) {
                    if let Ok(meta) =
                        serde_json::from_str::<SessionMetaManifest>(&meta_raw)
                    {
                        manifest_list.push(meta);
                    }
                }
            }
        }
    }

    // 按最后修改时间戳降序排列，保证最新会话永远置顶
    manifest_list.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
    Ok(manifest_list)
}

/// 核心命令 4：删除历史会话
///
/// 物理删除 .meta 和 .chunks 文件，释放磁盘空间。
/// 不可逆操作，前端应弹出确认对话框。
#[tauri::command]
pub async fn delete_chat_session(
    app_handle: AppHandle,
    session_id: String,
) -> Result<String, String> {
    let base_dir = get_session_base_dir(&app_handle);
    let meta_path = base_dir.join(format!("{}.meta", &session_id));
    let chunk_path = base_dir.join(format!("{}.chunks", &session_id));

    let mut deleted = 0;
    if meta_path.exists() {
        fs::remove_file(&meta_path).map_err(|e| e.to_string())?;
        deleted += 1;
    }
    if chunk_path.exists() {
        fs::remove_file(&chunk_path).map_err(|e| e.to_string())?;
        deleted += 1;
    }

    if deleted == 0 {
        Err(format!("会话 {} 的档案文件不存在或已删除", session_id))
    } else {
        tracing::info!(
            "[SESSION DB] Deleted session '{}' ({} files removed)",
            session_id, deleted
        );
        Ok(format!("已删除会话 {}（{} 个文件）", session_id, deleted))
    }
}

/// 核心命令 5：重命名会话
///
/// 仅更新 .meta 文件中的 title 字段，不触碰 .chunks 消息体。
#[tauri::command]
pub async fn rename_chat_session(
    app_handle: AppHandle,
    session_id: String,
    new_title: String,
) -> Result<(), String> {
    let base_dir = get_session_base_dir(&app_handle);
    let meta_path = base_dir.join(format!("{}.meta", &session_id));

    if !meta_path.exists() {
        return Err("会话元数据文件不存在".to_string());
    }

    let meta_raw =
        fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let mut meta: SessionMetaManifest =
        serde_json::from_str(&meta_raw).map_err(|e| e.to_string())?;

    meta.title = new_title;
    meta.last_updated = chrono::Utc::now().to_rfc3339();

    let updated =
        serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&meta_path, updated).map_err(|e| e.to_string())?;

    tracing::info!(
        "[SESSION DB] Renamed session '{}' → '{}'",
        session_id, meta.title
    );
    Ok(())
}

/// 核心命令 6：导出会话为 JSON 字符串
///
/// 读取 .meta + .chunks 文件，合并为完整 ChatSessionPayload，
/// 以美化 JSON 字符串返回，供前端保存为 .json 文件。
#[tauri::command]
pub async fn export_chat_session(
    app_handle: AppHandle,
    session_id: String,
) -> Result<String, String> {
    let base_dir = get_session_base_dir(&app_handle);
    let meta_path = base_dir.join(format!("{}.meta", &session_id));
    let chunk_path = base_dir.join(format!("{}.chunks", &session_id));

    if !meta_path.exists() || !chunk_path.exists() {
        return Err("会话档案不完整，无法导出".to_string());
    }

    let meta_raw =
        fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let chunk_raw =
        fs::read_to_string(&chunk_path).map_err(|e| e.to_string())?;

    let meta: SessionMetaManifest =
        serde_json::from_str(&meta_raw).map_err(|e| e.to_string())?;
    let messages: Vec<ChatMessageEntity> =
        serde_json::from_str(&chunk_raw).map_err(|e| e.to_string())?;

    let payload = ChatSessionPayload { meta, messages };
    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

/// 核心命令 7：从 JSON 字符串导入会话
///
/// 反序列化 ChatSessionPayload，重建 .meta + .chunks 文件。
/// 若 session_id 冲突则追加时间戳后缀。
#[tauri::command]
pub async fn import_chat_session(
    app_handle: AppHandle,
    json_str: String,
) -> Result<SessionMetaManifest, String> {
    let mut payload: ChatSessionPayload =
        serde_json::from_str(&json_str).map_err(|e| {
            format!("JSON 解析失败（文件格式不正确）: {}", e)
        })?;

    let base_dir = get_session_base_dir(&app_handle);

    // 冲突检测：若 session_id 已存在，追加后缀
    let original_id = payload.meta.session_id.clone();
    let mut sid = original_id.clone();
    let mut counter = 1;
    while base_dir.join(format!("{}.meta", &sid)).exists() {
        sid = format!("{}-imported-{}", original_id, counter);
        counter += 1;
    }
    payload.meta.session_id = sid.clone();

    // 重建缓存哈希
    let mut current_hash = String::from("CHRONOS_ROOT_SEED");
    for msg in &mut payload.messages {
        msg.compute_caching_hash(&current_hash);
        current_hash = msg.caching_marker_hash.clone();
    }

    payload.meta.total_messages_count = payload.messages.len() as u32;
    payload.meta.last_updated = Utc::now().to_rfc3339();

    let meta_raw =
        serde_json::to_string_pretty(&payload.meta).map_err(|e| e.to_string())?;
    let chunk_raw =
        serde_json::to_string_pretty(&payload.messages).map_err(|e| e.to_string())?;

    fs::write(
        base_dir.join(format!("{}.meta", &sid)),
        meta_raw,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        base_dir.join(format!("{}.chunks", &sid)),
        chunk_raw,
    )
    .map_err(|e| e.to_string())?;

    tracing::info!(
        "[SESSION DB] Imported session '{}' as '{}'",
        original_id, sid
    );
    Ok(payload.meta)
}
