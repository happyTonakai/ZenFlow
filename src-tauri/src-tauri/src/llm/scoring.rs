//! LLM 论文评分模块

use anyhow::Result;

use super::client::LlmClient;

/// 单篇论文信息（用于评分）
pub struct ArticleInfo {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
}

const SCORING_SYSTEM_PROMPT: &str = r#"你是一个学术论文推荐系统的评分助手。根据用户的兴趣偏好，为论文打分。

评分规则：
- 分数范围 0.0 到 1.0
- 1.0 表示与用户兴趣完全匹配
- 0.0 表示与用户兴趣完全不相关
- 考虑论文主题、方法和研究方向与用户偏好的匹配度

请严格按照以下 JSON 数组格式返回，不要返回任何其他内容：
[{"id": "论文ID", "score": 0.85}, ...]

注意：
1. 只返回 JSON 数组，不要有解释或注释
2. 每篇论文必须有一个分数
3. id 必须与输入的论文 ID 完全一致"#;

/// 对一批论文进行 LLM 评分
pub async fn score_articles(
    client: &LlmClient,
    preferences: &str,
    articles: &[ArticleInfo],
) -> Result<Vec<(String, f32)>> {
    if articles.is_empty() {
        return Ok(Vec::new());
    }

    // 构建用户消息
    let mut user_content = format!("## 用户兴趣偏好\n{}\n\n## 待评分论文\n", preferences);
    for article in articles {
        user_content.push_str(&format!(
            "ID: {}\n标题: {}\n摘要: {}\n---\n",
            article.id,
            article.title,
            truncate_text(&article.abstract_text, 500),
        ));
    }

    let response = client
        .chat_completion(SCORING_SYSTEM_PROMPT, &user_content, 0.3, 4000)
        .await?;

    // 清理 markdown 代码块标记
    let content = response
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 解析 JSON
    let parsed: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("评分结果解析失败: {}, 原始内容: {}", e, content);
            return Ok(Vec::new());
        }
    };

    let mut scores = Vec::new();
    for item in parsed {
        if let (Some(id), Some(score)) = (
            item.get("id").and_then(|v| v.as_str()),
            item.get("score").and_then(|v| v.as_f64()),
        ) {
            scores.push((id.to_string(), score.clamp(0.0, 1.0) as f32));
        }
    }

    Ok(scores)
}

/// 分批评分（每批 batch_size 篇）
pub async fn score_articles_batched(
    client: &LlmClient,
    preferences: &str,
    articles: &[ArticleInfo],
    batch_size: usize,
) -> Result<Vec<(String, f32)>> {
    let mut all_scores = Vec::new();

    for chunk in articles.chunks(batch_size) {
        match score_articles(client, preferences, chunk).await {
            Ok(scores) => {
                all_scores.extend(scores);
            }
            Err(e) => {
                tracing::error!("批次评分失败: {}", e);
                // 跳过失败的批次，继续处理
            }
        }
    }

    Ok(all_scores)
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}
