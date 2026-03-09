//! 文章相关的 Tauri Commands

use serde::Serialize;

use crate::algorithm;
use crate::db;
use crate::embedding;
use crate::feed;
use crate::settings;

/// 抓取新文章（从本地测试文件）
#[tauri::command]
pub async fn fetch_articles() -> Result<usize, String> {
    // 开发模式：从本地测试文件读取
    let test_file = std::path::Path::new("/Users/hanzerui/joyspace/ZenFlow/test_rss.xml");
    
    let articles = if test_file.exists() {
        feed::FeedFetcher::fetch_from_local_file(test_file.to_str().unwrap())
            .map_err(|e| format!("读取本地文件失败: {}", e))?
    } else {
        // 生产模式：从网络抓取
        let fetcher = feed::FeedFetcher::new()
            .map_err(|e| format!("初始化失败: {}", e))?;
        fetcher.fetch_all().await
            .map_err(|e| format!("抓取失败: {}", e))?
    };
    
    // 转换为数据库格式并保存
    let new_articles: Vec<db::NewArticle> = articles
        .into_iter()
        .map(|a| db::NewArticle {
            id: a.id,
            title: a.title,
            link: a.link,
            abstract_text: a.abstract_text,
            source: a.source,
            vector: None,
            translated_title: None,
            translated_abstract: None,
            author: a.author,
            category: a.category,
        })
        .collect();
    
    db::save_articles(&new_articles)
        .map_err(|e| format!("保存失败: {}", e))
}

/// 为新文章生成向量（只处理还没有向量的文章）
#[tauri::command]
pub async fn generate_embeddings(limit: usize) -> Result<usize, String> {
    // 先获取已经有向量的文章 ID
    let existing_vectors = db::get_vectors_by_statuses(&[
        crate::config::status::UNREAD,
        crate::config::status::CLICKED,
        crate::config::status::LIKED,
        crate::config::status::DISLIKED,
        crate::config::status::MARKED_READ,
    ]).map_err(|e| format!("获取向量数据失败: {}", e))?;
    
    let has_vector_ids: std::collections::HashSet<String> = existing_vectors
        .into_iter()
        .map(|v| v.id)
        .collect();
    
    // 获取所有文章，过滤掉已有向量的
    let all_articles = db::get_articles(None, limit, 0)
        .map_err(|e| format!("获取文章失败: {}", e))?;
    
    let articles: Vec<_> = all_articles
        .into_iter()
        .filter(|a| !has_vector_ids.contains(&a.id))
        .collect();
    
    tracing::info!("需要生成向量的文章数量: {}", articles.len());
    
    let client = embedding::EmbeddingClient::new();
    
    if !client.is_available() {
        return Err("SILICONFLOW_API_KEY 未设置".to_string());
    }
    
    let mut count = 0;
    for article in articles {
        let text = match &article.abstract_text {
            Some(a) => format!("{} {}", article.title, a),
            None => article.title.clone(),
        };
        
        match client.embed(&text).await {
            Ok(vector) => {
                // 保存向量到数据库
                if let Err(e) = db::save_article_vector(&article.id, &vector) {
                    tracing::error!("保存向量失败 {}: {}", article.id, e);
                } else {
                    tracing::info!("✅ 已为文章 {} 生成并保存向量", article.id);
                    count += 1;
                }
            }
            Err(e) => {
                tracing::error!("生成向量失败 {}: {}", article.id, e);
            }
        }
    }
    
    Ok(count)
}

/// 获取文章列表
#[tauri::command]
pub fn get_articles(
    status: Option<i32>,
    limit: usize,
    offset: usize,
) -> Result<Vec<db::Article>, String> {
    db::get_articles(status, limit, offset)
        .map_err(|e| format!("获取失败: {}", e))
}

/// 获取推荐文章列表（按照 70% 分数排序 + 30% 随机多样性的逻辑）
#[tauri::command]
pub fn get_recommended_articles() -> Result<Vec<db::Article>, String> {
    let settings = settings::get_settings().unwrap_or_default();
    let daily_papers = settings.daily_papers;
    let diversity_ratio = settings.diversity_ratio;
    
    db::get_recommended_articles(daily_papers, diversity_ratio)
        .map_err(|e| format!("获取推荐失败: {}", e))
}

/// 更新文章状态
#[tauri::command]
pub async fn update_status(article_id: String, status: i32) -> Result<(), String> {
    db::update_article_status(&article_id, status)
        .map_err(|e| format!("更新失败: {}", e))?;
    
    tracing::info!("📝 文章 {} 状态更新为 {}", article_id, status);
    
    // 如果有足够的反馈，触发聚类更新
    if let Ok(true) = db::is_initialized() {
        tracing::info!("🎯 触发聚类更新...");
        if let Err(e) = algorithm::update_clusters() {
            tracing::error!("更新聚类失败: {}", e);
        }
    }
    
    Ok(())
}

/// 批量标记未读为已读
#[tauri::command]
pub fn mark_all_read() -> Result<usize, String> {
    db::mark_all_unread_as_read()
        .map_err(|e| format!("操作失败: {}", e))
}

/// 统计结果
#[derive(Serialize)]
pub struct RefreshResult {
    pub pos_count: usize,
    pub neg_count: usize,
    pub pos_clusters: usize,
    pub neg_clusters: usize,
    pub scores_updated: usize,
}

/// 更新聚类并重新计算分数
#[tauri::command]
pub async fn refresh_recommendations() -> Result<RefreshResult, String> {
    let cluster_result = algorithm::update_clusters()
        .map_err(|e| format!("聚类更新失败: {}", e))?;
    
    let scores_updated = algorithm::recalculate_all_scores()
        .map_err(|e| format!("分数计算失败: {}", e))?;
    
    Ok(RefreshResult {
        pos_count: cluster_result.pos_count,
        neg_count: cluster_result.neg_count,
        pos_clusters: cluster_result.pos_centroids.len(),
        neg_clusters: cluster_result.neg_centroids.len(),
        scores_updated,
    })
}

/// 统计数据
#[derive(Serialize)]
pub struct Stats {
    pub unread: i32,
    pub clicked: i32,
    pub liked: i32,
    pub marked_read: i32,
    pub disliked: i32,
    pub initialized: bool,
}

/// 获取统计数据
#[tauri::command]
pub fn get_stats() -> Result<Stats, String> {
    let counts = db::get_article_count_by_status()
        .map_err(|e| format!("获取统计失败: {}", e))?;
    
    let initialized = db::is_initialized().unwrap_or(false);
    
    Ok(Stats {
        unread: counts.get(&0).copied().unwrap_or(0),
        clicked: counts.get(&1).copied().unwrap_or(0),
        liked: counts.get(&2).copied().unwrap_or(0),
        marked_read: counts.get(&3).copied().unwrap_or(0),
        disliked: counts.get(&-1).copied().unwrap_or(0),
        initialized,
    })
}

/// 检查是否已初始化
#[tauri::command]
pub fn is_initialized() -> Result<bool, String> {
    db::is_initialized()
        .map_err(|e| format!("检查失败: {}", e))
}

/// 清理旧文章
#[tauri::command]
pub fn clean_old_articles(days: i32) -> Result<usize, String> {
    db::clean_old_articles(days)
        .map_err(|e| format!("清理失败: {}", e))
}