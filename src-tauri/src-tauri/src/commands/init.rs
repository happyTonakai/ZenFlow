//! 初始化相关的 Tauri Commands

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use rand::seq::IteratorRandom;
use tauri::{AppHandle, Emitter};

use crate::db;
use crate::db::operations::{get_existing_article_ids, save_articles, set_setting, update_article_status, update_article_translation};
use crate::embedding::EmbeddingClient;
use crate::feed::fetch_arxiv_paper;
use crate::settings::{self, AppSettings};
use crate::algorithm;
use crate::commands::article::fetch_articles;

/// 初始化进度事件
#[derive(Clone, Serialize)]
pub struct InitProgress {
    pub stage: String,
    pub message: String,
    pub progress: f32,  // 0.0 - 1.0
    pub detail: Option<String>,
}

fn emit_progress(app: &AppHandle, stage: &str, message: &str, progress: f32, detail: Option<String>) {
    let _ = app.emit("init-progress", InitProgress {
        stage: stage.to_string(),
        message: message.to_string(),
        progress,
        detail,
    });
}

/// 解析 arXiv ID 的正则表达式
fn extract_arxiv_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    
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
            seen.insert(id.clone());
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
    // 嵌入向量 API 配置
    pub embedding_api_base_url: String,
    pub embedding_api_key: String,
    pub embedding_model: String,
    // 翻译 API 配置
    pub translation_api_base_url: String,
    pub translation_api_key: String,
    pub translation_model: String,
    // 推荐参数
    pub pos_clusters: usize,
    pub neg_clusters: usize,
    pub daily_papers: usize,
    pub negative_alpha: f32,
    pub diversity_ratio: f32,
}

/// 初始化结果
#[derive(Debug, Serialize)]
pub struct InitResult {
    pub settings_saved: bool,
    pub papers_fetched: usize,    // 偏好论文 + RSS 抓取的文章
    pub papers_embedded: usize,    // 生成向量的文章数
    pub pos_clusters: usize,      // 正向聚类数 = 偏好论文数量
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
                    translated_title: None,
                    translated_abstract: None,
                    author: paper.author,
                    category: paper.category,
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
        
        // 更新新保存的文章状态为 LIKED
        for article in &new_articles {
            if let Err(e) = update_article_status(&article.id, crate::config::status::LIKED) {
                tracing::warn!("更新文章状态失败 {}: {}", article.id, e);
            }
        }
    }
    
    Ok(fetched_count)
}

/// 执行完整的初始化流程
#[tauri::command]
pub async fn initialize_app(app: AppHandle, request: InitSettingsRequest) -> Result<InitResult, String> {
    let mut result = InitResult {
        settings_saved: false,
        papers_fetched: 0,
        papers_embedded: 0,
        pos_clusters: 0,
        errors: Vec::new(),
    };
    
    // 0. 清空数据库（初始化时重置数据）
    emit_progress(&app, "clear", "清空数据库", 0.05, None);
    if let Err(e) = db::clear_all_data() {
        result.errors.push(format!("清空数据库失败: {}", e));
        return Ok(result);
    }
    
    // 1. 保存设置
    emit_progress(&app, "save_settings", "保存设置", 0.1, None);
    let settings = AppSettings {
        arxiv_categories: request.arxiv_categories.clone(),
        siliconflow_api_key: request.siliconflow_api_key.clone(),
        embedding_api_base_url: request.embedding_api_base_url.clone(),
        embedding_api_key: request.embedding_api_key.clone(),
        embedding_model: request.embedding_model.clone(),
        translation_api_base_url: request.translation_api_base_url.clone(),
        translation_api_key: request.translation_api_key.clone(),
        translation_model: request.translation_model.clone(),
        pos_clusters: request.pos_clusters,
        neg_clusters: request.neg_clusters,
        daily_papers: request.daily_papers,
        negative_alpha: request.negative_alpha,
        diversity_ratio: request.diversity_ratio,
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
    
    // 保存偏好论文数量（用于后续设置 pos_clusters）
    let favorite_papers_count = request.favorite_papers.len();
    
    // 2. 获取并处理用户喜欢的论文
    if !request.favorite_papers.is_empty() {
        emit_progress(&app, "fetch_favorites", "获取偏好论文", 0.15, Some(format!("共 {} 篇", request.favorite_papers.len())));
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
    emit_progress(&app, "embed_favorites", "生成偏好论文向量", 0.25, None);
    let client = EmbeddingClient::with_config(
        &request.embedding_api_base_url,
        &request.embedding_api_key,
        &request.embedding_model,
    );
    if client.is_available() {
        // 获取已经有向量的文章ID
        let existing_vectors = db::get_vectors_by_statuses(&[
            crate::config::status::LIKED,
            crate::config::status::CLICKED,
        ]).map_err(|e| format!("获取向量数据失败: {}", e))?;
        let has_vector_ids: std::collections::HashSet<String> = existing_vectors
            .into_iter()
            .map(|v| v.id)
            .collect();
        
        // 获取完整的文章数据（包含标题和摘要）
        let liked_articles = db::get_articles(Some(crate::config::status::LIKED), 1000, 0)
            .map_err(|e| format!("获取喜欢文章失败: {}", e))?;
        let clicked_articles = db::get_articles(Some(crate::config::status::CLICKED), 1000, 0)
            .map_err(|e| format!("获取点击文章失败: {}", e))?;
        
        // 只处理还没有向量的文章
        let articles: Vec<_> = liked_articles.into_iter()
            .chain(clicked_articles.into_iter())
            .filter(|a| !has_vector_ids.contains(&a.id))
            .collect();
        
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
        emit_progress(&app, "clustering", "执行聚类分析", 0.5, None);
        match algorithm::update_clusters() {
            Ok(_cluster_result) => {
                // pos_clusters 应该是偏好论文数量（用户喜欢的论文）
                result.pos_clusters = result.papers_fetched;
                tracing::info!("✅ 聚类完成: {} 正向 (偏好论文数)", result.pos_clusters);

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

    // 5. 抓取 RSS 文章并生成推荐（让用户开始使用时就有推荐）
    emit_progress(&app, "fetch_rss", "抓取今日论文", 0.6, None);
    tracing::info!("📥 开始抓取今日 RSS 文章...");
    let mut rss_embedded_count = 0;
    match fetch_articles().await {
        Ok(count) => {
            result.papers_fetched += count; // 累加 RSS 抓取的文章数
            tracing::info!("✅ 抓取了 {} 篇文章", count);

            // 为新文章生成向量
            if count > 0 {
                let client = EmbeddingClient::with_config(
                    &request.embedding_api_base_url,
                    &request.embedding_api_key,
                    &request.embedding_model,
                );
                if client.is_available() {
                    // 获取还没有向量的文章
                    let existing_vectors = db::get_vectors_by_statuses(&[
                        crate::config::status::UNREAD,
                        crate::config::status::CLICKED,
                        crate::config::status::LIKED,
                        crate::config::status::DISLIKED,
                    ]).unwrap_or_default();
                    let has_vector_ids: std::collections::HashSet<String> = existing_vectors
                        .into_iter()
                        .map(|v| v.id)
                        .collect();

                    let all_articles = db::get_articles(None, 100, 0).unwrap_or_default();
                    let articles_without_vector: Vec<_> = all_articles
                        .into_iter()
                        .filter(|a| !has_vector_ids.contains(&a.id))
                        .collect();

                    tracing::info!("为 {} 篇新文章生成向量...", articles_without_vector.len());
                    emit_progress(&app, "embed_rss", "生成论文向量", 0.7, Some(format!("共 {} 篇", articles_without_vector.len())));

                    for (_i, article) in articles_without_vector.into_iter().enumerate() {
                        let text = match &article.abstract_text {
                            Some(a) => format!("{} {}", article.title, a),
                            None => article.title.clone(),
                        };

                        match client.embed(&text).await {
                            Ok(vector) => {
                                if let Err(e) = db::save_article_vector(&article.id, &vector) {
                                    tracing::error!("保存向量失败 {}: {}", article.id, e);
                                } else {
                                    tracing::info!("✅ 文章 {} 向量已保存", article.id);
                                    rss_embedded_count += 1;
                                }
                            }
                            Err(e) => {
                                tracing::error!("生成向量失败 {}: {}", article.id, e);
                            }
                        }
                    }

                    result.papers_embedded += rss_embedded_count; // 累加 RSS 文章的向量数

                    // 重新计算分数生成推荐
                    if let Err(e) = algorithm::update_clusters() {
                        tracing::warn!("更新聚类失败: {}", e);
                    }
                    if let Ok(count) = algorithm::recalculate_all_scores() {
                        tracing::info!("✅ 已为 {} 篇文章计算推荐分数", count);
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("抓取文章失败: {}", e);
        }
    }

    // 更新 pos_clusters 为偏好论文数量
    result.pos_clusters = favorite_papers_count;
    tracing::info!("📊 初始化完成: 获取文章={}, 生成向量={}, 正向偏好={}",
        result.papers_fetched, result.papers_embedded, result.pos_clusters);

    // 6. 翻译推荐的文章（按照推荐算法的逻辑：70% 分数排序 + 30% 随机多样性）
    if crate::config::is_translation_configured() {
        let settings = crate::settings::get_settings().unwrap_or_default();
        let daily_papers = settings.daily_papers;
        let diversity_ratio = settings.diversity_ratio;
        
        let score_based_count = (daily_papers as f32 * (1.0 - diversity_ratio)).ceil() as usize;
        let diversity_count = daily_papers - score_based_count;
        
        emit_progress(&app, "translate", "翻译推荐论文", 0.95, Some(format!("推荐 {} 篇", daily_papers)));
        
        // 获取分数 > 0 的所有文章
        if let Ok(all_articles) = db::get_articles(None, 1000, 0) {
            let mut scored_articles: Vec<_> = all_articles.iter().filter(|a| a.score > 0.0).cloned().collect();
            
            // 按分数降序排序
            scored_articles.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            
            // 70% 按分数排序
            let score_based: Vec<_> = scored_articles.iter().take(score_based_count).cloned().collect();
            
            // 30% 从剩余中随机
            let diversity: Vec<_> = {
                let remaining: Vec<_> = scored_articles.iter().skip(score_based_count).cloned().collect();
                if !remaining.is_empty() {
                    let mut rng = rand::thread_rng();
                    let count = diversity_count.min(remaining.len());
                    let indices: Vec<usize> = (0..remaining.len()).collect();
                    let chosen: Vec<usize> = indices.iter().choose_multiple(&mut rng, count).into_iter().cloned().collect();
                    chosen.iter().map(|&i| remaining[i].clone()).collect()
                } else {
                    vec![]
                }
            };
            
            // 合并并去重
            let mut final_recommendations: Vec<String> = score_based.iter().map(|a| a.id.clone()).collect();
            for a in &diversity {
                if !final_recommendations.contains(&a.id) {
                    final_recommendations.push(a.id.clone());
                }
            }
            
            tracing::info!("📝 推荐论文: 分数排序 {} 篇 + 随机多样性 {} 篇 = 共 {} 篇", 
                score_based.len(), diversity.len(), final_recommendations.len());
            
            // 获取完整的文章信息进行翻译
            let papers_to_translate: Vec<TranslateRequest> = all_articles
                .iter()
                .filter(|a| final_recommendations.contains(&a.id))
                .map(|a| TranslateRequest {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    abstract_text: a.abstract_text.clone().unwrap_or_default(),
                })
                .collect();
            
            if !papers_to_translate.is_empty() {
                tracing::info!("📝 开始翻译 {} 篇推荐论文", papers_to_translate.len());
                
                match translate_batch(papers_to_translate).await {
                    Ok(results) => {
                        for r in &results {
                            if let Err(e) = update_article_translation(&r.id, &r.title, &r.abstract_text) {
                                tracing::warn!("更新翻译失败 {}: {}", r.id, e);
                            }
                        }
                        tracing::info!("✅ 翻译完成");
                    }
                    Err(e) => {
                        tracing::warn!("翻译失败: {}", e);
                    }
                }
            }
        }
    }

    // 7. 标记为已初始化
    emit_progress(&app, "complete", "初始化完成", 1.0, Some(format!("共获取 {} 篇文章", result.papers_fetched)));
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

/// 翻译请求结构（用于批量翻译）

#[derive(Debug, Serialize, Deserialize)]

pub struct TranslateRequest {

    pub id: String,

    pub title: String,

    pub abstract_text: String,

}



/// 翻译结果结构

#[derive(Debug, Serialize, Deserialize)]

pub struct TranslateResult {

    pub id: String,

    pub title: String,

    pub abstract_text: String,

}



/// 批量翻译论文（使用 LLM）

#[tauri::command]

pub async fn translate_batch(papers: Vec<TranslateRequest>) -> Result<Vec<TranslateResult>, String> {

    if papers.is_empty() {

        return Ok(Vec::new());

    }

    

    let settings = settings::get_settings()

        .map_err(|e| format!("获取设置失败: {}", e))?;

    

    // 检查翻译 API 是否已配置

    if settings.translation_api_key.is_empty() || settings.translation_api_base_url.is_empty() {

        tracing::info!("翻译 API 未配置，跳过翻译");

        return Ok(papers.into_iter().map(|p| TranslateResult {

            id: p.id,

            title: p.title,

            abstract_text: p.abstract_text,

        }).collect());

    }

    

    // 使用配置中的 API

    let client = reqwest::Client::new();

    let url = format!("{}/chat/completions", settings.translation_api_base_url.trim_end_matches('/'));

    

    // 构建批量翻译的提示词

    let system_prompt = r#"你是一个专业的学术论文翻译助手。请将用户提供的多篇学术论文的标题和摘要翻译成中文。

请严格按照以下JSON数组格式返回，不要返回任何其他内容：

[{"id": "论文ID", "title": "翻译后的标题", "abstract": "翻译后的摘要"}, ...]



注意：

1. 只返回JSON数组，不要有任何解释、注释或额外文本

2. 保持学术性和准确性，像Transformer、GPU、CNN这种众所周知的词汇不用翻译

3. 保持LaTeX公式和符号不变

4. 请逐篇翻译，保持id与输入对应"#;

    

    // 构建用户提示词，包含所有论文

    let mut user_content = String::new();

    for paper in &papers {

        user_content.push_str(&format!("论文ID: {}\n", paper.id));

        user_content.push_str(&format!("标题: {}\n", paper.title));

        user_content.push_str(&format!("摘要: {}\n", paper.abstract_text));

        user_content.push_str("---\n");

    }

    

    let is_modelscope = settings.translation_api_base_url.contains("modelscope");

    let mut request_body = serde_json::json!({
        "model": settings.translation_model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.3,
        "max_tokens": 8000,
    });

    if is_modelscope {
        request_body["enable_thinking"] = serde_json::json!(false);
    }

    

    let response = client

        .post(&url)

        .header("Authorization", format!("Bearer {}", settings.translation_api_key))

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

    

    let content = json

        .get("choices")

        .and_then(|c| c.get(0))

        .and_then(|c| c.get("message"))

        .and_then(|m| m.get("content"))

        .and_then(|c| c.as_str())

        .ok_or("解析翻译结果失败")?;

    

    // 清理可能的代码块标记

    let content = content

        .trim_start_matches("```json")

        .trim_start_matches("```")

        .trim_end_matches("```")

        .trim();

    

    // 解析翻译结果

    let results: Vec<TranslateResult> = match serde_json::from_str(content) {

        Ok(r) => r,

        Err(e) => {

            tracing::warn!("解析翻译结果失败: {}，返回原始文本", e);

            // 如果解析失败，返回原始论文（不翻译）

            return Ok(papers.into_iter().map(|p| TranslateResult {

                id: p.id,

                title: p.title,

                abstract_text: p.abstract_text,

            }).collect());

        }

    };

    

    Ok(results)

}



/// 翻译单个文本（兼容旧接口）

#[tauri::command]

pub async fn translate_text(text: String, _api_key: Option<String>) -> Result<String, String> {

    // 先尝试获取论文 ID（从文本中提取）

    // 由于旧接口只接受单个文本，我们将其作为单篇论文处理

    let translate_request = vec![TranslateRequest {

        id: "single".to_string(),

        title: text.lines().next().unwrap_or("").to_string(),

        abstract_text: text.lines().skip(1).collect::<Vec<_>>().join("\n"),

    }];

    

    let results = translate_batch(translate_request).await?;

    

    if results.is_empty() {

        return Ok(text);

    }

    

    let result = &results[0];

    Ok(serde_json::json!({

        "title": result.title,

        "abstract": result.abstract_text

    }).to_string())

}

/// 测试/请求钥匙串访问权限
/// 尝试读取或写入钥匙串来触发系统权限对话框
#[tauri::command]
pub async fn request_keychain_access(api_key: String) -> Result<bool, String> {
    // 先尝试保存 API Key 到钥匙串，这会触发权限请求
    match settings::save_api_key_to_keyring("siliconflow_api_key", &api_key) {
        Ok(_) => {
            tracing::info!("✅ API Key 已保存到钥匙串");
            Ok(true)
        }
        Err(e) => {
            tracing::warn!("❌ 钥匙串访问失败: {}", e);
            Err(format!("钥匙串访问失败: {}", e))
        }
    }
}