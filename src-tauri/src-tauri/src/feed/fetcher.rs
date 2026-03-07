//! RSS/Atom Feed 抓取和解析

use anyhow::Result;
use chrono::{DateTime, Utc};
use md5;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config;

/// 抓取的文章数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedArticle {
    pub id: String,
    pub title: String,
    pub link: String,
    pub abstract_text: Option<String>,
    pub source: String,
    pub published: Option<DateTime<Utc>>,
}

/// RSS 抓取器
pub struct FeedFetcher {
    client: Client,
}

impl FeedFetcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ZenFlow/0.1.0")
            .build()?;
        
        Ok(Self { client })
    }
    
    /// 从本地测试文件读取 RSS
    pub fn fetch_from_local_file(path: &str) -> Result<Vec<FetchedArticle>> {
        let content = std::fs::read_to_string(path)?;
        let articles = parse_arxiv_rss(&content)?;
        tracing::info!("📦 从本地文件 {} 读取了 {} 篇文章", path, articles.len());
        Ok(articles)
    }
    
    /// 抓取所有配置的 RSS 源
    pub async fn fetch_all(&self) -> Result<Vec<FetchedArticle>> {
        let feeds = config::rss_feeds();
        let mut all_articles = Vec::new();
        
        for feed_url in feeds {
            match self.fetch_feed(&feed_url).await {
                Ok(articles) => {
                    tracing::info!("📦 从 {} 获取了 {} 篇文章", feed_url, articles.len());
                    all_articles.extend(articles);
                }
                Err(e) => {
                    tracing::error!("抓取 {} 失败: {}", feed_url, e);
                }
            }
        }
        
        // 去重（基于 id）
        let mut seen = std::collections::HashSet::new();
        all_articles.retain(|a| seen.insert(a.id.clone()));
        
        tracing::info!("📊 总计 {} 篇新文章", all_articles.len());
        Ok(all_articles)
    }
    
    /// 抓取单个 RSS 源
    async fn fetch_feed(&self, url: &str) -> Result<Vec<FetchedArticle>> {
        let response = self.client.get(url).send().await?;
        let body = response.text().await?;
        
        // 使用自定义解析器处理 arXiv RSS
        parse_arxiv_rss(&body)
    }
}

impl Default for FeedFetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create FeedFetcher")
    }
}

/// 解析 arXiv RSS 格式 (RSS 2.0)
fn parse_arxiv_rss(content: &str) -> Result<Vec<FetchedArticle>> {
    let mut articles = Vec::new();
    
    // 分割每个 <item>
    let items: Vec<&str> = content.split("<item>").skip(1).collect();
    
    for item in items {
        let title = extract_xml_content(item, "title");
        let link = extract_xml_content(item, "link");
        let description = extract_xml_content(item, "description");
        
        if title.is_empty() || link.is_empty() {
            continue;
        }
        
        // 提取 arXiv ID
        let id = extract_arxiv_id(&link).unwrap_or_else(|| {
            format!("{:x}", md5::compute(&link))
        });
        
        // 解析摘要
        let abstract_text = if description.contains("Abstract:") {
            description
                .split("Abstract:")
                .nth(1)
                .map(|s| s.trim().chars().take(config::ABSTRACT_MAX_LENGTH).collect())
        } else {
            Some(description.chars().take(config::ABSTRACT_MAX_LENGTH).collect())
        };
        
        // 检查 announce_type (只保留 new 和 cross)
        let announce_type = extract_arxiv_announce_type(item);
        if let Some(ref at) = announce_type {
            if at != "new" && at != "cross" {
                continue;
            }
        }
        
        articles.push(FetchedArticle {
            id,
            title: title.trim().to_string(),
            link: link.trim().to_string(),
            abstract_text,
            source: "arxiv".to_string(),
            published: None,
        });
    }
    
    Ok(articles)
}

/// 提取 XML 标签内容
fn extract_xml_content(content: &str, tag: &str) -> String {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    if let Some(start) = content.find(&open_tag) {
        let rest = &content[start + open_tag.len()..];
        if let Some(end) = rest.find(&close_tag) {
            return rest[..end].to_string();
        }
    }
    String::new()
}

/// 提取 arXiv announce_type
fn extract_arxiv_announce_type(content: &str) -> Option<String> {
    // 查找 <arxiv:announce_type>new</arxiv:announce_type>
    if let Some(start) = content.find("<arxiv:announce_type>") {
        let rest = &content[start + 21..];
        if let Some(end) = rest.find("</arxiv:announce_type>") {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// 从链接中提取 arXiv ID
fn extract_arxiv_id(link: &str) -> Option<String> {
    // 匹配格式: https://arxiv.org/abs/2506.14724 或 /abs/2506.14724v2
    if let Some(pos) = link.find("/abs/") {
        let id_part = &link[pos + 5..];
        // 移除版本后缀 (v1, v2, etc.)
        let id = id_part.split('v').next().unwrap_or(id_part);
        return Some(id.to_string());
    }
    None
}

/// 通过 arXiv ID 获取文章详情
pub async fn fetch_arxiv_by_ids(ids: &[String]) -> Result<Vec<FetchedArticle>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let id_list = ids.join(",");
    let url = format!("https://export.arxiv.org/api/query?id_list={}", id_list);
    
    let response = client.get(&url).send().await?;
    let body = response.text().await?;
    
    // arXiv API 返回 Atom 格式，使用简单解析
    let mut articles = Vec::new();
    let entries: Vec<&str> = body.split("<entry>").skip(1).collect();
    
    for entry in entries {
        let title = extract_xml_content(entry, "title");
        let summary = extract_xml_content(entry, "summary");
        
        // 获取链接和 ID
        let link = if let Some(start) = entry.find("href=\"") {
            let rest = &entry[start + 6..];
            if let Some(end) = rest.find("\"") {
                let l = &rest[..end];
                if l.contains("/abs/") && !l.contains("pdf") {
                    l.to_string()
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        };
        
        let id = extract_arxiv_id(&link).unwrap_or_default();
        if id.is_empty() || title.is_empty() {
            continue;
        }
        
        articles.push(FetchedArticle {
            id,
            title: title.trim().replace('\n', " "),
            link,
            abstract_text: Some(summary.chars().take(config::ABSTRACT_MAX_LENGTH).collect()),
            source: "arxiv".to_string(),
            published: None,
        });
    }
    
    Ok(articles)
}
