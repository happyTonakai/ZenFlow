//! ZenFlow 配置常量

use std::env;

use crate::settings;

// === API 配置 ===

/// 获取 SiliconFlow API Key（环境变量）
pub fn siliconflow_api_key() -> Option<String> {
    env::var("SILICONFLOW_API_KEY").ok()
}

/// 获取嵌入向量 API Base URL（从设置中读取）
pub fn embedding_api_base_url() -> String {
    settings::get_settings()
        .map(|s| s.embedding_api_base_url)
        .unwrap_or_else(|_| "https://api.siliconflow.cn/v1".to_string())
}

/// 获取嵌入向量模型（从设置中读取）
pub fn embedding_model() -> String {
    settings::get_settings()
        .map(|s| s.embedding_model)
        .unwrap_or_else(|_| "BAAI/bge-m3".to_string())
}

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

// === 聚类配置 ===
pub const MAX_CLUSTERS: usize = 100;
pub const CLUSTER_TRIGGER_THRESHOLD: usize = 5;
pub const DIVERSITY_RATIO: f32 = 0.3;

// === 负向惩罚系数 (α > 1 时对不感兴趣的内容更敏感) ===
pub const NEGATIVE_PENALTY_ALPHA: f32 = 1.5;

// === 反馈权重 ===
pub const WEIGHT_LIKED: f32 = 2.0;   // 点赞权重
pub const WEIGHT_CLICKED: f32 = 1.0; // 点击权重

// === 文章状态 ===
pub mod status {
    pub const UNREAD: i32 = 0;
    pub const CLICKED: i32 = 1;      // 已点击（真正阅读）- 参与正向聚类
    pub const LIKED: i32 = 2;        // 已点赞 - 参与正向聚类，权重更高
    pub const MARKED_READ: i32 = 3;  // 批量标记已读 - 不参与聚类
    pub const DISLIKED: i32 = -1;    // 不喜欢 - 参与负向聚类
}

// === 数据库配置 ===
pub fn db_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.zenflow/zenflow.db", home)
}

// === 其他配置 ===
pub const ABSTRACT_MAX_LENGTH: usize = 2000;
pub const EMBEDDING_TEXT_MAX_LENGTH: usize = 8000;
pub const CLEAN_OLD_ARTICLES_DAYS: i32 = 30;
