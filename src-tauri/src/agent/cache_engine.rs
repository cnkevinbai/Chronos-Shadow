// 统一缓存引擎 (Unified Cache Engine)
//
// 跨模块缓存层，为所有模型交互提供缓存加速：
//   - Web Search 结果缓存（相同查询 → 缓存命中，避免重复 API 调用）
//   - Web Fetch 内容缓存（相同 URL → 缓存内容，支持 TTL 过期）
//   - 蒸馏结果缓存（相同 URL+级别 → 命中蒸馏引擎缓存层）
//   - LLM 响应缓存（相同 prompt hash → 复用响应，节省资费）
//
// 设计原则：
//   1. 统一接口 — 所有模块通过相同 API 读写缓存
//   2. 分类 TTL — 搜索/抓取/蒸馏/LLM 各有独立过期策略
//   3. LRU 淘汰 — 内存限制自动淘汰最久未用条目
//   4. 磁盘持久 — 缓存可落盘，重启后恢复
//   5. 统计透明 — 命中率/节省量实时可见

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── 缓存分类 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheCategory {
    /// Web 搜索结果
    WebSearch,
    /// Web 抓取内容
    WebFetch,
    /// 蒸馏文档
    Distillation,
    /// LLM API 响应
    LlmResponse,
    /// 通用键值
    General,
}

impl CacheCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::WebSearch => "Web搜索",
            Self::WebFetch => "Web抓取",
            Self::Distillation => "蒸馏文档",
            Self::LlmResponse => "LLM响应",
            Self::General => "通用",
        }
    }

    /// 默认 TTL（秒）
    pub fn default_ttl_secs(&self) -> u64 {
        match self {
            Self::WebSearch => 600,      // 10分钟
            Self::WebFetch => 3600,      // 1小时
            Self::Distillation => 86400, // 1天
            Self::LlmResponse => 3600,   // 1小时
            Self::General => 300,        // 5分钟
        }
    }
}

// ─── 缓存条目 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// 缓存键
    pub key: String,
    /// 缓存分类
    pub category: CacheCategory,
    /// 缓存值（JSON 序列化）
    pub value: String,
    /// 原始大小（字节）
    pub original_size: usize,
    /// 创建时间戳
    pub created_at: u64,
    /// 过期时间戳
    pub expires_at: u64,
    /// 访问次数
    pub hit_count: u64,
    /// 最后访问时间
    pub last_accessed: u64,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        let now = now_secs();
        now >= self.expires_at
    }

    pub fn touch(&mut self) {
        self.hit_count += 1;
        self.last_accessed = now_secs();
    }
}

// ─── 缓存统计 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// 总命中次数
    pub total_hits: u64,
    /// 总未命中次数
    pub total_misses: u64,
    /// 命中率
    pub hit_rate: f64,
    /// 因过期淘汰的条目数
    pub expired_entries: u64,
    /// 因 LRU 淘汰的条目数
    pub evicted_entries: u64,
    /// 累计节省 API 调用
    pub api_calls_saved: u64,
    /// 累计节省字节（避免重复下载）
    pub bytes_saved: u64,
    /// 分类型统计
    pub per_category: HashMap<String, CategoryStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

// ─── 统一缓存引擎 ──────────────────────────────────────────────────

pub struct UnifiedCache {
    /// 缓存存储（分类 → 键 → 条目）
    store: HashMap<CacheCategory, HashMap<String, CacheEntry>>,
    /// 最大内存条目数
    max_entries: usize,
    /// 最大内存大小（字节，估算）
    max_memory_bytes: usize,
    /// 当前内存使用（估算）
    current_memory: usize,
    /// 统计
    stats: CacheStats,
    /// 分类 TTL 覆盖
    custom_ttls: HashMap<CacheCategory, u64>,
    /// 是否启用
    pub enabled: bool,
}

impl UnifiedCache {
    pub fn new() -> Self {
        let mut store = HashMap::new();
        for cat in &[
            CacheCategory::WebSearch,
            CacheCategory::WebFetch,
            CacheCategory::Distillation,
            CacheCategory::LlmResponse,
            CacheCategory::General,
        ] {
            store.insert(*cat, HashMap::new());
        }

        let mut per_category = HashMap::new();
        for cat in &[
            CacheCategory::WebSearch,
            CacheCategory::WebFetch,
            CacheCategory::Distillation,
            CacheCategory::LlmResponse,
            CacheCategory::General,
        ] {
            per_category.insert(cat.label().to_string(), CategoryStat {
                entries: 0, hits: 0, misses: 0, hit_rate: 0.0,
            });
        }

        Self {
            store,
            max_entries: 10000,
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB
            current_memory: 0,
            stats: CacheStats {
                total_entries: 0,
                total_hits: 0,
                total_misses: 0,
                hit_rate: 0.0,
                expired_entries: 0,
                evicted_entries: 0,
                api_calls_saved: 0,
                bytes_saved: 0,
                per_category,
            },
            custom_ttls: HashMap::new(),
            enabled: true,
        }
    }

    // ── CRUD ────────────────────────────────────────────────────

    /// 🔬 语义去重: 相似请求指纹匹配, 10s 窗口内复用响应
    pub fn semantic_dedup_check(&mut self, request_fingerprint: &str, window_ms: u64) -> Option<String> {
        if !self.enabled { return None; }
        let now = now_secs() * 1000;
        // 收集匹配的 key+value (避免同时持有不可变和可变借用)
        let mut match_key: Option<(CacheCategory, String, String, usize)> = None;
        for (&cat, entries) in self.store.iter() {
            for (key, entry) in entries.iter() {
                if key.contains(request_fingerprint) && !entry.is_expired() {
                    let age_ms = now.saturating_sub(entry.created_at * 1000);
                    if age_ms < window_ms {
                        match_key = Some((cat, key.clone(), entry.value.clone(), entry.original_size));
                        break;
                    }
                }
            }
            if match_key.is_some() { break; }
        }
        // 更新统计 (需要 &mut self)
        if let Some((cat, key, value, size)) = match_key {
            if let Some(entries) = self.store.get_mut(&cat) {
                if let Some(entry) = entries.get_mut(&key) {
                    entry.touch();
                }
            }
            self.stats.total_hits += 1;
            self.stats.api_calls_saved += 1;
            self.stats.bytes_saved += size as u64;
            return Some(value);
        }
        None
    }

    /// 带语义指纹的存储
    pub fn set_with_fingerprint(
        &mut self, category: CacheCategory, key: &str, fingerprint: &str,
        value: String, original_size: usize, ttl_override: Option<u64>,
    ) {
        let composite = format!("{}|fp:{}", key, fingerprint);
        self.set(category, &composite, value, original_size, ttl_override);
    }

    /// 获取缓存条目
    pub fn get(&mut self, category: CacheCategory, key: &str) -> Option<String> {
        if !self.enabled { self.stats.total_misses += 1; return None; }

        if let Some(entries) = self.store.get_mut(&category) {
            if let Some(entry) = entries.get_mut(key) {
                if entry.is_expired() {
                    self.stats.expired_entries += 1;
                    self.stats.total_misses += 1;
                    let _ = entries.remove(key);
                    self.stats.total_entries -= 1;
                    return None;
                }
                entry.touch();
                self.stats.total_hits += 1;
                self.stats.api_calls_saved += 1;
                self.stats.bytes_saved += entry.original_size as u64;
                return Some(entry.value.clone());
            }
        }
        self.stats.total_misses += 1;
        None
    }

    /// 设置缓存条目
    pub fn set(
        &mut self,
        category: CacheCategory,
        key: &str,
        value: String,
        original_size: usize,
        ttl_override: Option<u64>,
    ) {
        if !self.enabled { return; }

        let ttl = ttl_override.unwrap_or_else(|| {
            self.custom_ttls.get(&category).copied().unwrap_or_else(|| category.default_ttl_secs())
        });

        let now = now_secs();
        let entry = CacheEntry {
            key: key.to_string(),
            category,
            value: value.clone(),
            original_size,
            created_at: now,
            expires_at: now + ttl,
            hit_count: 0,
            last_accessed: now,
        };

        let value_size = value.len();
        self.current_memory += value_size;

        // 淘汰检查
        while self.current_memory > self.max_memory_bytes
            || self.total_entries() > self.max_entries
        {
            self.evict_lru();
        }

        if let Some(entries) = self.store.get_mut(&category) {
            if entries.insert(key.to_string(), entry).is_none() {
                self.stats.total_entries += 1;
            }
        }
    }

    /// 获取或计算（类似 memoize）
    pub fn get_or_compute<F>(
        &mut self,
        category: CacheCategory,
        key: &str,
        original_size: usize,
        compute: F,
    ) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(val) = self.get(category, key) {
            return Some(val);
        }
        let value = compute()?;
        self.set(category, key, value.clone(), original_size, None);
        Some(value)
    }

    /// 删除条目
    pub fn remove(&mut self, category: CacheCategory, key: &str) {
        if let Some(entries) = self.store.get_mut(&category) {
            if let Some(entry) = entries.remove(key) {
                self.current_memory = self.current_memory.saturating_sub(entry.value.len());
                self.stats.total_entries -= 1;
            }
        }
    }

    /// 清除整个分类
    pub fn clear_category(&mut self, category: CacheCategory) {
        if let Some(entries) = self.store.get_mut(&category) {
            for entry in entries.values() {
                self.current_memory = self.current_memory.saturating_sub(entry.value.len());
            }
            self.stats.total_entries -= entries.len();
            entries.clear();
        }
    }

    // ── 自适应 TTL (EvolutionBus 驱动) ───────────────────────

    /// 根据命中模式自适应调整 TTL
    /// 高频命中 → 延长 TTL；低频命中 → 缩短 TTL
    pub fn adapt_ttl(&mut self, category: CacheCategory, hit_rate: f64) -> f64 {
        let current = self.custom_ttls.get(&category)
            .copied()
            .unwrap_or_else(|| category.default_ttl_secs());

        // 高命中率 → 延长缓存 (最多 4×)
        let adjusted = if hit_rate > 0.8 {
            (current as f64 * 1.5).min(category.default_ttl_secs() as f64 * 4.0)
        } else if hit_rate > 0.5 {
            (current as f64 * 1.1).min(category.default_ttl_secs() as f64 * 2.0)
        } else if hit_rate < 0.2 {
            (current as f64 * 0.7).max(30.0) // 最少保留 30 秒
        } else {
            current as f64
        };

        let new_ttl = adjusted as u64;
        if new_ttl != current {
            self.custom_ttls.insert(category, new_ttl);
            tracing::info!(
                "[Cache] Adaptive TTL {}: {}s → {}s (hit_rate={:.1}%)",
                category.label(), current, new_ttl, hit_rate * 100.0
            );
        }
        new_ttl as f64
    }

    /// 获取分类命中率
    pub fn category_hit_rate(&self, category: CacheCategory) -> f64 {
        let stats = self.stats();
        stats.per_category.get(&category.label().to_string())
            .map(|s| s.hit_rate)
            .unwrap_or(0.0)
    }

    /// 获取当前 TTL
    pub fn current_ttl(&self, category: CacheCategory) -> u64 {
        self.custom_ttls.get(&category)
            .copied()
            .unwrap_or_else(|| category.default_ttl_secs())
    }

    /// 清除所有过期条目
    pub fn purge_expired(&mut self) -> u64 {
        let mut removed = 0u64;
        for entries in self.store.values_mut() {
            let before = entries.len();
            entries.retain(|_, e| !e.is_expired());
            removed += (before - entries.len()) as u64;
        }
        self.stats.total_entries -= removed as usize;
        removed
    }

    // ── 配置 ────────────────────────────────────────────────────

    pub fn set_ttl(&mut self, category: CacheCategory, ttl_secs: u64) {
        self.custom_ttls.insert(category, ttl_secs);
    }

    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
    }

    pub fn set_max_memory(&mut self, max_bytes: usize) {
        self.max_memory_bytes = max_bytes;
    }

    // ── 统计 ────────────────────────────────────────────────────

    fn total_entries(&self) -> usize {
        self.store.values().map(|e| e.len()).sum()
    }

    pub fn stats(&self) -> CacheStats {
        let mut stats = self.stats.clone();
        stats.total_entries = self.total_entries();
        stats.hit_rate = if stats.total_hits + stats.total_misses > 0 {
            stats.total_hits as f64 / (stats.total_hits + stats.total_misses) as f64
        } else { 0.0 };

        // 更新分类型统计
        for (cat, entries) in &self.store {
            let label = cat.label().to_string();
            let hits = entries.values().map(|e| e.hit_count).sum::<u64>();
            let misses = stats.per_category.get(&label).map(|c| c.misses).unwrap_or(0);
            stats.per_category.insert(label.clone(), CategoryStat {
                entries: entries.len(),
                hits,
                misses,
                hit_rate: if hits + misses > 0 { hits as f64 / (hits + misses) as f64 } else { 0.0 },
            });
        }

        stats
    }

    // ── 持久化 ──────────────────────────────────────────────────

    pub fn save_to_disk(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("unified_cache.json");
        let mut persistable: HashMap<String, serde_json::Value> = HashMap::new();

        for (cat, entries) in &self.store {
            let cat_key = format!("{:?}", cat);
            let cat_entries: Vec<serde_json::Value> = entries.values()
                .filter(|e| !e.is_expired())
                .map(|e| serde_json::json!({
                    "key": e.key,
                    "value": e.value,
                    "original_size": e.original_size,
                    "created_at": e.created_at,
                    "expires_at": e.expires_at,
                }))
                .collect();
            persistable.insert(cat_key, serde_json::Value::Array(cat_entries));
        }

        let json = serde_json::to_string_pretty(&persistable)
            .map_err(|e| e.to_string())?;
        std::fs::write(&path, &json).map_err(|e| e.to_string())?;
        Ok(format!("Cache persisted to {:?} ({} categories)", path, self.store.len()))
    }

    pub fn load_from_disk(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("unified_cache.json");
        if !path.exists() { return Ok("No cache file found".into()); }

        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let data: HashMap<String, serde_json::Value> = serde_json::from_str(&json)
            .map_err(|e| e.to_string())?;

        let mut loaded = 0usize;

        for (cat_key, entries_val) in &data {
            let category = match cat_key.as_str() {
                "WebSearch" => CacheCategory::WebSearch,
                "WebFetch" => CacheCategory::WebFetch,
                "Distillation" => CacheCategory::Distillation,
                "LlmResponse" => CacheCategory::LlmResponse,
                _ => CacheCategory::General,
            };

            if let Some(arr) = entries_val.as_array() {
                for entry_json in arr {
                    let key = entry_json["key"].as_str().unwrap_or("").to_string();
                    let value = entry_json["value"].as_str().unwrap_or("").to_string();
                    let original_size = entry_json["original_size"].as_u64().unwrap_or(0) as usize;
                    let created_at = entry_json["created_at"].as_u64().unwrap_or(0);
                    let expires_at = entry_json["expires_at"].as_u64().unwrap_or(0);

                    if expires_at > now_secs() {
                        let entry = CacheEntry {
                            key: key.clone(),
                            category,
                            value,
                            original_size,
                            created_at,
                            expires_at,
                            hit_count: 0,
                            last_accessed: now_secs(),
                        };
                        if let Some(entries) = self.store.get_mut(&category) {
                            entries.insert(key, entry);
                            loaded += 1;
                        }
                    }
                }
            }
        }

        self.stats.total_entries = self.total_entries();
        Ok(format!("Cache loaded: {} entries from {:?}", loaded, path))
    }

    // ── 内部 ────────────────────────────────────────────────────

    #[allow(dead_code)]
    fn update_category_stats(&mut self, category: CacheCategory, hit: bool) {
        let label = category.label().to_string();
        if let Some(stat) = self.stats.per_category.get_mut(&label) {
            if hit { stat.hits += 1; } else { stat.misses += 1; }
            stat.hit_rate = if stat.hits + stat.misses > 0 {
                stat.hits as f64 / (stat.hits + stat.misses) as f64
            } else { 0.0 };
            stat.entries = self.store.get(&category).map(|e| e.len()).unwrap_or(0);
        }
    }

    fn evict_lru(&mut self) {
        let mut oldest: Option<(CacheCategory, String, u64)> = None;

        for (cat, entries) in &self.store {
            for (key, entry) in entries {
                match &oldest {
                    None => oldest = Some((*cat, key.clone(), entry.last_accessed)),
                    Some((_, _, ts)) if entry.last_accessed < *ts => {
                        oldest = Some((*cat, key.clone(), entry.last_accessed));
                    }
                    _ => {}
                }
            }
        }

        if let Some((cat, key, _)) = oldest {
            if let Some(entries) = self.store.get_mut(&cat) {
                if let Some(entry) = entries.remove(&key) {
                    self.current_memory = self.current_memory.saturating_sub(entry.value.len());
                    self.stats.total_entries -= 1;
                    self.stats.evicted_entries += 1;
                }
            }
        }
    }
}

impl Default for UnifiedCache {
    fn default() -> Self { Self::new() }
}

// ─── 工具 ──────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_get() {
        let mut cache = UnifiedCache::new();
        cache.set(CacheCategory::WebSearch, "rust-async", r#"{"results":["doc1","doc2"]}"#.into(), 50, None);

        let entry = cache.get(CacheCategory::WebSearch, "rust-async");
        assert!(entry.is_some());
        assert!(entry.unwrap().contains("doc1"));
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = UnifiedCache::new();
        assert!(cache.get(CacheCategory::WebSearch, "nonexistent").is_none());
        assert_eq!(cache.stats.total_misses, 1);
    }

    #[test]
    fn test_cache_expiry() {
        let mut cache = UnifiedCache::new();
        // 设置 TTL 为 0 → 立即过期
        cache.set_ttl(CacheCategory::General, 0);
        cache.set(CacheCategory::General, "ephemeral", "data".into(), 4, Some(0));

        assert!(cache.get(CacheCategory::General, "ephemeral").is_none());
    }

    #[test]
    fn test_get_or_compute() {
        let mut cache = UnifiedCache::new();
        let compute_count = std::cell::RefCell::new(0);

        let r1 = cache.get_or_compute(CacheCategory::General, "key1", 10, || {
            *compute_count.borrow_mut() += 1;
            Some("computed_value".into())
        });
        assert_eq!(r1.unwrap(), "computed_value");
        assert_eq!(*compute_count.borrow(), 1);

        // Second call should hit cache
        let r2 = cache.get_or_compute(CacheCategory::General, "key1", 10, || {
            *compute_count.borrow_mut() += 1;
            Some("recomputed".into())
        });
        assert_eq!(r2.unwrap(), "computed_value");
        assert_eq!(*compute_count.borrow(), 1); // Not recomputed
    }

    #[test]
    fn test_purge_expired() {
        let mut cache = UnifiedCache::new();
        cache.set_ttl(CacheCategory::General, 0);
        cache.set(CacheCategory::General, "old", "stale".into(), 5, Some(0));
        cache.set(CacheCategory::WebSearch, "fresh", "good".into(), 4, Some(3600));

        let removed = cache.purge_expired();
        assert!(removed >= 1);
        assert!(cache.get(CacheCategory::WebSearch, "fresh").is_some());
    }

    #[test]
    fn test_category_ttl() {
        assert_eq!(CacheCategory::WebSearch.default_ttl_secs(), 600);
        assert_eq!(CacheCategory::WebFetch.default_ttl_secs(), 3600);
        assert_eq!(CacheCategory::Distillation.default_ttl_secs(), 86400);
        assert_eq!(CacheCategory::LlmResponse.default_ttl_secs(), 3600);
    }

    #[test]
    fn test_stats() {
        let mut cache = UnifiedCache::new();
        cache.set(CacheCategory::WebSearch, "q1", "r1".into(), 2, None);
        cache.get(CacheCategory::WebSearch, "q1");
        cache.get(CacheCategory::WebSearch, "q1");
        cache.get(CacheCategory::WebSearch, "q2"); // miss

        let stats = cache.stats();
        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert!(stats.hit_rate > 0.6);
    }
}