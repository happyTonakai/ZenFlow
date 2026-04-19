//! 后台定时任务：每日 RSS 抓取 + 可选的自动推荐

use tokio::time::{interval, Duration};

use crate::algorithm;
use crate::db;
use crate::feed;
use crate::settings;

const CHECK_INTERVAL_SECS: u64 = 3600; // 每小时检查一次

/// 启动后台调度器
pub fn start_scheduler() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(CHECK_INTERVAL_SECS));

        loop {
            ticker.tick().await;

            let today = chrono::Local::now().format("%Y-%m-%d").to_string();

            // 1. 检查是否需要 RSS 抓取
            let needs_fetch = match db::get_setting("last_fetch_date") {
                Ok(Some(last_fetch)) => last_fetch != today,
                Ok(None) => true, // 首次运行
                Err(e) => {
                    tracing::error!("定时任务：读取 last_fetch_date 失败: {}", e);
                    false
                }
            };

            if needs_fetch {
                tracing::info!("定时任务：开始抓取 RSS...");
                match do_fetch_articles().await {
                    Ok(count) => {
                        tracing::info!("定时任务：抓取了 {} 篇新文章", count);
                        let _ = db::set_setting("last_fetch_date", &today);
                    }
                    Err(e) => {
                        tracing::error!("定时任务：抓取失败: {}", e);
                    }
                }
            }

            // 2. 检查是否需要自动推荐
            let auto_refresh = settings::get_settings()
                .map(|s| s.auto_refresh_recommendations)
                .unwrap_or(false);

            if auto_refresh && db::is_initialized().unwrap_or(false) {
                let needs_recommend = match db::get_setting("last_recommend_date") {
                    Ok(Some(last_recommend)) => last_recommend != today,
                    Ok(None) => true,
                    Err(_) => false,
                };

                if needs_recommend {
                    tracing::info!("定时任务：开始自动生成推荐...");
                    match do_generate_daily_recommendations(&today).await {
                        Ok(count) => {
                            tracing::info!("定时任务：生成了 {} 条每日推荐", count);
                            let _ = db::set_setting("last_recommend_date", &today);
                        }
                        Err(e) => {
                            tracing::error!("定时任务：推荐生成失败: {}", e);
                        }
                    }
                }
            }
        }
    });
}

/// 内部抓取 RSS
async fn do_fetch_articles() -> anyhow::Result<usize> {
    let test_file = std::path::Path::new("/Users/hanzerui/joyspace/ZenFlow/test_rss.xml");

    let articles = if test_file.exists() {
        feed::FeedFetcher::fetch_from_local_file(test_file.to_str().unwrap())?
    } else {
        let fetcher = feed::FeedFetcher::new()?;
        fetcher.fetch_all().await?
    };

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

    db::save_articles(&new_articles).map_err(Into::into)
}

/// 内部生成每日推荐
async fn do_generate_daily_recommendations(date: &str) -> anyhow::Result<usize> {
    // 1. 更新偏好
    let _ = algorithm::update_user_preferences().await;

    // 2. 评分所有未读文章
    algorithm::score_all_unread_articles().await?;

    // 3. 标记今日批次
    let s = settings::get_settings().unwrap_or_default();
    let count = db::tag_daily_recommendations(date, s.daily_papers, s.diversity_ratio)?;

    Ok(count)
}
