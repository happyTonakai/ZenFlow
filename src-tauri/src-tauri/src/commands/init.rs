//! 初始化相关的 Tauri Commands

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::db;
use crate::db::operations::{get_existing_article_ids, save_articles, set_setting, update_article_status};
use crate::embedding::EmbeddingClient;
use crate::feed::{fetch_arxiv_by_id, fetch_arxiv_paper};
use crate::settings::{self, AppSettings};
use crate::algorithm;

/// 解析 arXiv ID 的正则表达式
fn extract_arxiv_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let seen = HashSet::new();
    
    // 匹配格式: 2501.12345, 2501.12345v1, cs/9901001
    let re = Regex::new(r"(\d{4}\.\d{4,5}(?:v\d+)?)|([a-z-]+/\d{7})").unwrap();
    
    for cap in re.captures_iter(text) {
        let id = if let Some(m) = cap.get(1) {
            m.as_str().to_string()
        } else if let Some(m) = cap.get(2) {
            m.as_str().to_string()
        } else {
            continue;
        };
        
        if !seen.contains(&id) {
            ids.push(id);
        }
    }
    
    ids
}

/// 解析 arXiv 链接提取 ID
fn extract_arxiv_id_from_url(url: &str) -> Option<String> {
    // https://arxiv.org/abs/2501.12345
    // https://arxiv.org/pdf/2501.12345.pdf
    // https://ar5iv.org/html/2501.12345
    
    let re = Regex::new(r"arxiv\.org/(?:abs|pdf|format)/([\w\./]+)").ok()?;
    if let Some(cap) = re.captures(url) {
        let id = cap.get(1)?.as_str();
        // 移除 .pdf 后缀
        return Some(id.replace(".pdf", ""));
    }
    
    // ar5iv.org/html/2501.12345
    let re2 = Regex::new(r"ar5iv\.org/html/([\w\.]+)").ok()?;
    if let Some(cap) = re2.captures(url) {
        return Some(cap.get(1)?.as_str().to_string());
    }
    
    None
}

/// 初始化设置请求
#[derive(Debug, Deserialize)]
pub struct InitSettingsRequest {
    pub arxiv_categories: Vec<String>,
    pub favorite_papers: Vec<String>,  // 用户喜欢的论文链接或ID
    pub siliconflow_api_key: String,
    pub pos_clusters: usize,
    pub neg_clusters: usize,
    pub daily_papers: usize,
    pub negative_alpha: f32,
    pub diversity_ratio: f32,
    pub enable_translation: bool,
}

/// 初始化结果
#[derive(Debug, Serialize)]
pub struct InitResult {
    pub settings_saved: bool,
    pub papers_fetched: usize,
    pub papers_embedded: usize,
    pub pos_clusters: usize,
    pub neg_clusters: usize,
    pub errors: Vec<String>,
}

/// 保存初始化设置（不处理论文）
#[tauri::command]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings::update_settings(&settings)
        .map_err(|e| format!("保存设置失败: {}", e))
}

/// 获取当前设置
#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    settings::get_settings()
        .map_err(|e| format!("获取设置失败: {}", e))
}

/// 检查是否需要初始化
#[tauri::command]
pub fn needs_initialization() -> Result<bool, String> {
    settings::AppSettings::is_initialized()
        .map(|initialized| !initialized)
        .map_err(|e| format!("检查失败: {}", e))
}

/// 重置初始化状态
#[tauri::command]
pub fn reset_initialization() -> Result<(), String> {
    settings::AppSettings::reset_initialized()
        .map_err(|e| format!("重置失败: {}", e))
}

/// 解析并获取用户喜欢的论文
#[tauri::command]
pub async fn fetch_favorite_papers(paper_links: Vec<String>) -> Result<usize, String> {
    let client = EmbeddingClient::new();
    if !client.is_available() {
        return Err("API Key 未设置，无法获取论文".to_string());
    }
    
    let mut ids_to_fetch = Vec::new();
    
    for link in paper_links {
        let link = link.trim();
        if link.is_empty() {
            continue;
        }
        
        // 直接是 arXiv ID
        if link.len() < 20 && !link.contains("/") && !link.contains(":") {
            ids_to_fetch.push(link.to_string());
            continue;
        }
        
        // 从 URL 提取 ID
        if let Some(id) = extract_arxiv_id_from_url(link) {
            ids_to_fetch.push(id);
        } else {
            // 尝试直接作为 ID
            let ids = extract_arxiv_ids(link);
            ids_to_fetch.extend(ids);
        }
    }
    
    if ids_to_fetch.is_empty() {
        return Ok(0);
    }
    
    // 去重
    let ids_to_fetch: Vec<String> = ids_to_fetch.into_iter().collect::<HashSet<_>>().into_iter().collect();
    
    // 检查哪些已经存在
    let existing = get_existing_article_ids(&ids_to_fetch)
        .map_err(|e| format!("查询失败: {}", e))?;
    
    let mut new_articles = Vec::new();
    let mut fetched_count = 0;
    
    for id in ids_to_fetch {
        if existing.contains(&id) {
            // 已存在，标记为喜欢
            update_article_status(&id, crate::config::status::LIKED)
                .map_err(|e| format!("更新状态失败 {}: {}", id, e))?;
            fetched_count += 1;
            continue;
        }
        
        // 获取论文信息
        match fetch_arxiv_paper(&id).await {
            Ok(paper) => {
                new_articles.push(db::NewArticle {
                    id: paper.id.clone(),
                    title: paper.title,
                    link: paper.link,
                    abstract_text: paper.abstract_text,
                    source: paper.source,
                    vector: None,
                    translated_abstract: None,
                });
                fetched_count += 1;
            }
            Err(e) => {
                tracing::warn!("获取论文失败 {}: {}", id, e);
            }
        }
    }
    
    // 保存新文章
    if !new_articles.is_empty() {
        save_articles(&new_articles)
            .map_err(|e| format!("保存文章失败: {}", e))?;
    }
    
    Ok(fetched_count)
}

/// 执行完整的初始化流程
#[tauri::command]
pub async fn initialize_app(request: InitSettingsRequest) -> Result<InitResult, String> {
    let mut result = InitResult {
        settings_saved: false,
        papers_fetched: 0,
        papers_embedded: 0,
        pos_clusters: 0,
        neg_clusters: 0,
        errors: Vec::new(),
    };
    
    // 1. 保存设置
    let settings = AppSettings {
        arxiv_categories: request.arxiv_categories.clone(),
        siliconflow_api_key: request.siliconflow_api_key.clone(),
        pos_clusters: request.pos_clusters,
        neg_clusters: request.neg_clusters,
        daily_papers: request.daily_papers,
        negative_alpha: request.negative_alpha,
        diversity_ratio: request.diversity_ratio,
        enable_translation: request.enable_translation,
        translation_model: "Qwen/Qwen2.5-7B-Instruct".to_string(),
    };
    
    if let Err(e) = settings::update_settings(&settings) {
        result.errors.push(format!("保存设置失败: {}", e));
        return Ok(result);
    }
    result.settings_saved = true;
    
    // 保存 RSS 分类设置
    let categories_str = request.arxiv_categories.join(",");
    if let Err(e) = set_setting("arxiv_categories", &categories_str) {
        result.errors.push(format!("保存分类失败: {}", e));
    }
    
    // 2. 获取并处理用户喜欢的论文
    if !request.favorite_papers.is_empty() {
        match fetch_favorite_papers(request.favorite_papers).await {
            Ok(count) => {
                result.papers_fetched = count;
                tracing::info!("✅ 成功获取 {} 篇喜欢的论文", count);
            }
            Err(e) => {
                result.errors.push(format!("获取论文失败: {}", e));
            }
        }
    }
    
    // 3. 为喜欢/点击的论文生成向量
    let client = EmbeddingClient::with_key(&request.siliconflow_api_key);
    if client.is_available() {
        let articles = db::get_vectors_by_statuses(&[
            crate::config::status::LIKED,
            crate::config::status::CLICKED,
        ]).map_err(|e| format!("获取文章失败: {}", e))?;
        
        tracing::info!("为 {} 篇偏好文章生成向量...", articles.len());
        
        for article in articles {
            let text = if let Some(ref abs) = article.abstract_text {
                format!("{} {}", article.title, abs)
            } else {
                article.title.clone()
            };
            
            match client.embed(&text).await {
                Ok(vector) => {
                    // 保存向量到数据库
                    if let Err(e) = db::save_article_vector(&article.id, &vector) {
                        result.errors.push(format!("保存向量失败 {}: {}", article.id, e));
                    } else {
                        result.papers_embedded += 1;
                    }
                }
                Err(e) => {
                    result.errors.push(format!("生成向量失败 {}: {}", article.id, e));
                }
            }
        }
    } else {
        result.errors.push("API Key 无效，无法生成向量".to_string());
    }
    
    // 4. 执行聚类（如果有足够的向量）
    if result.papers_embedded > 0 {
        match algorithm::update_clusters() {
            Ok(cluster_result) => {
                result.pos_clusters = cluster_result.pos_centroids.len();
                result.neg_clusters = cluster_result.neg_centroids.len();
                tracing::info!("✅ 聚类完成: {} 正向, {} 负向", result.pos_clusters, result.neg_clusters);
                
                // 重新计算分数
                if let Err(e) = algorithm::recalculate_all_scores() {
                    result.errors.push(format!("计算分数失败: {}", e));
                }
            }
            Err(e) => {
                result.errors.push(format!("聚类失败: {}", e));
            }
        }
    }
    
    // 5. 标记为已初始化
    if let Err(e) = settings::AppSettings::mark_initialized() {
        result.errors.push(format!("标记初始化失败: {}", e));
    }
    
    Ok(result)
}

/// 获取推荐的 arXiv 分类列表
#[tauri::command]
pub fn get_arxiv_categories() -> Vec<&'static str> {
    vec![
        "cs.AI", "cs.LG", "cs.CL", "cs.CV", "cs.RO", "cs.DB", "cs.DC", 
        "cs.DS", "cs.GT", "cs.HC", "cs.IR", "cs.IT", "cs.MA", "cs.MM",
        "cs.NE", "cs.NI", "cs.OS", "cs.PF", "cs.PL", "cs.SC", "cs.SE",
        "cs.SD", "cs.SY", "eess.AS", "eess.IV", "eess.SP", "eess.SY",
        "math.OC", "stat.ML", "physics.chem-ph", "physics.comp-ph", "q-bio.QM",
    ]
}

/// 翻译文本（使用 SiliconFlow LLM）
#[tauri::command]
pub async fn translate_text(text: String, api_key: Option<String>) -> Result<String, String> {
    let settings = settings::get_settings()
        .map_err(|e| format!("获取设置失败: {}", e))?;
    
    let api_key = api_key.or_else(|| {
        if !settings.siliconflow_api_key.is_empty() {
            Some(settings.siliconflow_api_key)
        } else {
            None
        }
    }).ok_or("API Key 未设置")?;
    
    if !settings.enable_translation {
        return Ok(text);
    }
    
    // 使用 Qwen 模型进行翻译
    let client = reqwest::Client::new();
    let url = "https://api.siliconflow.cn/v1/chat/completions";
    
    let prompt = format!(
        "请将以下学术论文摘要翻译成中文，保持学术性和准确性：\n\n{}",
        text
    );
    
    let request_body = serde_json::json!({
        "model": settings.translation_model,
        "messages": [
            {"role": "system", "content": "你是一个专业的学术论文翻译助手，擅长将英文论文摘要翻译成准确、流畅的中文。"},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 2000,
    });
    
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API 错误: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;
    
    let translated = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("解析翻译结果失败")?;
    
    Ok(translated.to_string())
}