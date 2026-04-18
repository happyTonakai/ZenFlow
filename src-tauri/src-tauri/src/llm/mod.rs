//! LLM 客户端模块 - 用于评分、偏好更新和翻译

pub mod client;
pub mod scoring;
pub mod preferences;

pub use client::LlmClient;

/// 截断文本到指定字符数
pub(crate) fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}
