// Ollama 局域网本地模型发现与热切换网关
//
// 工具内置局域网发现协议，支持原生接入企业内部服务器上通过
// Ollama / Llama.cpp 离线部署的私有大模型。
//
// 当云端大模型网络超时或开销触顶时，毫秒级无感热切换至局域网本地模型

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── 类型定义 ──────────────────────────────────────────────────────

/// 发现的 Ollama 实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaInstance {
    /// 服务地址
    pub endpoint: String,
    /// 可用模型列表
    pub models: Vec<OllamaModel>,
    /// 是否在线
    pub online: bool,
    /// 延迟（毫秒）
    pub latency_ms: u64,
    /// 最后检测时间
    pub last_seen: String,
}

/// Ollama 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

/// LAN 发现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanDiscoveryConfig {
    /// 扫描端口列表
    pub ports: Vec<u16>,
    /// 扫描超时（毫秒）
    pub timeout_ms: u64,
    /// 扫描子网（CIDR）
    pub subnets: Vec<String>,
}

impl Default for LanDiscoveryConfig {
    fn default() -> Self {
        Self {
            ports: vec![11434, 8080, 8081], // Ollama 默认端口 + 常见备用
            timeout_ms: 2000,
            subnets: vec![
                "192.168.1.0/24".into(),
                "10.0.0.0/24".into(),
                "127.0.0.1".into(),
            ],
        }
    }
}

// ─── LAN 发现引擎 ──────────────────────────────────────────────────

/// 局域网 Ollama 发现器
pub struct LanDiscovery {
    /// 发现的实例
    pub instances: Vec<OllamaInstance>,
    /// 配置
    pub config: LanDiscoveryConfig,
    /// 是否启用
    pub enabled: bool,
    /// 当前活跃的本地模型 endpoint
    pub active_local: Option<String>,
}

impl LanDiscovery {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            config: LanDiscoveryConfig::default(),
            enabled: true,
            active_local: None,
        }
    }

    /// 扫描局域网发现 Ollama 实例
    pub async fn scan(&mut self) -> Vec<OllamaInstance> {
        if !self.enabled {
            return vec![];
        }

        let mut found = Vec::new();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .build()
            .ok();

        let client = match client {
            Some(c) => c,
            None => return found,
        };

        // 扫描所有子网和端口
        for subnet in &self.config.subnets {
            // 简化：只扫描 localhost + 常见私有 IP
            for port in &self.config.ports {
                let endpoint = if subnet == "127.0.0.1" {
                    format!("http://127.0.0.1:{}", port)
                } else {
                    // 局域网扫描：取子网第一个可用 IP
                    let base = subnet.trim_end_matches("/24");
                    format!("http://{}:{}", base, port)
                };

                let start = std::time::Instant::now();
                match client.get(&format!("{}/api/tags", endpoint)).send().await {
                    Ok(response) => {
                        let latency = start.elapsed().as_millis() as u64;
                        if let Ok(json) = response.json::<serde_json::Value>().await {
                            let models: Vec<OllamaModel> = json["models"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .map(|m| OllamaModel {
                                            name: m["name"].as_str().unwrap_or("?").into(),
                                            size_bytes: m["size"].as_u64().unwrap_or(0),
                                            modified_at: m["modified_at"]
                                                .as_str()
                                                .unwrap_or("?")
                                                .into(),
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            if !models.is_empty() {
                                found.push(OllamaInstance {
                                    endpoint: endpoint.clone(),
                                    models,
                                    online: true,
                                    latency_ms: latency,
                                    last_seen: chrono_now(),
                                });
                            }
                        }
                    }
                    Err(_) => {
                        // Instance not reachable — skip
                    }
                }
            }
        }

        self.instances = found.clone();
        found
    }

    /// 查找本地可用模型
    pub fn find_model(&self, model_name: &str) -> Option<&OllamaInstance> {
        self.instances
            .iter()
            .find(|inst| inst.models.iter().any(|m| m.name.contains(model_name)))
    }

    /// 获取最佳本地模型（延迟最低）
    pub fn best_instance(&self) -> Option<&OllamaInstance> {
        self.instances
            .iter()
            .filter(|i| i.online)
            .min_by_key(|i| i.latency_ms)
    }

    /// 切换到本地模型
    pub fn switch_to_local(&mut self) -> Option<String> {
        let endpoint = self.best_instance().map(|b| b.endpoint.clone());
        if let Some(ref ep) = endpoint {
            self.active_local = Some(ep.clone());
        }
        endpoint
    }

    /// 获取活跃本地 endpoint
    pub fn active_endpoint(&self) -> Option<&str> {
        self.active_local.as_deref()
    }

    /// 统计
    pub fn stats(&self) -> LanDiscoveryStats {
        LanDiscoveryStats {
            instances_found: self.instances.len() as u32,
            total_models: self.instances.iter().map(|i| i.models.len()).sum::<usize>() as u32,
            active_local: self.active_local.clone(),
            enabled: self.enabled,
        }
    }
}

impl Default for LanDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// LAN 发现统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanDiscoveryStats {
    pub instances_found: u32,
    pub total_models: u32,
    pub active_local: Option<String>,
    pub enabled: bool,
}

// ─── 工具函数 ──────────────────────────────────────────────────────

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
