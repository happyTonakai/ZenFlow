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
        let is_thinking_model = self.model.contains("qwen") || self.model.contains("qwq");

        // 对思维链模型，在 system prompt 中添加 /no_think 标记以禁用思考过程
        let effective_system = if is_thinking_model {
            format!("{} /no_think", system)
        } else {
            system.to_string()
        };

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": effective_system},
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

#[cfg(test)]
mod tests {
    use super::*;

    const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
    const OLLAMA_MODEL: &str = "qwen3.5:4b";

    fn ollama_client() -> LlmClient {
        LlmClient::new(OLLAMA_BASE_URL, "ollama", OLLAMA_MODEL)
    }

    #[test]
    fn test_is_available() {
        let client = LlmClient::new("http://example.com", "key", "model");
        assert!(client.is_available());

        let empty = LlmClient::new("http://example.com", "", "model");
        assert!(!empty.is_available());
    }

    #[tokio::test]
    #[ignore] // Requires local Ollama instance
    async fn test_chat_completion_basic() {
        let client = ollama_client();
        let result = client
            .chat_completion("You are a helpful assistant.", "Say hello in one word.", 0.3, 500)
            .await;
        assert!(result.is_ok(), "LLM call failed: {:?}", result.err());
        let response = result.unwrap();
        assert!(!response.is_empty(), "Response should not be empty (thinking models may need higher max_tokens)");
    }

    #[tokio::test]
    #[ignore] // Requires local Ollama instance
    async fn test_score_articles_via_ollama() {
        let client = ollama_client();
        let preferences = "## 感兴趣的主题\n- 大语言模型\n- 机器学习优化";

        let articles = vec![
            super::super::scoring::ArticleInfo {
                id: "2401.001".to_string(),
                title: "Efficient Fine-tuning of Large Language Models".to_string(),
                abstract_text: "We propose a novel method for efficient fine-tuning of LLMs using low-rank adaptation.".to_string(),
            },
            super::super::scoring::ArticleInfo {
                id: "2401.002".to_string(),
                title: "Underwater Basket Weaving Techniques".to_string(),
                abstract_text: "A comprehensive survey of modern basket weaving methods in aquatic environments.".to_string(),
            },
        ];

        let scores = super::super::scoring::score_articles(&client, preferences, &articles).await;
        assert!(scores.is_ok(), "Scoring failed: {:?}", scores.err());
        let scores = scores.unwrap();
        // Should return scores for both articles
        assert!(!scores.is_empty(), "No scores returned");
        // The LLM paper should score higher than basket weaving
        if scores.len() == 2 {
            let llm_score = scores.iter().find(|(id, _)| id == "2401.001").map(|(_, s)| *s);
            let basket_score = scores.iter().find(|(id, _)| id == "2401.002").map(|(_, s)| *s);
            if let (Some(ls), Some(bs)) = (llm_score, basket_score) {
                assert!(ls > bs, "LLM paper ({}) should score higher than basket weaving ({})", ls, bs);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires local Ollama instance
    async fn test_generate_preferences_via_ollama() {
        let client = ollama_client();
        let papers = vec![
            super::super::preferences::FeedbackArticle {
                title: "Attention Is All You Need".to_string(),
                abstract_text: "We propose a new simple network architecture based solely on attention mechanisms.".to_string(),
                status: 2,
                comment: Some("Foundational transformer paper".to_string()),
            },
        ];

        let result = super::super::preferences::generate_initial_preferences(&client, &papers).await;
        assert!(result.is_ok(), "Preference generation failed: {:?}", result.err());
        let prefs = result.unwrap();
        assert!(!prefs.is_empty(), "Generated preferences should not be empty");
    }
}
