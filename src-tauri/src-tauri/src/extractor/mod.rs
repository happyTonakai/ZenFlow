//! arXiv 论文文本提取模块
//!
//! 从 arXiv 论文中提取纯文本内容。优先使用 HTML 版本，
//! 如果没有 HTML 则回退到 TeX 源码。
//!
//! 感谢以下开源项目：
//! - [arxiv2md](https://github.com/lukas-blecher/arxiv2md) - HTML 到 Markdown 转换
//! - [arxiv-to-prompt](https://github.com/AgnostiqHQ/arxiv-to-prompt) - LaTeX 源码合并
//! - [pylatexenc](https://github.com/phfaist/pylatexenc) - LaTeX 到 Unicode 转换

mod checker;
mod html;
mod tex;
mod latex2text;

pub use checker::has_html_version;
pub use html::fetch_and_convert;
pub use tex::{download_and_extract, find_main_tex, flatten_tex_content};
pub use latex2text::convert_latex_to_text;

use std::path::PathBuf;
use std::time::Duration;

/// 从 arXiv 提取论文纯文本
///
/// 优先尝试 HTML 版本，如果没有则下载 TeX 源码进行转换。
pub async fn extract_paper_text(arxiv_id: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ZenFlow/0.1.0")
        .build()?;

    // 优先使用 HTML 版本
    if checker::has_html_version(&client, arxiv_id).await? {
        tracing::info!("论文 {} 有 HTML 版本，使用 HTML 提取", arxiv_id);
        return html::fetch_and_convert(&client, arxiv_id).await;
    }

    // 回退到 TeX 源码
    tracing::info!("论文 {} 无 HTML 版本，回退到 TeX 源码", arxiv_id);
    let cache_dir = get_cache_dir()?;
    let paper_dir = cache_dir.join(arxiv_id.replace('/', "_"));

    tex::download_and_extract(&client, arxiv_id, &paper_dir).await?;
    let main_tex = tex::find_main_tex(&paper_dir)?;
    let latex_source = tex::flatten_tex_content(&paper_dir, &main_tex)?;
    let text = latex2text::convert_latex_to_text(&latex_source);
    Ok(text)
}

/// 强制使用 TeX 源码提取（跳过 HTML 检查）
pub async fn extract_paper_text_from_tex(arxiv_id: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ZenFlow/0.1.0")
        .build()?;

    let cache_dir = get_cache_dir()?;
    let paper_dir = cache_dir.join(arxiv_id.replace('/', "_"));

    tex::download_and_extract(&client, arxiv_id, &paper_dir).await?;
    let main_tex = tex::find_main_tex(&paper_dir)?;
    let latex_source = tex::flatten_tex_content(&paper_dir, &main_tex)?;
    let text = latex2text::convert_latex_to_text(&latex_source);
    Ok(text)
}

fn get_cache_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("无法获取 HOME 环境变量"))?;
    let cache_dir = PathBuf::from(home).join(".zenflow").join("arxiv_cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_extract_paper_text_html() {
        // 1706.03762 (Attention Is All You Need) - 有 HTML 版本
        let text = extract_paper_text("1706.03762").await.unwrap();
        assert!(
            text.to_lowercase().contains("attention"),
            "应该包含论文标题关键词, 实际长度: {}",
            text.len()
        );
        assert!(text.len() > 1000, "应该包含大量文本, 实际长度: {}", text.len());
    }

    #[tokio::test]
    #[ignore]
    async fn test_extract_paper_text_tex() {
        // 1604.03121 - 无 HTML，单一 tex 文件
        let text = extract_paper_text("1604.03121").await.unwrap();
        assert!(!text.is_empty(), "不应该为空");
        assert!(!text.contains("\\documentclass"), "不应该包含 LaTeX preamble");
        assert!(text.len() > 500, "应该包含大量文本, 实际长度: {}", text.len());
    }

    #[tokio::test]
    #[ignore]
    async fn test_extract_paper_multi_tex() {
        // 2505.09388 - 强制走 TeX 路径，多文件
        let text = extract_paper_text_from_tex("2505.09388").await.unwrap();
        assert!(!text.is_empty(), "不应该为空");
        assert!(text.len() > 1000, "应该包含大量文本, 实际长度: {}", text.len());
    }
}
