//! 文章相关的 Tauri Commands

use serde::Serialize;

use crate::algorithm;
use crate::db;
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
            translated_title: None,
            translated_abstract: None,
            author: a.author,
            category: a.category,
        })
        .collect();

    db::save_articles(&new_articles)
        .map_err(|e| format!("保存失败: {}", e))
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

    tracing::info!("文章 {} 状态更新为 {}", article_id, status);

    // 状态变更后，异步更新偏好（fire-and-forget）
    if let Ok(true) = db::is_initialized() {
        tokio::spawn(async {
            if let Err(e) = algorithm::update_user_preferences().await {
                tracing::error!("更新偏好失败: {}", e);
            }
        });
    }

    Ok(())
}

/// 为文章添加评论
#[tauri::command]
pub async fn add_comment(article_id: String, comment: String) -> Result<(), String> {
    db::update_article_comment(&article_id, &comment)
        .map_err(|e| format!("保存评论失败: {}", e))?;

    tracing::info!("文章 {} 添加评论", article_id);

    // 有评论意味着有更丰富的反馈，异步更新偏好
    if let Ok(true) = db::is_initialized() {
        tokio::spawn(async {
            if let Err(e) = algorithm::update_user_preferences().await {
                tracing::error!("更新偏好失败: {}", e);
            }
        });
    }

    Ok(())
}

/// 批量标记未读为已读
#[tauri::command]
pub fn mark_all_read() -> Result<usize, String> {
    db::mark_all_unread_as_read()
        .map_err(|e| format!("操作失败: {}", e))
}

/// 刷新推荐结果
#[derive(Serialize)]
pub struct RefreshResult {
    pub preferences_updated: bool,
    pub scores_updated: usize,
}

/// 更新偏好并重新计算分数
#[tauri::command]
pub async fn refresh_recommendations() -> Result<RefreshResult, String> {
    let mut result = RefreshResult {
        preferences_updated: false,
        scores_updated: 0,
    };

    // 更新用户偏好
    match algorithm::update_user_preferences().await {
        Ok(()) => {
            result.preferences_updated = true;
        }
        Err(e) => {
            tracing::warn!("偏好更新失败: {}", e);
        }
    }

    // 重新评分
    match algorithm::score_all_unread_articles().await {
        Ok(count) => {
            result.scores_updated = count;
        }
        Err(e) => {
            return Err(format!("评分失败: {}", e));
        }
    }

    Ok(result)
}

/// 手动触发偏好更新
#[tauri::command]
pub async fn update_preferences() -> Result<(), String> {
    algorithm::update_user_preferences()
        .await
        .map_err(|e| format!("偏好更新失败: {}", e))
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
