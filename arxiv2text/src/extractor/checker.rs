//! 检查 arXiv 论文是否有 HTML 版本

/// 检查指定 arXiv 论文是否有 HTML 版本
///
/// 通过 HEAD 请求 `https://arxiv.org/html/{id}` 来判断。
pub async fn has_html_version(client: &reqwest::Client, arxiv_id: &str) -> anyhow::Result<bool> {
    let url = format!("https://arxiv.org/html/{}", arxiv_id);
    let resp = client.head(&url).send().await?;
    Ok(resp.status().is_success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_has_html_version_true() {
        // 1706.03762 (Attention Is All You Need) 有 HTML
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ZenFlow/0.1.0")
            .build()
            .unwrap();
        let result = has_html_version(&client, "1706.03762").await.unwrap();
        assert!(result, "1706.03762 应该有 HTML 版本");
    }

    #[tokio::test]
    #[ignore]
    async fn test_has_html_version_false() {
        // 1604.03121 没有 HTML
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ZenFlow/0.1.0")
            .build()
            .unwrap();
        let result = has_html_version(&client, "1604.03121").await.unwrap();
        assert!(!result, "1604.03121 不应该有 HTML 版本");
    }
}
