// 标准 MCP 宿主协议通信总线 — 工业级 Model Context Protocol 客户端
//
// 核心功能：
// - JSON-RPC 2.0 协议栈，通过 Stdio 管道异步拉起外部 MCP 服务器
// - 动态握手 (initialize) + 工具列表抓取 (tools/list) + Function Calling Schema 清洗
// - 端侧结果蒸馏 → 只喂结论给大模型，避免 Token 浪费
// - 支持 SSE 长连接（声明式） + Prompt 注入

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, oneshot};
use futures_util::StreamExt;
use tauri::Manager;

// ─── JSON-RPC 2.0 协议类型 ─────────────────────────────────────────

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

// ─── MCP 类型定义 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio { command: String, args: Vec<String>, env: HashMap<String, String> },
    Sse { url: String, headers: HashMap<String, String> },
}

impl McpTransport {
    pub fn label(&self) -> &str {
        match self { McpTransport::Stdio { .. } => "STDIO", McpTransport::Sse { .. } => "SSE" }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub tools_count: u32,
    pub resources_count: u32,
    pub connected: bool,
    pub connection_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(default = "default_input_schema")]
    pub input_schema: serde_json::Value,
}

fn default_input_schema() -> serde_json::Value {
    json!({"type": "object", "properties": {}})
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledResult {
    pub original_size: usize,
    pub distilled_size: usize,
    pub summary: String,
    pub key_points: Vec<String>,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub distilled: Option<DistilledResult>,
    pub error: Option<String>,
}

// ─── MCP 进程句柄 ──────────────────────────────────────────────────

/// 活跃的 Stdio MCP 进程（线程安全）
struct McpProcess {
    child: Child,
    request_id: u64,
}

/// 活跃的 SSE 连接（HTTP + Server-Sent Events）
struct SseConnection {
    http: reqwest::Client,
    /// 发送 JSON-RPC 消息的 POST 端点 URI（来自 endpoint 事件）
    post_url: String,
    /// request_id → 响应发送器（后台 GET 流按 id 分发响应）
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>>,
    request_id: Arc<AtomicU64>,
    /// 后台读取任务句柄（disconnect 时 abort）
    task: tokio::task::JoinHandle<()>,
}

// ─── MCP 客户端 ────────────────────────────────────────────────────

pub struct McpClient {
    pub servers: HashMap<String, McpServer>,
    pub tools: HashMap<String, Vec<McpTool>>,
    pub resources: HashMap<String, Vec<McpResource>>,
    /// Stdio 进程句柄映射（server_id → process）
    processes: HashMap<String, Arc<Mutex<McpProcess>>>,
    /// SSE 连接映射（server_id → 连接）
    sse: HashMap<String, SseConnection>,
    pub distillation_threshold: usize,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            tools: HashMap::new(),
            resources: HashMap::new(),
            processes: HashMap::new(),
            sse: HashMap::new(),
            distillation_threshold: 1024,
        }
    }

    // ── 进程管理（对齐白皮书 spawn_server） ────────────────────────

    /// 通过 Stdio 管道动态拉起外部 MCP 服务器守护进程
    pub fn spawn_server(server_name: &str, command: &str, args: &[&str]) -> std::io::Result<(Self, String)> {
        let server_id = server_name.to_lowercase().replace(' ', "-");
        tracing::info!("[MCP] Launching external MCP server: [{}]", server_name);

        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let process = McpProcess { child, request_id: 1 };
        let mut client = Self::new();
        client.processes.insert(server_id.clone(), Arc::new(Mutex::new(process)));
        client.servers.insert(server_id.clone(), McpServer {
            id: server_id.clone(), name: server_name.into(),
            transport: McpTransport::Stdio {
                command: command.into(), args: args.iter().map(|s| s.to_string()).collect(),
                env: HashMap::new(),
            },
            tools_count: 0, resources_count: 0, connected: true, connection_failures: 0,
        });
        client.tools.insert(server_id.clone(), Vec::new());
        client.resources.insert(server_id.clone(), Vec::new());

        Ok((client, server_id))
    }

    /// JSON-RPC 2.0 请求 → 响应（线程安全）
    async fn send_request(&self, server_id: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        // SSE 传输走独立的 POST + 流式响应通道
        let is_sse = self.servers.get(server_id)
            .map(|s| matches!(&s.transport, McpTransport::Sse { .. }))
            .unwrap_or(false);
        if is_sse {
            return self.send_request_sse(server_id, method, params).await;
        }

        let proc_arc = self.processes.get(server_id)
            .ok_or_else(|| format!("Server '{}' has no active process", server_id))?;
        let mut proc = proc_arc.lock().await;

        let id = proc.request_id;
        proc.request_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(), id, method: method.into(), params,
        };
        let raw = serde_json::to_string(&request).map_err(|e| e.to_string())? + "\n";

        // Write to stdin
        let stdin = proc.child.stdin.as_mut().ok_or("stdin pipe unavailable")?;
        stdin.write_all(raw.as_bytes()).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;

        // Read from stdout
        let stdout = proc.child.stdout.as_mut().ok_or("stdout pipe unavailable")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;

        let response: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| format!("MCP parse error: {} (raw: {})", e, &line[..200.min(line.len())]))?;

        if let Some(err) = response.error {
            return Err(format!("MCP error: {:?}", err));
        }
        response.result.ok_or_else(|| "MCP empty result".into())
    }

    /// SSE 传输的请求发送：POST JSON-RPC 到 endpoint，等待后台 GET 流按 id 返回响应
    async fn send_request_sse(&self, server_id: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let conn = self.sse.get(server_id)
            .ok_or_else(|| format!("SSE server '{}' not connected", server_id))?;
        let id = conn.request_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest { jsonrpc: "2.0".into(), id, method: method.into(), params };
        let raw = serde_json::to_string(&request).map_err(|e| e.to_string())?;

        let (tx, rx) = oneshot::channel();
        conn.pending.lock().await.insert(id, tx);

        let resp = match conn.http.post(&conn.post_url)
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .body(raw)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                conn.pending.lock().await.remove(&id);
                return Err(format!("SSE POST failed: {}", e));
            }
        };

        if !resp.status().is_success() {
            conn.pending.lock().await.remove(&id);
            return Err(format!("SSE POST returned {}", resp.status()));
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("SSE response channel closed".into()),
            Err(_) => { conn.pending.lock().await.remove(&id); Err("SSE response timeout".into()) }
        }
    }

    // ── 协议握手（对齐白皮书 initialize_handshake） ───────────────

    /// 与外部 MCP 服务器执行标准 Protocol 握手
    pub async fn initialize_handshake(&self, server_id: &str) -> Result<(), String> {
        let server = self.servers.get(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?;
        tracing::info!("[MCP HANDSHAKE] Shaking hands with: [{}]", server.name);

        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "Chronos-Shadow-Client", "version": "1.0.0" }
        });

        let result = self.send_request(server_id, "initialize", params).await?;
        tracing::info!("[MCP HANDSHAKE] Established. Capabilities: {:?}", result.get("capabilities"));
        Ok(())
    }

    // ── 动态工具抓取（对齐白皮书 fetch_and_clean_tools） ──────────

    /// 动态抓取 Tools 列表，解构清洗为 Function Calling Schema
    pub async fn fetch_and_clean_tools(&mut self, server_id: &str) -> Result<Vec<McpTool>, String> {
        let server = self.servers.get(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?;
        tracing::info!("[MCP SCANNER] Grabbing tools from [{}]...", server.name);

        let result = self.send_request(server_id, "tools/list", json!({})).await?;
        let arr = result.get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| "MCP tools/list returned invalid format")?;

        let mut cleaned = Vec::new();
        for val in arr {
            if let Ok(tool) = serde_json::from_value::<McpTool>(val.clone()) {
                tracing::info!("  [TOOL] {} — {:?}", tool.name, tool.description);
                cleaned.push(tool);
            }
        }

        // Update registry
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.tools_count = cleaned.len() as u32;
        }
        self.tools.insert(server_id.into(), cleaned.clone());

        tracing::info!("[MCP SCANNER] Registered {} tools into active hub.", cleaned.len());
        Ok(cleaned)
    }

    /// 注册 MCP 服务器并自动连接+握手+拉取工具
    pub fn register_server(&mut self, server: McpServer) {
        let id = server.id.clone();
        self.tools.entry(id.clone()).or_default();
        self.resources.entry(id.clone()).or_default();
        self.servers.insert(id, server);
    }

    /// 建立真实连接：spawn 进程 + 握手 + 拉取工具
    pub async fn connect_and_init(&mut self, server_id: &str) -> Result<(), String> {
        let server = self.servers.get(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?.clone();

        match &server.transport {
            McpTransport::Stdio { command, args, .. } => {
                let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let child = Command::new(command)
                    .args(&str_args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| format!("Spawn failed: {}", e))?;

                let process = McpProcess { child, request_id: 1 };
                self.processes.insert(server_id.into(), Arc::new(Mutex::new(process)));
            }
            McpTransport::Sse { url, .. } => {
                self.connect_sse(server_id, url).await?;
            }
        }

        // Update server state
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.connected = true;
            srv.connection_failures = 0;
        }

        // Handshake + fetch tools
        self.initialize_handshake(server_id).await?;
        self.fetch_and_clean_tools(server_id).await?;

        Ok(())
    }

    /// 建立 SSE 连接：GET SSE 端点 → 解析 endpoint 事件 → 后台任务持续读取并分发 message 事件
    async fn connect_sse(&mut self, server_id: &str, url: &str) -> Result<(), String> {
        let http = reqwest::Client::new();
        let resp = http.get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| format!("SSE connect failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("SSE connect returned {}", resp.status()));
        }

        let mut stream = resp.bytes_stream();
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (endpoint_tx, endpoint_rx) = oneshot::channel::<String>();
        let pending_clone = pending.clone();

        let task = tokio::spawn(async move {
            let mut endpoint_tx = Some(endpoint_tx);
            let mut buf = String::new();
            let mut endpoint_sent = false;
            while let Some(chunk) = stream.next().await {
                let bytes = match chunk { Ok(b) => b, Err(_) => break };
                // 归一化 CRLF → LF，便于按 \n\n 切分事件
                buf.push_str(&String::from_utf8_lossy(&bytes).replace("\r\n", "\n"));
                while let Some(pos) = buf.find("\n\n") {
                    let block: String = buf.drain(..pos + 2).collect();
                    if let Some((event, data)) = parse_sse_event(&block) {
                        if event == "endpoint" && !endpoint_sent {
                            if let Some(tx) = endpoint_tx.take() {
                                let _ = tx.send(data.clone());
                            }
                            endpoint_sent = true;
                        } else if event == "message" || event.is_empty() {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                                if let Some(id) = v.get("id").and_then(|i| i.as_u64()) {
                                    if let Some(tx) = pending_clone.lock().await.remove(&id) {
                                        let _ = tx.send(Ok(v));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let endpoint = tokio::time::timeout(std::time::Duration::from_secs(10), endpoint_rx)
            .await
            .map_err(|_| "SSE endpoint event timeout".to_string())?
            .map_err(|_| "SSE endpoint channel closed".to_string())?;

        let post_url = resolve_url(url, &endpoint);

        self.sse.insert(server_id.to_string(), SseConnection {
            http,
            post_url,
            pending,
            request_id: Arc::new(AtomicU64::new(1)),
            task,
        });
        Ok(())
    }

    pub fn connect(&mut self, server_id: &str) -> Result<(), String> {
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.connected = true;
            srv.connection_failures = 0;
            Ok(())
        } else {
            Err(format!("Server '{}' not found", server_id))
        }
    }

    pub fn disconnect(&mut self, server_id: &str) -> Result<(), String> {
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.connected = false;
        }
        // Kill process if any
        if let Some(proc_arc) = self.processes.remove(server_id) {
            if let Ok(mut proc) = proc_arc.try_lock() {
                let _ = proc.child.kill();
            }
        }
        // Abort SSE 后台读取任务
        if let Some(conn) = self.sse.remove(server_id) {
            conn.task.abort();
        }
        Ok(())
    }

    pub fn register_tool(&mut self, server_id: &str, tool: McpTool) -> Result<(), String> {
        let tools = self.tools.get_mut(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?;
        tools.push(tool);
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.tools_count = tools.len() as u32;
        }
        Ok(())
    }

    pub fn register_resource(&mut self, server_id: &str, resource: McpResource) -> Result<(), String> {
        let resources = self.resources.get_mut(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?;
        resources.push(resource);
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.resources_count = resources.len() as u32;
        }
        Ok(())
    }

    // ── 工具调用（真实 JSON-RPC + 端侧蒸馏） ──────────────────────

    pub async fn call_tool(&self, server_id: &str, tool_name: &str, args: &serde_json::Value) -> McpCallResult {
        let server = match self.servers.get(server_id) {
            Some(s) if s.connected => s,
            Some(_) => return McpCallResult { success: false, data: None, distilled: None, error: Some(format!("'{}' not connected", server_id)) },
            None => return McpCallResult { success: false, data: None, distilled: None, error: Some(format!("'{}' not found", server_id)) },
        };

        // Try real JSON-RPC if process is active
        if self.processes.contains_key(server_id) {
            let params = json!({ "name": tool_name, "arguments": args });
            match self.send_request(server_id, "tools/call", params).await {
                Ok(result) => {
                    let distilled = self.distill(&result);
                    return McpCallResult { success: true, data: Some(result), distilled: Some(distilled), error: None };
                }
                Err(e) => {
                    tracing::warn!("[MCP] Real call failed, falling back: {}", e);
                }
            }
        }

        // Fallback: 返回错误而非虚构数据，防止 LLM 收到幻觉输出
        McpCallResult {
            success: false,
            data: None,
            distilled: None,
            error: Some(format!("MCP tool '{}' on server '{}' is unavailable", tool_name, server.name)),
        }
    }

    // ── 蒸馏 + Prompt 注入 ────────────────────────────────────────

    pub fn distill(&self, data: &serde_json::Value) -> DistilledResult {
        let raw = serde_json::to_string(data).unwrap_or_default();
        let original_size = raw.len();
        if original_size <= self.distillation_threshold {
            return DistilledResult { original_size, distilled_size: original_size, summary: raw, key_points: vec![], compression_ratio: 1.0 };
        }
        let mut key_points = Vec::new();
        if let Some(obj) = data.as_object() {
            for (k, v) in obj.iter().take(5) {
                key_points.push(format!("{}: {}", k, distill_value(v)));
            }
        }
        let summary = format!("[Distilled] {} bytes → {} key points", original_size, key_points.len());
        let dsize = summary.len();
        DistilledResult { original_size, distilled_size: dsize, summary, key_points, compression_ratio: dsize as f64 / original_size as f64 }
    }

    pub fn connected_servers(&self) -> Vec<&McpServer> {
        self.servers.values().filter(|s| s.connected).collect()
    }

    pub fn generate_prompt_fragment(&self) -> String {
        let connected = self.connected_servers();
        if connected.is_empty() { return String::new(); }
        let mut frag = String::from("## Available MCP Tools\n\n");
        for srv in &connected {
            if let Some(tools) = self.tools.get(&srv.id) {
                frag.push_str(&format!("### {} (via {})\n", srv.name, srv.transport.label()));
                for t in tools {
                    frag.push_str(&format!("- **{}**: {}\n", t.name, t.description));
                }
                frag.push('\n');
            }
        }
        frag
    }
}

impl Default for McpClient {
    fn default() -> Self { Self::new() }
}

// ─── 内置服务器配置加载（对齐 resources/mcp/*.json） ────────────

/// resources/mcp/*.json 的配置结构（与 McpServer 分离，便于反序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub transport: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl McpServerConfig {
    /// 配置 → McpServer；相对脚本路径解析为配置目录下的绝对路径
    pub fn to_server(&self, dir: &std::path::Path) -> McpServer {
        let args: Vec<String> = self.args.iter().map(|a| {
            let p = std::path::Path::new(a);
            if p.is_absolute() {
                a.clone()
            } else {
                dir.join(p.file_name().unwrap_or_default()).to_string_lossy().into_owned()
            }
        }).collect();
        McpServer {
            id: self.name.clone(),
            name: if self.display_name.is_empty() { self.name.clone() } else { self.display_name.clone() },
            transport: McpTransport::Stdio { command: self.command.clone(), args, env: HashMap::new() },
            tools_count: 0,
            resources_count: 0,
            connected: false,
            connection_failures: 0,
        }
    }
}

/// 从目录读取所有 *.json MCP 配置，转换为 McpServer 列表
pub fn load_server_configs(dir: &std::path::Path) -> Vec<McpServer> {
    let mut servers = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        tracing::warn!("[MCP] Cannot read config dir: {:?}", dir);
        return servers;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<McpServerConfig>(&json) {
                Ok(cfg) => servers.push(cfg.to_server(dir)),
                Err(e) => tracing::warn!("[MCP] Invalid config {}: {}", path.display(), e),
            },
            Err(e) => tracing::warn!("[MCP] Cannot read {}: {}", path.display(), e),
        }
    }
    servers
}

/// 解析 MCP 配置目录：运行时资源目录优先，回退到编译期 CARGO_MANIFEST_DIR
pub fn resolve_mcp_dir(app_handle: &tauri::AppHandle) -> std::path::PathBuf {
    if let Ok(res) = app_handle.path().resource_dir() {
        let p = res.join("mcp");
        if p.exists() { return p; }
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/mcp")
}

/// 将内置 MCP 服务器配置注册进客户端，返回注册数量
pub fn register_builtin_servers(mcp: &mut McpClient, app_handle: &tauri::AppHandle) -> usize {
    let dir = resolve_mcp_dir(app_handle);
    let servers = load_server_configs(&dir);
    let count = servers.len();
    for s in servers {
        mcp.register_server(s);
    }
    tracing::info!("[MCP] Registered {} builtin servers from {:?}", count, dir);
    count
}

// ─── 工具函数 ──────────────────────────────────────────────────────

/// 解析单个 SSE 事件块（`event:` / `data:` 行），返回 (事件类型, 数据)。多行 data 以 `\n` 拼接。
fn parse_sse_event(block: &str) -> Option<(String, String)> {
    let mut event = String::new();
    let mut data_lines: Vec<&str> = Vec::new();
    for line in block.lines() {
        if let Some(v) = line.strip_prefix("event:") {
            event = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("data:") {
            data_lines.push(v.trim_start_matches(' '));
        }
    }
    if data_lines.is_empty() {
        return None;
    }
    Some((event, data_lines.join("\n")))
}

/// 将（可能相对的）endpoint URI 解析为绝对 URL，相对 SSE URL 的 origin 解析
fn resolve_url(base: &str, endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return endpoint.to_string();
    }
    if let Some(scheme_end) = base.find("://") {
        let rest = &base[scheme_end + 3..];
        let host_end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
        let origin = &base[..scheme_end + 3 + host_end];
        return if endpoint.starts_with('/') {
            format!("{}{}", origin, endpoint)
        } else {
            format!("{}/{}", origin, endpoint)
        };
    }
    endpoint.to_string()
}

fn distill_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            if s.chars().count() > 80 {
                let preview: String = s.chars().take(77).collect();
                format!("{}...", preview)
            } else {
                s.clone()
            }
        },
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        serde_json::Value::Object(obj) => format!("{{{} keys}}", obj.len()),
        _ => value.to_string(),
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_registration() {
        let mut client = McpClient::new();
        client.register_server(McpServer {
            id: "postgres".into(), name: "PostgreSQL".into(),
            transport: McpTransport::Stdio { command: "pg-mcp".into(), args: vec!["--dsn".into()], env: HashMap::new() },
            tools_count: 0, resources_count: 0, connected: false, connection_failures: 0,
        });
        assert_eq!(client.servers.len(), 1);
    }

    #[test]
    fn test_connect_and_disconnect() {
        let mut client = McpClient::new();
        client.register_server(McpServer {
            id: "pg".into(), name: "PG".into(),
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: HashMap::new() },
            tools_count: 0, resources_count: 0, connected: false, connection_failures: 0,
        });
        client.connect("pg").unwrap();
        assert!(client.servers.get("pg").unwrap().connected);
        client.disconnect("pg").unwrap();
        assert!(!client.servers.get("pg").unwrap().connected);
    }

    #[test]
    fn test_tool_registration() {
        let mut client = McpClient::new();
        client.register_server(McpServer {
            id: "pg".into(), name: "PG".into(),
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: HashMap::new() },
            tools_count: 0, resources_count: 0, connected: false, connection_failures: 0,
        });
        client.register_tool("pg", McpTool {
            name: "query".into(), description: "Execute SQL".into(),
            input_schema: json!({"type":"object","properties":{"sql":{"type":"string"}}}),
        }).unwrap();
        assert_eq!(client.tools.get("pg").unwrap().len(), 1);
    }

    #[test]
    fn test_call_tool_disconnected() {
        let mut client = McpClient::new();
        client.register_server(McpServer {
            id: "pg".into(), name: "PG".into(),
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: HashMap::new() },
            tools_count: 0, resources_count: 0, connected: false, connection_failures: 0,
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.call_tool("pg", "query", &json!({})));
        assert!(!result.success);
    }

    #[test]
    fn test_distillation() {
        let client = McpClient::new();
        let small = json!({"a":1});
        let r = client.distill(&small);
        assert_eq!(r.compression_ratio, 1.0);

        let large = json!({"key":"x".repeat(2000)});
        let r = client.distill(&large);
        assert!(r.compression_ratio < 1.0);
    }

    #[test]
    fn test_prompt_fragment() {
        let mut client = McpClient::new();
        client.register_server(McpServer {
            id: "pg".into(), name: "PostgreSQL".into(),
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: HashMap::new() },
            tools_count: 0, resources_count: 0, connected: false, connection_failures: 0,
        });
        client.connect("pg").unwrap();
        client.register_tool("pg", McpTool { name: "query".into(), description: "SQL".into(), input_schema: json!({}) }).unwrap();
        let frag = client.generate_prompt_fragment();
        assert!(frag.contains("PostgreSQL"));
        assert!(frag.contains("query"));
    }

    #[test]
    fn test_spawn_server_mock() {
        // Uses 'echo' which exists on both Windows and Unix — just verifies no panic
        let result = McpClient::spawn_server("test", "echo", &["hello"]);
        assert!(result.is_ok());
        let (mut client, id) = result.unwrap();
        assert_eq!(id, "test");
        assert!(client.servers.contains_key("test"));
        assert!(client.processes.contains_key("test"));
        // Clean up
        client.disconnect("test").unwrap();
    }

    #[test]
    fn test_load_server_configs() {
        let dir = std::env::temp_dir().join("chronos_mcp_cfg_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("audit-server.json"), r#"{
            "name": "mcp-server-audit",
            "displayName": "Audit Vault",
            "transport": "stdio",
            "command": "node",
            "args": ["resources/mcp/audit-server.cjs"]
        }"#).unwrap();
        std::fs::write(dir.join("audit-server.cjs"), "// stub").unwrap();

        let servers = load_server_configs(&dir);
        assert_eq!(servers.len(), 1);
        let s = &servers[0];
        assert_eq!(s.id, "mcp-server-audit");
        assert_eq!(s.name, "Audit Vault");
        match &s.transport {
            McpTransport::Stdio { command, args, .. } => {
                assert_eq!(command, "node");
                // 相对脚本路径应解析为配置目录下的绝对路径
                assert_eq!(args[0], dir.join("audit-server.cjs").to_string_lossy());
            }
            _ => panic!("expected stdio transport"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_sse_event_endpoint() {
        let block = "event: endpoint\ndata: /messages?sessionId=abc\n\n";
        let (event, data) = parse_sse_event(block).unwrap();
        assert_eq!(event, "endpoint");
        assert_eq!(data, "/messages?sessionId=abc");
    }

    #[test]
    fn test_parse_sse_event_message() {
        let block = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}\n\n";
        let (event, data) = parse_sse_event(block).unwrap();
        assert_eq!(event, "message");
        assert!(data.contains("\"id\":7"));
    }

    #[test]
    fn test_parse_sse_event_multiline_data() {
        let block = "event: message\ndata: line1\ndata: line2\n\n";
        let (event, data) = parse_sse_event(block).unwrap();
        assert_eq!(event, "message");
        assert_eq!(data, "line1\nline2");
    }

    #[test]
    fn test_resolve_url_absolute() {
        assert_eq!(resolve_url("https://host/sse", "https://other/messages"), "https://other/messages");
    }

    #[test]
    fn test_resolve_url_relative() {
        assert_eq!(resolve_url("https://host:8080/sse", "/messages?sid=1"), "https://host:8080/messages?sid=1");
        assert_eq!(resolve_url("https://host/sse", "messages"), "https://host/messages");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn mcp_connect_and_init(state: tauri::State<'_, crate::state::AppState>, server_id: String) -> Result<String, String> {
    state.mcp_client.lock().await.connect_and_init(&server_id).await
        .map(|_| format!("MCP server '{}' connected and initialized", server_id))
}

#[tauri::command]
pub async fn mcp_fetch_tools(state: tauri::State<'_, crate::state::AppState>, server_id: String) -> Result<String, String> {
    let tools = state.mcp_client.lock().await.fetch_and_clean_tools(&server_id).await?;
    Ok(format!("Fetched {} tools from '{}'", tools.len(), server_id))
}

#[tauri::command]
pub fn mcp_disconnect(state: tauri::State<crate::state::AppState>, server_id: String) -> Result<String, String> {
    state.mcp_client.blocking_lock().disconnect(&server_id)
        .map(|_| format!("Disconnected {}", server_id))
}

#[tauri::command]
pub fn mcp_cleanup_stale(state: tauri::State<crate::state::AppState>) -> String {
    let mcp = state.mcp_client.blocking_lock();
    let count = mcp.connected_servers().len();
    format!("MCP cleanup check: {} active servers (zombie detection pending)", count)
}

#[tauri::command]
pub fn list_mcp_servers(state: tauri::State<crate::state::AppState>) -> Vec<McpServer> {
    state.mcp_client.blocking_lock().connected_servers().into_iter().cloned().collect()
}

#[tauri::command]
pub async fn mcp_register_builtin_servers(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    let mut mcp = state.mcp_client.lock().await;
    let count = register_builtin_servers(&mut mcp, &app_handle);
    Ok(format!("Registered {} builtin MCP servers", count))
}
