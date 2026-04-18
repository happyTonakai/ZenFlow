//! ZenFlow 配置常量

use std::env;

use crate::settings;

// === 评分 API 配置 ===

/// 获取评分 API Base URL（从设置中读取）
pub fn scoring_api_base_url() -> String {
    settings::get_settings()
        .map(|s| s.scoring_api_base_url)
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
}

/// 获取评分模型（从设置中读取）
pub fn scoring_model() -> String {
    settings::get_settings()
        .map(|s| s.scoring_model)
        .unwrap_or_else(|_| "gpt-4o-mini".to_string())
}

// === 翻译 API 配置 ===

/// 获取翻译 API Base URL（从设置中读取）
pub fn translation_api_base_url() -> String {
    settings::get_settings()
        .map(|s| s.translation_api_base_url)
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
}

/// 获取翻译模型（从设置中读取）
pub fn translation_model() -> String {
    settings::get_settings()
        .map(|s| s.translation_model)
        .unwrap_or_else(|_| "gpt-3.5-turbo".to_string())
}

/// 检查翻译是否已配置
pub fn is_translation_configured() -> bool {
    settings::get_settings()
        .map(|s| !s.translation_api_key.is_empty() && !s.translation_api_base_url.is_empty())
        .unwrap_or(false)
}

// === arXiv RSS 配置 ===
pub const ARXIV_CATEGORIES: &[&str] = &["cs.SD", "cs.AI", "cs.LG", "cs.CV"];

pub fn rss_feeds() -> Vec<String> {
    ARXIV_CATEGORIES
        .iter()
        .map(|cat| format!("https://rss.arxiv.org/rss/{}", cat))
        .collect()
}

// === 推荐配置 ===
pub const DIVERSITY_RATIO: f32 = 0.3;
pub const SCORING_BATCH_SIZE: usize = 20;

// === 文章状态 ===
pub mod status {
    pub const UNREAD: i32 = 0;
    pub const CLICKED: i32 = 1;      // 已点击（真正阅读）
    pub const LIKED: i32 = 2;        // 已点赞
    pub const MARKED_READ: i32 = 3;  // 批量标记已读
    pub const DISLIKED: i32 = -1;    // 不喜欢
}

// === 数据库配置 ===
pub fn db_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.zenflow/zenflow.db", home)
}

// === 其他配置 ===
pub const ABSTRACT_MAX_LENGTH: usize = 2000;
pub const CLEAN_OLD_ARTICLES_DAYS: i32 = 30;
