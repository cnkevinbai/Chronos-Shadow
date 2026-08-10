// 用户画像与个性化引擎 (User Persona & Personalization Engine)
//
// 让冷冰冰的项目拥有温度 — 记住你的名字、偏好、成就，
// 像真正的人类助手一样与你互动。
//
// 核心功能：
//   1. 用户画像 — 名字/昵称/头像/时区/语言偏好
//   2. 个性化问候 — 基于时间的温暖问候语
//   3. 成就系统 — 使用里程碑追踪
//   4. 情感状态 — 系统"心情"与共情能力
//   5. 使用节奏 — 活跃时段/连续使用天数/总交互次数

use serde::{Deserialize, Serialize};
use chrono::Timelike;

// ─── 用户画像 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// 用户名称
    pub display_name: String,
    /// 昵称（用于温暖问候）
    pub nickname: String,
    /// 头像 emoji
    pub avatar: String,
    /// 时区偏移 (小时)
    pub timezone_offset: i32,
    /// 语言偏好
    pub language: String,
    /// 主题偏好
    pub theme: String,
    /// 首次使用时间
    pub first_seen: String,
    /// 最后活跃时间
    pub last_active: String,
    /// 总交互次数
    pub total_interactions: u64,
    /// 连续使用天数
    pub streak_days: u32,
    /// 今日交互次数
    pub today_interactions: u32,
    /// 系统个性: "professional" | "friendly" | "playful"
    pub personality: String,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "开发者".into(),
            nickname: "伙伴".into(),
            avatar: "🦀".into(),
            timezone_offset: 8,
            language: "zh-CN".into(),
            theme: "dark".into(),
            first_seen: chrono::Utc::now().to_rfc3339(),
            last_active: chrono::Utc::now().to_rfc3339(),
            total_interactions: 0,
            streak_days: 1,
            today_interactions: 0,
            personality: "friendly".into(),
        }
    }
}

impl UserProfile {
    /// 基于时间的个性化问候
    pub fn greeting(&self) -> String {
        let hour = self.current_hour();
        let time_greeting = match hour {
            0..=5 => "夜深了，还在为梦想奋斗 🌙",
            6..=8 => "早上好，新的一天充满可能 ☀️",
            9..=11 => "上午好，精力充沛的时刻 💪",
            12..=13 => "中午好，别忘了休息一下 🍵",
            14..=17 => "下午好，继续加油 🚀",
            18..=20 => "傍晚好，今天辛苦了 🌅",
            _ => "晚上好，放松一下 🌃",
        };

        match self.personality.as_str() {
            "professional" => format!("{}，{}。", time_greeting, self.display_name),
            "playful" => format!("{}！{}，今天想玩点什么？🎮", time_greeting, self.nickname),
            _ => format!("{}，{}~ 我在这里陪着你 ❤️", time_greeting, self.nickname),
        }
    }

    /// 共情回复模板
    pub fn empathy_message(&self, context: &str) -> String {
        match context {
            "error" => format!("{}，遇到了一点小麻烦…让我来帮你解决吧 🔧", self.nickname),
            "success" => format!("太棒了，{}！我们一起做到了 🎉", self.nickname),
            "long_session" => format!("{}，你已经连续工作很久了，休息一下吧 ☕", self.nickname),
            "returning" => format!("欢迎回来，{}！{}天不见，想你了 💫", self.nickname, self.streak_days),
            "achievement" => format!("🏆 恭喜{}！解锁了新成就", self.nickname),
            _ => format!("{}，有什么我可以帮你的？", self.nickname),
        }
    }

    /// 系统"心跳" — 返回当前活跃状态描述
    pub fn heartbeat(&self) -> serde_json::Value {
        let hour = self.current_hour();
        let energy = match hour { 6..=11 => "high", 12..=17 => "medium", _ => "low" };
        let mood = if self.today_interactions > 50 { "excited" }
            else if self.today_interactions > 10 { "happy" }
            else { "calm" };

        serde_json::json!({
            "energy": energy,
            "mood": mood,
            "streak": self.streak_days,
            "total_interactions": self.total_interactions,
            "today": self.today_interactions,
            "avatar": self.avatar,
        })
    }

    /// 记录一次交互
    pub fn touch(&mut self) {
        self.total_interactions += 1;
        self.today_interactions += 1;
        self.last_active = chrono::Utc::now().to_rfc3339();
    }

    fn current_hour(&self) -> u32 {
        let utc = chrono::Utc::now();
        let local = utc + chrono::Duration::hours(self.timezone_offset as i64);
        local.hour()
    }
}

// ─── 成就系统 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub emoji: String,
    pub unlocked: bool,
    pub unlocked_at: Option<String>,
    pub progress: f32, // 0.0-1.0
}

impl Achievement {
    pub fn all() -> Vec<Self> {
        vec![
            Self { id: "first-chat".into(), name: "初次对话".into(), description: "发送第一条消息".into(), emoji: "💬".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
            Self { id: "streak-3".into(), name: "三日连续".into(), description: "连续使用3天".into(), emoji: "🔥".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
            Self { id: "streak-7".into(), name: "一周坚持".into(), description: "连续使用7天".into(), emoji: "⭐".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
            Self { id: "msg-100".into(), name: "百条消息".into(), description: "累计发送100条消息".into(), emoji: "💯".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
            Self { id: "approve-10".into(), name: "审批达人".into(), description: "完成10次审批".into(), emoji: "🛡️".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
            Self { id: "skill-used".into(), name: "技能大师".into(), description: "使用本地Skill完成一次任务".into(), emoji: "⚡".into(), unlocked: false, unlocked_at: None, progress: 0.0 },
        ]
    }

    pub fn update_progress(&mut self, profile: &UserProfile, approvals: u32, skills_used: u32) {
        match self.id.as_str() {
            "first-chat" => { self.unlocked = profile.total_interactions >= 1; self.progress = if self.unlocked { 1.0 } else { 0.0 }; }
            "streak-3" => { self.unlocked = profile.streak_days >= 3; self.progress = (profile.streak_days as f32 / 3.0).min(1.0); }
            "streak-7" => { self.unlocked = profile.streak_days >= 7; self.progress = (profile.streak_days as f32 / 7.0).min(1.0); }
            "msg-100" => { self.unlocked = profile.total_interactions >= 100; self.progress = (profile.total_interactions as f32 / 100.0).min(1.0); }
            "approve-10" => { self.unlocked = approvals >= 10; self.progress = (approvals as f32 / 10.0).min(1.0); }
            "skill-used" => { self.unlocked = skills_used >= 1; self.progress = if self.unlocked { 1.0 } else { 0.0 }; }
            _ => {}
        }
        if self.unlocked && self.unlocked_at.is_none() {
            self.unlocked_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_variations() {
        let mut profile = UserProfile::default();
        profile.nickname = "测试员".into();
        let g = profile.greeting();
        assert!(g.contains("测试员"));
        assert!(g.len() > 10);
    }

    #[test]
    fn test_empathy_error() {
        let profile = UserProfile::default();
        let msg = profile.empathy_message("error");
        assert!(msg.contains("🔧") || msg.contains("解决"));
    }

    #[test]
    fn test_achievement_progress() {
        let mut achievements = Achievement::all();
        let profile = UserProfile { total_interactions: 50, ..Default::default() };
        for a in &mut achievements { a.update_progress(&profile, 0, 0); }
        let msg100 = achievements.iter().find(|a| a.id == "msg-100").unwrap();
        assert!((msg100.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_touch_increments() {
        let mut profile = UserProfile::default();
        profile.touch();
        profile.touch();
        assert_eq!(profile.total_interactions, 2);
        assert_eq!(profile.today_interactions, 2);
    }
}
