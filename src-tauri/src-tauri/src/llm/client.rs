//! OpenAI 兼容的 LLM 客户端

use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;

/// LLM 客户端，支持 OpenAI 兼容的 chat completions API
pub struct LlmClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    /// 检查 API 是否可用（key 非空）
    pub fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// 发送 chat completion 请求，返回助手回复内容
    pub async fn chat_completion(
        &self,
        system: &str,
        user: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        if !self.is_available() {
            return Err(anyhow!("LLM API key not configured"));
        }

        let url = format!("{}/chat/completions", self.base_url);

        let is_modelscope = self.base_url.contains("modelscope");
        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        if is_modelscope {
            request_body["enable_thinking"] = serde_json::json!(false);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("LLM API error: {}", error_text));
        }

        let json: serde_json::Value = response.json().await?;

        let content = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow!("Failed to parse LLM response"))?;

        Ok(content.to_string())
    }
}
