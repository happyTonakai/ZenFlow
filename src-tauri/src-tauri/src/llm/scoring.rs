//! LLM 论文评分模块

use anyhow::Result;

use super::client::LlmClient;
use super::truncate_text;

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

    Ok(parse_scoring_response(&response))
}

/// 解析 LLM 评分响应 JSON（纯函数，无副作用）
pub fn parse_scoring_response(raw: &str) -> Vec<(String, f32)> {
    // 清理 markdown 代码块标记
    let content = raw
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 解析 JSON
    let parsed: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("评分结果解析失败: {}, 原始内容: {}", e, content);
            return Vec::new();
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

    scores
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let input = r#"[{"id": "2401.01234", "score": 0.85}, {"id": "2401.05678", "score": 0.42}]"#;
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0].0, "2401.01234");
        assert!((scores[0].1 - 0.85).abs() < 0.01);
        assert_eq!(scores[1].0, "2401.05678");
        assert!((scores[1].1 - 0.42).abs() < 0.01);
    }

    #[test]
    fn test_parse_markdown_fenced_json() {
        let input = "```json\n[{\"id\": \"abc\", \"score\": 0.9}]\n```";
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].0, "abc");
    }

    #[test]
    fn test_parse_bare_fenced_json() {
        let input = "```\n[{\"id\": \"abc\", \"score\": 0.5}]\n```";
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 1);
    }

    #[test]
    fn test_parse_malformed_json_returns_empty() {
        let input = "this is not json at all";
        let scores = parse_scoring_response(input);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_parse_empty_input() {
        let scores = parse_scoring_response("");
        assert!(scores.is_empty());
    }

    #[test]
    fn test_parse_empty_array() {
        let scores = parse_scoring_response("[]");
        assert!(scores.is_empty());
    }

    #[test]
    fn test_parse_missing_id_field_skipped() {
        let input = r#"[{"score": 0.5}, {"id": "good", "score": 0.8}]"#;
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].0, "good");
    }

    #[test]
    fn test_parse_missing_score_field_skipped() {
        let input = r#"[{"id": "no_score"}, {"id": "has_score", "score": 0.7}]"#;
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].0, "has_score");
    }

    #[test]
    fn test_parse_score_clamped_to_range() {
        let input = r#"[{"id": "high", "score": 1.5}, {"id": "low", "score": -0.3}]"#;
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 2);
        assert!((scores[0].1 - 1.0).abs() < 0.01);
        assert!((scores[1].1 - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let input = r#"[{"id": "x", "score": 0.6, "reason": "interesting", "extra": 42}]"#;
        let scores = parse_scoring_response(input);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].0, "x");
    }

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_text_long() {
        let result = truncate_text("hello world", 5);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_truncate_text_unicode() {
        let result = truncate_text("你好世界测试", 4);
        assert_eq!(result, "你好世界...");
    }
}
