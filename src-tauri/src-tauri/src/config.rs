//! ZenFlow 配置常量

use std::env;

// === API 配置 ===
pub fn siliconflow_api_key() -> Option<String> {
    env::var("SILICONFLOW_API_KEY").ok()
}

pub const SILICONFLOW_API_URL: &str = "https://api.siliconflow.cn/v1/embeddings";
pub const EMBEDDING_MODEL: &str = "BAAI/bge-m3";

// === arXiv RSS 配置 ===
pub const ARXIV_CATEGORIES: &[&str] = &["cs.SD", "cs.AI", "cs.LG", "cs.CV"];

pub fn rss_feeds() -> Vec<String> {
    ARXIV_CATEGORIES
        .iter()
        .map(|cat| format!("https://rss.arxiv.org/rss/{}", cat))
        .collect()
}

// === 聚类配置 ===
pub const MAX_CLUSTERS: usize = 10;
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
