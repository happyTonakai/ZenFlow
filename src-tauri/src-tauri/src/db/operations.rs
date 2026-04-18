//! 数据库 CRUD 操作

use anyhow::Result;
use rand::seq::IteratorRandom;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::pool::get_db;
use crate::config;

/// 文章数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub link: String,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub source: String,
    pub status: i32,
    pub score: f32,
    pub translated_title: Option<String>,
    pub translated_abstract: Option<String>,
    pub author: Option<String>,
    pub category: Option<String>,
    pub hf_upvotes: Option<i32>,
    pub ax_upvotes: Option<i32>,
    pub ax_downvotes: Option<i32>,
    pub comment: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "recommendationType")]
    pub recommendation_type: Option<String>,
}

/// 用于插入的文章数据
#[derive(Debug, Clone)]
pub struct NewArticle {
    pub id: String,
    pub title: String,
    pub link: String,
    pub abstract_text: Option<String>,
    pub source: String,
    pub translated_title: Option<String>,
    pub translated_abstract: Option<String>,
    pub author: Option<String>,
    pub category: Option<String>,
}

/// 保存文章
pub fn save_article(article: &NewArticle) -> Result<()> {
    let conn = get_db()?;

    conn.execute(
        "INSERT OR IGNORE INTO articles
         (id, title, link, abstract, source, status, score, translated_title, translated_abstract, author, category)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0.0, ?6, ?7, ?8, ?9)",
        params![
            article.id,
            article.title,
            article.link,
            article.abstract_text,
            article.source,
            article.translated_title,
            article.translated_abstract,
            article.author,
            article.category,
        ],
    )?;

    Ok(())
}

/// 批量保存文章
pub fn save_articles(articles: &[NewArticle]) -> Result<usize> {
    let conn = get_db()?;
    let mut count = 0;

    for article in articles {
        let result = conn.execute(
            "INSERT OR IGNORE INTO articles
             (id, title, link, abstract, source, status, score, translated_title, translated_abstract, author, category)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0.0, ?6, ?7, ?8, ?9)",
            params![
                article.id,
                article.title,
                article.link,
                article.abstract_text,
                article.source,
                article.translated_title,
                article.translated_abstract,
                article.author,
                article.category,
            ],
        );

        if let Ok(rows) = result {
            count += rows;
        }
    }

    Ok(count)
}

/// 更新文章状态
pub fn update_article_status(article_id: &str, status: i32) -> Result<()> {
    let conn = get_db()?;
    conn.execute(
        "UPDATE articles SET status = ?1 WHERE id = ?2",
        params![status, article_id],
    )?;
    Ok(())
}

/// 批量标记未读文章为已读 (status = 3)
pub fn mark_all_unread_as_read() -> Result<usize> {
    let conn = get_db()?;
    let rows = conn.execute(
        "UPDATE articles SET status = ?1 WHERE status = ?2",
        params![config::status::MARKED_READ, config::status::UNREAD],
    )?;
    Ok(rows)
}

fn map_article_row(row: &rusqlite::Row) -> rusqlite::Result<Article> {
    Ok(Article {
        id: row.get(0)?,
        title: row.get(1)?,
        link: row.get(2)?,
        abstract_text: row.get(3)?,
        source: row.get(4)?,
        status: row.get(5)?,
        score: row.get(6)?,
        translated_title: row.get(7)?,
        translated_abstract: row.get(8)?,
        author: row.get(9)?,
        category: row.get(10)?,
        hf_upvotes: row.get(11)?,
        ax_upvotes: row.get(12)?,
        ax_downvotes: row.get(13)?,
        comment: row.get(14)?,
        timestamp: row.get(15)?,
        recommendation_type: None,
    })
}

const ARTICLE_COLUMNS: &str = "id, title, link, abstract, source, status, score,
    translated_title, translated_abstract, author, category, hf_upvotes, ax_upvotes, ax_downvotes, comment, timestamp";

/// 获取文章列表
pub fn get_articles(status: Option<i32>, limit: usize, offset: usize) -> Result<Vec<Article>> {
    let conn = get_db()?;

    let sql = match status {
        Some(_) => format!(
            "SELECT {} FROM articles WHERE status = ?1 ORDER BY score DESC LIMIT ?2 OFFSET ?3",
            ARTICLE_COLUMNS
        ),
        None => format!(
            "SELECT {} FROM articles ORDER BY score DESC LIMIT ?1 OFFSET ?2",
            ARTICLE_COLUMNS
        ),
    };

    let mut stmt = conn.prepare(&sql)?;

    let articles = match status {
        Some(s) => stmt
            .query_map(params![s, limit as i32, offset as i32], map_article_row)?
            .collect::<Result<Vec<_>, _>>()?,
        None => stmt
            .query_map(params![limit as i32, offset as i32], map_article_row)?
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(articles)
}

/// 获取推荐文章（按照 70% 分数排序 + 30% 随机多样性的逻辑）
pub fn get_recommended_articles(daily_papers: usize, diversity_ratio: f32) -> Result<Vec<Article>> {
    let score_based_count = (daily_papers as f32 * (1.0 - diversity_ratio)).ceil() as usize;
    let diversity_count = daily_papers - score_based_count;

    // 获取分数 > 0 的所有文章
    let all_articles = get_articles(None, 1000, 0)?;
    let mut scored_articles: Vec<_> = all_articles.into_iter().filter(|a| a.score > 0.0).collect();

    // 按分数降序排序
    scored_articles.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 分割数组
    let (score_part, remaining_part) =
        scored_articles.split_at(score_based_count.min(scored_articles.len()));

    // 70% 按分数排序
    let score_based: Vec<_> = score_part.to_vec();

    // 30% 从剩余中随机
    let mut rng = rand::thread_rng();
    let mut diversity: Vec<_> = if !remaining_part.is_empty() {
        let count = diversity_count.min(remaining_part.len());
        let indices: Vec<usize> = (0..remaining_part.len()).collect();
        let chosen: Vec<usize> = indices
            .iter()
            .choose_multiple(&mut rng, count)
            .into_iter()
            .cloned()
            .collect();
        let mut chosen_articles: Vec<_> =
            chosen.iter().map(|&i| remaining_part[i].clone()).collect();
        // 随机选择的也按分数降序排列
        chosen_articles.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        chosen_articles
    } else {
        vec![]
    };

    // 合并：分数排序的在前，随机在后
    let mut result = score_based;
    for a in diversity.iter_mut() {
        a.recommendation_type = Some("diversity".to_string());
    }
    result.extend(diversity);

    // 标记分数排序的文章
    for a in result.iter_mut() {
        if a.recommendation_type.is_none() {
            a.recommendation_type = Some("score".to_string());
        }
    }

    Ok(result)
}

/// 更新文章分数
pub fn update_article_score(article_id: &str, score: f32) -> Result<()> {
    let conn = get_db()?;
    conn.execute(
        "UPDATE articles SET score = ?1 WHERE id = ?2",
        params![score, article_id],
    )?;
    Ok(())
}

/// 批量更新文章分数
pub fn update_articles_scores(scores: &[(String, f32)]) -> Result<()> {
    let mut conn = get_db()?;
    let tx = conn.transaction()?;

    for (id, score) in scores {
        tx.execute(
            "UPDATE articles SET score = ?1 WHERE id = ?2",
            params![score, id],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// 反馈文章数据
pub struct FeedbackArticleRow {
    pub id: String,
    pub title: String,
    pub abstract_text: Option<String>,
    pub status: i32,
    pub comment: Option<String>,
}

/// 获取最近有反馈的文章（用于偏好更新）
pub fn get_recent_feedback_articles(since_days: i32) -> Result<Vec<FeedbackArticleRow>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, title, abstract, status, comment FROM articles
         WHERE status IN (?1, ?2, ?3)
         AND timestamp >= datetime('now', ?4)
         ORDER BY timestamp DESC",
    )?;

    let articles = stmt
        .query_map(
            params![
                config::status::CLICKED,
                config::status::LIKED,
                config::status::DISLIKED,
                format!("-{} days", since_days),
            ],
            |row| {
                Ok(FeedbackArticleRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    abstract_text: row.get(2)?,
                    status: row.get(3)?,
                    comment: row.get(4)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(articles)
}

/// 更新文章评论
pub fn update_article_comment(article_id: &str, comment: &str) -> Result<()> {
    let conn = get_db()?;
    let comment_val = if comment.is_empty() { None } else { Some(comment) };
    conn.execute(
        "UPDATE articles SET comment = ?1 WHERE id = ?2",
        params![comment_val, article_id],
    )?;
    Ok(())
}

/// 清理旧文章
pub fn clean_old_articles(days: i32) -> Result<usize> {
    let conn = get_db()?;
    let rows = conn.execute(
        "DELETE FROM articles WHERE timestamp < datetime('now', ?1)",
        params![format!("-{} days", days)],
    )?;
    Ok(rows)
}

/// 获取各状态文章数量
pub fn get_article_count_by_status() -> Result<std::collections::HashMap<i32, i32>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM articles GROUP BY status")?;

    let counts: std::collections::HashMap<i32, i32> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();

    Ok(counts)
}

/// 获取已点赞文章数量
pub fn get_liked_count() -> Result<i32> {
    let conn = get_db()?;
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM articles WHERE status = ?1",
        params![config::status::LIKED],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// 检查是否已完成初始化（通过设置表判断）
pub fn is_initialized() -> Result<bool> {
    crate::settings::AppSettings::is_initialized().map_err(Into::into)
}

/// 检查文章是否已存在
pub fn get_existing_article_ids(
    article_ids: &[String],
) -> Result<std::collections::HashSet<String>> {
    if article_ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }

    let conn = get_db()?;
    let placeholders: Vec<String> = article_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id FROM articles WHERE id IN ({})",
        placeholders.join(",")
    );

    let params: Vec<&dyn rusqlite::ToSql> = article_ids
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();

    let mut stmt = conn.prepare(&sql)?;
    let ids: std::collections::HashSet<String> = stmt
        .query_map(params.as_slice(), |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();

    Ok(ids)
}

/// 获取设置值
pub fn get_setting(key: &str) -> Result<Option<String>> {
    let conn = get_db()?;
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// 设置值
pub fn set_setting(key: &str, value: &str) -> Result<()> {
    let conn = get_db()?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
        params![key, value],
    )?;
    Ok(())
}

/// 批量设置
pub fn set_settings(settings: &[(String, String)]) -> Result<()> {
    let mut conn = get_db()?;
    let tx = conn.transaction()?;

    for (key, value) in settings {
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            params![key, value],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// 获取所有设置
pub fn get_all_settings() -> Result<std::collections::HashMap<String, String>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;

    let settings = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()?;

    Ok(settings)
}

/// 清空所有数据（用于初始化）
pub fn clear_all_data() -> Result<()> {
    let conn = get_db()?;
    conn.execute("DELETE FROM articles", [])?;
    conn.execute("DELETE FROM settings", [])?;
    tracing::info!("已清空数据库所有表");
    Ok(())
}

/// 更新文章的翻译结果
pub fn update_article_translation(
    article_id: &str,
    translated_title: &str,
    translated_abstract: &str,
) -> Result<()> {
    let conn = get_db()?;
    conn.execute(
        "UPDATE articles SET translated_title = ?1, translated_abstract = ?2 WHERE id = ?3",
        params![translated_title, translated_abstract, article_id],
    )?;
    Ok(())
}
