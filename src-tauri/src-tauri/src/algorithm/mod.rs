//! 算法模块 - LLM 驱动的推荐

mod score;

pub use score::{score_all_unread_articles, update_user_preferences};
