// 交互式 AI "影子结对驾驶" (Shadow Mode)
//
// 非侵入式随航：当用户在原生 IDE（VS Code 等）或生产力软件中正常工作时，
// CS-Agent 利用 Windows 钩子（Hooks）在后台静默监听。
//
// 核心功能：
// - Windows 键盘/鼠标钩子低消耗监听
// - 上下文理解：分析用户当前编辑的文件 + 编译输出
// - 主动纠错：长时间停顿或编译死循环时浮现"一键自愈"智能卡片
// - 唤醒词：用户可通过特定快捷键或语音唤醒

use serde::{Deserialize, Serialize};

// ─── 类型定义 ──────────────────────────────────────────────────────

/// 影子模式状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowState {
    /// 休眠（未激活）
    Dormant,
    /// 监听中（后台静默运行）
    Listening,
    /// 分析中（检测到潜在问题）
    Analyzing,
    /// 主动建议（智能卡片浮现）
    Suggesting(String),
    /// 暂停（用户手动关闭）
    Paused,
}

/// 用户活动事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityEvent {
    /// 键盘输入
    KeyPress { key: String, timestamp: String },
    /// 鼠标移动
    MouseMove { x: i32, y: i32, timestamp: String },
    /// IDE 文件切换
    FileSwitch { path: String, timestamp: String },
    /// 编译开始
    BuildStart { project: String, timestamp: String },
    /// 编译错误
    BuildError { error: String, count: u32, timestamp: String },
    /// 长时间停顿（无活动）
    IdleTimeout { duration_secs: u64, timestamp: String },
}

/// 智能建议卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionCard {
    /// 建议 ID
    pub id: String,
    /// 建议标题
    pub title: String,
    /// 建议描述
    pub description: String,
    /// 操作类型
    pub action: SuggestionAction,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 触发原因
    pub trigger: String,
}

/// 建议操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionAction {
    /// 一键修复命令
    Fix { command: String, description: String },
    /// 打开文档
    OpenDocs { url: String },
    /// 运行诊断
    RunDiagnostics,
    /// 回滚到上一个快照
    RewindToSnapshot { snapshot_id: String },
    /// 忽略
    Dismiss,
}

// ─── 影子引擎 ──────────────────────────────────────────────────────

/// 影子随航引擎
pub struct ShadowEngine {
    /// 当前状态
    pub state: ShadowState,
    /// 是否启用
    pub enabled: bool,
    /// 停顿阈值（秒）— 超过此时间未活动触发分析
    pub idle_threshold_secs: u64,
    /// 编译错误阈值 — 超过此数量触发建议
    pub error_threshold: u32,
    /// 累计错误数
    pub error_count: u32,
    /// 已生成的建议列表
    pub suggestions: Vec<SuggestionCard>,
    /// 已忽略的建议数
    pub dismissed_count: u32,
    /// 已采纳的建议数
    pub accepted_count: u32,
}

impl ShadowEngine {
    pub fn new() -> Self {
        Self {
            state: ShadowState::Dormant,
            enabled: false,
            idle_threshold_secs: 30,
            error_threshold: 3,
            error_count: 0,
            suggestions: Vec::new(),
            dismissed_count: 0,
            accepted_count: 0,
        }
    }

    /// 激活影子模式
    pub fn activate(&mut self) {
        self.enabled = true;
        self.state = ShadowState::Listening;
    }

    /// 暂停影子模式
    pub fn pause(&mut self) {
        self.state = ShadowState::Paused;
    }

    /// 恢复影子模式
    pub fn resume(&mut self) {
        if self.enabled {
            self.state = ShadowState::Listening;
        }
    }

    /// 处理编译错误事件
    pub fn on_build_error(&mut self, error: &str) -> Option<SuggestionCard> {
        if !self.enabled {
            return None;
        }

        self.error_count += 1;
        self.state = ShadowState::Analyzing;

        if self.error_count >= self.error_threshold {
            let card = SuggestionCard {
                id: format!("fix-{}", self.suggestions.len() + 1),
                title: "检测到重复编译错误".into(),
                description: format!(
                    "已连续 {} 次编译失败：{}。建议启动自愈修复流程。",
                    self.error_count, error
                ),
                action: SuggestionAction::Fix {
                    command: "cargo check --fix".into(),
                    description: "自动修复常见编译错误".into(),
                },
                confidence: 0.85,
                trigger: "build_error_loop".into(),
            };
            self.suggestions.push(card.clone());
            self.state = ShadowState::Suggesting(card.id.clone());
            Some(card)
        } else {
            None
        }
    }

    /// 处理空闲超时
    pub fn on_idle(&mut self, duration_secs: u64) -> Option<SuggestionCard> {
        if !self.enabled || duration_secs < self.idle_threshold_secs {
            return None;
        }

        let card = SuggestionCard {
            id: format!("idle-{}", self.suggestions.len() + 1),
            title: "检测到长时间停顿".into(),
            description: format!(
                "您已 {} 秒未操作。需要我帮您检查当前代码吗？",
                duration_secs
            ),
            action: SuggestionAction::RunDiagnostics,
            confidence: 0.6,
            trigger: "idle_timeout".into(),
        };
        self.suggestions.push(card.clone());
        self.state = ShadowState::Suggesting(card.id.clone());
        Some(card)
    }

    /// 采纳建议
    pub fn accept_suggestion(&mut self, id: &str) -> bool {
        if let Some(card) = self.suggestions.iter().find(|s| s.id == id) {
            self.accepted_count += 1;
            self.error_count = 0;
            self.state = ShadowState::Listening;
            tracing::info!("[Shadow] Accepted suggestion: {}", card.title);
            true
        } else {
            false
        }
    }

    /// 忽略建议
    pub fn dismiss_suggestion(&mut self, _id: &str) {
        self.dismissed_count += 1;
        self.state = ShadowState::Listening;
    }

    /// 重置错误计数
    pub fn reset_errors(&mut self) {
        self.error_count = 0;
    }

    /// 统计信息
    pub fn stats(&self) -> ShadowStats {
        ShadowStats {
            state: format!("{:?}", self.state),
            enabled: self.enabled,
            suggestions_generated: self.suggestions.len() as u32,
            accepted: self.accepted_count,
            dismissed: self.dismissed_count,
        }
    }
}

impl Default for ShadowEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 影子模式统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowStats {
    pub state: String,
    pub enabled: bool,
    pub suggestions_generated: u32,
    pub accepted: u32,
    pub dismissed: u32,
}
