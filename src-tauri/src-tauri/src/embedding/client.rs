//! SiliconFlow Embedding API 客户端

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config;

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
}

impl EmbeddingClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key: config::siliconflow_api_key(),
        }
    }
    
    /// 获取文本的向量表示
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow!("SILICONFLOW_API_KEY not set"))?;
        
        let truncated_text: String = text.chars().take(config::EMBEDDING_TEXT_MAX_LENGTH).collect();
        
        let request = EmbeddingRequest {
            model: config::EMBEDDING_MODEL.to_string(),
            input: truncated_text,
        };
        
        let response = self.client
            .post(config::SILICONFLOW_API_URL)
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
