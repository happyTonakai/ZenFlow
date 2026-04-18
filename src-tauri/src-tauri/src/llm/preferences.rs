//! 用户偏好管理模块

use anyhow::Result;
use std::path::PathBuf;

use super::client::LlmClient;
use super::truncate_text;

/// 获取偏好文件路径 (~/.zenflow/preferences.md)
pub fn preferences_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.zenflow/preferences.md", home))
}

/// 读取偏好文件
pub fn read_preferences() -> Result<String> {
    let path = preferences_path();
    if path.exists() {
        Ok(std::fs::read_to_string(path)?)
    } else {
        Ok(String::new())
    }
}

/// 写入偏好文件
pub fn write_preferences(content: &str) -> Result<()> {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

/// 用于偏好更新的反馈文章
pub struct FeedbackArticle {
    pub title: String,
    pub abstract_text: String,
    pub status: i32,  // 1=clicked, 2=liked, -1=disliked
    pub comment: Option<String>,  // 用户对这篇文章的评论
}

const UPDATE_PREFERENCES_SYSTEM_PROMPT: &str = r#"你是一个用户偏好分析助手。根据用户对学术论文的反馈行为，更新和完善用户的兴趣偏好描述。

偏好描述应包含以下方面（使用 Markdown 格式）：
## 感兴趣的主题
- 列出用户感兴趣的研究主题和方向

## 不感兴趣的主题
- 列出用户明确不感兴趣的主题

## 偏好的研究方法/技术
- 列出用户偏好的技术路线、方法论

## 备注
- 其他观察到的偏好模式

规则：
1. 直接输出更新后的完整偏好描述，不要有任何解释
2. 保持简洁，每个方面最多列出 10 条
3. 如果用户点赞(liked)了某论文，说明非常感兴趣
4. 如果用户点击(clicked)了某论文，说明比较感兴趣
5. 如果用户点踩(disliked)了某论文，说明不感兴趣
6. 用户的评论是最重要的信号，它直接表达了用户的想法和偏好原因，请重点参考
7. 综合考虑新的反馈和已有偏好，做出合理更新"#;

/// 根据用户反馈更新偏好
pub async fn update_preferences(
    client: &LlmClient,
    current_preferences: &str,
    feedback_articles: &[FeedbackArticle],
) -> Result<String> {
    if feedback_articles.is_empty() {
        return Ok(current_preferences.to_string());
    }

    let mut user_content = String::new();

    if !current_preferences.is_empty() {
        user_content.push_str(&format!(
            "## 当前用户偏好\n{}\n\n",
            current_preferences
        ));
    }

    user_content.push_str("## 新的用户反馈\n");
    for article in feedback_articles {
        let action = match article.status {
            2 => "点赞(非常感兴趣)",
            1 => "点击(比较感兴趣)",
            -1 => "点踩(不感兴趣)",
            _ => "未知",
        };
        user_content.push_str(&format!(
            "- [{}] {}\n  摘要: {}\n",
            action,
            article.title,
            truncate_text(&article.abstract_text, 300),
        ));
        if let Some(ref comment) = article.comment {
            if !comment.is_empty() {
                user_content.push_str(&format!("  用户评论: {}\n", comment));
            }
        }
    }

    let response = client
        .chat_completion(UPDATE_PREFERENCES_SYSTEM_PROMPT, &user_content, 0.3, 4000)
        .await?;

    Ok(response.trim().to_string())
}

const INITIAL_PREFERENCES_SYSTEM_PROMPT: &str = r#"你是一个用户偏好分析助手。根据用户选择的喜欢的论文，生成初始的用户兴趣偏好描述。

偏好描述应使用 Markdown 格式，包含以下方面：
## 感兴趣的主题
- 列出用户感兴趣的研究主题和方向

## 偏好的研究方法/技术
- 列出用户偏好的技术路线、方法论

## 备注
- 其他观察到的偏好模式

规则：
1. 直接输出偏好描述，不要有任何解释
2. 从论文的主题、方法、领域归纳出用户的兴趣模式
3. 保持简洁，每个方面最多列出 10 条"#;

/// 从初始喜欢的论文生成偏好
pub async fn generate_initial_preferences(
    client: &LlmClient,
    favorite_papers: &[FeedbackArticle],
) -> Result<String> {
    if favorite_papers.is_empty() {
        return Ok(String::new());
    }

    let mut user_content = String::from("## 用户喜欢的论文\n");
    for paper in favorite_papers {
        user_content.push_str(&format!(
            "- {}\n  摘要: {}\n",
            paper.title,
            truncate_text(&paper.abstract_text, 300),
        ));
    }

    let response = client
        .chat_completion(INITIAL_PREFERENCES_SYSTEM_PROMPT, &user_content, 0.3, 4000)
        .await?;

    Ok(response.trim().to_string())
}

