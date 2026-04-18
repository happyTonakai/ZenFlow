//! LLM 驱动的推荐分数计算

use anyhow::Result;

use crate::config;
use crate::db;
use crate::llm;
use crate::llm::preferences::{FeedbackArticle, read_preferences, write_preferences};
use crate::llm::scoring::ArticleInfo;
use crate::settings;

/// 对所有未读文章进行 LLM 评分
pub async fn score_all_unread_articles() -> Result<usize> {
    let preferences = read_preferences()?;
    if preferences.is_empty() {
        tracing::warn!("偏好文件为空，跳过评分");
        return Ok(0);
    }

    // 获取所有未读文章
    let unread_articles = db::get_articles(Some(config::status::UNREAD), 1000, 0)?;
    if unread_articles.is_empty() {
        return Ok(0);
    }

    // 构建评分客户端
    let s = settings::get_settings().unwrap_or_default();
    let client = llm::LlmClient::new(&s.scoring_api_base_url, &s.scoring_api_key, &s.scoring_model);
    if !client.is_available() {
        return Err(anyhow::anyhow!("评分 API 未配置"));
    }

    // 转换为评分所需的格式
    let articles: Vec<ArticleInfo> = unread_articles
        .iter()
        .map(|a| ArticleInfo {
            id: a.id.clone(),
            title: a.title.clone(),
            abstract_text: a.abstract_text.clone().unwrap_or_default(),
        })
        .collect();

    tracing::info!("开始为 {} 篇文章进行 LLM 评分...", articles.len());

    let scores = llm::scoring::score_articles_batched(
        &client,
        &preferences,
        &articles,
        config::SCORING_BATCH_SIZE,
    )
    .await?;

    if !scores.is_empty() {
        db::update_articles_scores(&scores)?;
        tracing::info!("已更新 {} 篇文章的分数", scores.len());
    }

    Ok(scores.len())
}

/// 更新用户偏好（根据最近的反馈）
pub async fn update_user_preferences() -> Result<()> {
    // 获取最近 30 天的反馈文章
    let feedback = db::get_recent_feedback_articles(30)?;
    if feedback.is_empty() {
        tracing::info!("没有新的反馈，跳过偏好更新");
        return Ok(());
    }

    let s = settings::get_settings().unwrap_or_default();
    let client = llm::LlmClient::new(&s.scoring_api_base_url, &s.scoring_api_key, &s.scoring_model);
    if !client.is_available() {
        return Err(anyhow::anyhow!("评分 API 未配置"));
    }

    let current_preferences = read_preferences()?;

    let feedback_articles: Vec<FeedbackArticle> = feedback
        .into_iter()
        .map(|row| FeedbackArticle {
            title: row.title,
            abstract_text: row.abstract_text.unwrap_or_default(),
            status: row.status,
            comment: row.comment,
        })
        .collect();

    tracing::info!("根据 {} 条反馈更新用户偏好...", feedback_articles.len());

    let updated = llm::preferences::update_preferences(
        &client,
        &current_preferences,
        &feedback_articles,
    )
    .await?;

    write_preferences(&updated)?;
    tracing::info!("用户偏好已更新");

    Ok(())
}
