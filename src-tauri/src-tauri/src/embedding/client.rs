//! SiliconFlow Embedding API 客户端

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config;
use crate::settings;

/// Embedding API 请求
#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

/// Embedding API 响应
#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Embedding 客户端
pub struct EmbeddingClient {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
}

impl EmbeddingClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        // 从设置中获取 API 配置
        let settings = settings::get_settings().ok();
        
        let api_key = settings
            .as_ref()
            .and_then(|s| {
                if !s.embedding_api_key.is_empty() {
                    Some(s.embedding_api_key.clone())
                } else {
                    None
                }
            })
            .or_else(|| config::siliconflow_api_key());
        
        let base_url = settings
            .as_ref()
            .map(|s| s.embedding_api_base_url.clone())
            .unwrap_or_else(|| "https://api.siliconflow.cn/v1".to_string());
        
        let model = settings
            .as_ref()
            .map(|s| s.embedding_model.clone())
            .unwrap_or_else(|| "BAAI/bge-m3".to_string());
        
        Self {
            client,
            api_key,
            base_url,
            model,
        }
    }
    
    /// 使用指定 API 配置创建客户端
    pub fn with_config(base_url: &str, api_key: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        let api_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key.to_string())
        };
        
        Self {
            client,
            api_key,
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
    
    /// 获取文本的向量表示
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow!("Embedding API key not set"))?;
        
        let truncated_text: String = text.chars().take(config::EMBEDDING_TEXT_MAX_LENGTH).collect();
        
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: truncated_text,
        };
        
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("API error: {} - {}", status, body));
        }
        
        let data: EmbeddingResponse = response.json().await?;
        
        data.data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| anyhow!("No embedding in response"))
    }
    
    /// 批量获取向量（带速率限制）
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Option<Vec<f32>>>> {
        let mut results = Vec::with_capacity(texts.len());
        
        for text in texts {
            // 简单的速率限制：每次请求间隔 30ms
            tokio::time::sleep(Duration::from_millis(30)).await;
            
            match self.embed(text).await {
                Ok(embedding) => results.push(Some(embedding)),
                Err(e) => {
                    tracing::error!("Embedding error: {}", e);
                    results.push(None);
                }
            }
        }
        
        Ok(results)
    }
    
    /// 检查 API 是否可用
    pub fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}

impl Default for EmbeddingClient {
    fn default() -> Self {
        Self::new()
    }
}

/// 为文章生成 embedding（标题 + 摘要）
pub async fn embed_article(title: &str, abstract_text: Option<&str>) -> Result<Vec<f32>> {
    let client = EmbeddingClient::new();
    
    let text = match abstract_text {
        Some(a) if !a.is_empty() => format!("{} {}", title, a),
        _ => title.to_string(),
    };
    
    client.embed(&text).await
}
