//! 数据库 CRUD 操作

use anyhow::Result;
use ndarray::Array1;
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
    pub translated_abstract: Option<String>,
    pub hf_upvotes: Option<i32>,
    pub ax_upvotes: Option<i32>,
    pub ax_downvotes: Option<i32>,
    pub timestamp: Option<String>,
}

/// 用于插入的文章数据
#[derive(Debug, Clone)]
pub struct NewArticle {
    pub id: String,
    pub title: String,
    pub link: String,
    pub abstract_text: Option<String>,
    pub source: String,
    pub vector: Option<Vec<f32>>,
    pub translated_abstract: Option<String>,
}

/// 向量数据（用于聚类）
pub struct VectorData {
    pub id: String,
    pub vector: Vec<f32>,
    pub status: i32,
}

fn vector_to_blob(v: &[f32]) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<f32>(),
        )
        .to_vec()
    }
}

fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    let len = blob.len() / std::mem::size_of::<f32>();
    unsafe {
        let ptr = blob.as_ptr() as *const f32;
        std::slice::from_raw_parts(ptr, len).to_vec()
    }
}

/// 保存文章
pub fn save_article(article: &NewArticle) -> Result<()> {
    let conn = get_db()?;
    let vector_blob = article.vector.as_ref().map(|v| vector_to_blob(v));
    
    conn.execute(
        "INSERT OR IGNORE INTO articles 
         (id, title, link, abstract, source, vector, status, score, translated_abstract)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0.0, ?7)",
        params![
            article.id,
            article.title,
            article.link,
            article.abstract_text,
            article.source,
            vector_blob,
            article.translated_abstract,
        ],
    )?;
    
    Ok(())
}

/// 批量保存文章
pub fn save_articles(articles: &[NewArticle]) -> Result<usize> {
    let conn = get_db()?;
    let mut count = 0;
    
    for article in articles {
        let vector_blob = article.vector.as_ref().map(|v| vector_to_blob(v));
        
        let result = conn.execute(
            "INSERT OR IGNORE INTO articles 
             (id, title, link, abstract, source, vector, status, score, translated_abstract)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0.0, ?7)",
            params![
                article.id,
                article.title,
                article.link,
                article.abstract_text,
                article.source,
                vector_blob,
                article.translated_abstract,
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

/// 获取文章列表
pub fn get_articles(status: Option<i32>, limit: usize, offset: usize) -> Result<Vec<Article>> {
    let conn = get_db()?;
    
    let sql = match status {
        Some(_) => "SELECT id, title, link, abstract, source, status, score, 
                           translated_abstract, hf_upvotes, ax_upvotes, ax_downvotes, timestamp
                    FROM articles WHERE status = ?1 
                    ORDER BY score DESC LIMIT ?2 OFFSET ?3",
        None => "SELECT id, title, link, abstract, source, status, score, 
                        translated_abstract, hf_upvotes, ax_upvotes, ax_downvotes, timestamp
                 FROM articles ORDER BY score DESC LIMIT ?1 OFFSET ?2",
    };
    
    let mut stmt = conn.prepare(sql)?;
    
    let articles = match status {
        Some(s) => {
            stmt.query_map(params![s, limit as i32, offset as i32], |row| {
                Ok(Article {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    link: row.get(2)?,
                    abstract_text: row.get(3)?,
                    source: row.get(4)?,
                    status: row.get(5)?,
                    score: row.get(6)?,
                    translated_abstract: row.get(7)?,
                    hf_upvotes: row.get(8)?,
                    ax_upvotes: row.get(9)?,
                    ax_downvotes: row.get(10)?,
                    timestamp: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        }
        None => {
            stmt.query_map(params![limit as i32, offset as i32], |row| {
                Ok(Article {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    link: row.get(2)?,
                    abstract_text: row.get(3)?,
                    source: row.get(4)?,
                    status: row.get(5)?,
                    score: row.get(6)?,
                    translated_abstract: row.get(7)?,
                    hf_upvotes: row.get(8)?,
                    ax_upvotes: row.get(9)?,
                    ax_downvotes: row.get(10)?,
                    timestamp: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        }
    };
    
    Ok(articles)
}

/// 获取文章向量（用于聚类）
pub fn get_vectors_by_statuses(statuses: &[i32]) -> Result<Vec<VectorData>> {
    let conn = get_db()?;
    
    let placeholders: Vec<String> = statuses.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, vector, status FROM articles WHERE status IN ({}) AND vector IS NOT NULL",
        placeholders.join(",")
    );
    
    let params: Vec<&dyn rusqlite::ToSql> = statuses.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    
    let mut stmt = conn.prepare(&sql)?;
    let vectors = stmt
        .query_map(params.as_slice(), |row| {
            let id: String = row.get(0)?;
            let vector_blob: Vec<u8> = row.get(1)?;
            let status: i32 = row.get(2)?;
            Ok(VectorData { 
                id, 
                vector: blob_to_vector(&vector_blob), 
                status 
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(vectors)
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

/// 保存聚类中心
pub fn save_clusters(cluster_type: &str, centroids: &[Array1<f32>]) -> Result<()> {
    let mut conn = get_db()?;
    let tx = conn.transaction()?;
    
    tx.execute("DELETE FROM clusters WHERE type = ?1", params![cluster_type])?;
    
    for centroid in centroids {
        let bytes = vector_to_blob(centroid.as_slice().unwrap());
        tx.execute(
            "INSERT INTO clusters (type, centroid) VALUES (?1, ?2)",
            params![cluster_type, bytes],
        )?;
    }
    
    tx.commit()?;
    Ok(())
}

/// 加载聚类中心
pub fn load_clusters(cluster_type: &str) -> Result<Vec<Array1<f32>>> {
    let conn = get_db()?;
    
    let mut stmt = conn.prepare("SELECT centroid FROM clusters WHERE type = ?1")?;
    let centroids = stmt
        .query_map(params![cluster_type], |row| {
            let blob: Vec<u8> = row.get(0)?;
            Ok(Array1::from_vec(blob_to_vector(&blob)))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(centroids)
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

/// 检查是否已完成初始化（至少有 CLUSTER_TRIGGER_THRESHOLD 篇偏好文章）
pub fn is_initialized() -> Result<bool> {
    let count = get_liked_count()?;
    Ok(count >= config::CLUSTER_TRIGGER_THRESHOLD as i32)
}

/// 检查文章是否已存在
pub fn get_existing_article_ids(article_ids: &[String]) -> Result<std::collections::HashSet<String>> {
    if article_ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    
    let conn = get_db()?;
    let placeholders: Vec<String> = article_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("SELECT id FROM articles WHERE id IN ({})", placeholders.join(","));
    
    let params: Vec<&dyn rusqlite::ToSql> = article_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    
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

/// 保存文章向量
pub fn save_article_vector(article_id: &str, vector: &[f32]) -> Result<()> {
    let conn = get_db()?;
    let vector_blob = vector_to_blob(vector);
    
    conn.execute(
        "UPDATE articles SET vector = ?1 WHERE id = ?2",
        params![vector_blob, article_id],
    )?;
    
    Ok(())
}