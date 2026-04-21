//! arXiv 论文文本提取模块
//!
//! 从 arXiv 论文中提取纯文本内容。优先使用 HTML 版本，
//! 如果没有 HTML 则回退到 TeX 源码。

mod checker;
mod html;
pub mod latex2text;
mod tex;

use std::path::PathBuf;
use std::time::Duration;

/// 从 arXiv 提取论文纯文本
///
/// 优先尝试 HTML 版本，如果没有则下载 TeX 源码进行转换。
pub async fn extract_paper_text(arxiv_id: &str, no_refs: bool) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("arxiv2text/0.1.0")
        .build()?;

    // 优先使用 HTML 版本
    if checker::has_html_version(&client, arxiv_id).await? {
        tracing::info!("论文 {} 有 HTML 版本，使用 HTML 提取", arxiv_id);
        return html::fetch_and_convert(&client, arxiv_id, no_refs).await;
    }

    // 回退到 TeX 源码
    tracing::info!("论文 {} 无 HTML 版本，回退到 TeX 源码", arxiv_id);
    extract_tex(&client, arxiv_id, no_refs).await
}

/// 强制使用 TeX 源码提取（跳过 HTML 检查）
pub async fn extract_paper_text_from_tex(arxiv_id: &str, no_refs: bool) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("arxiv2text/0.1.0")
        .build()?;

    extract_tex(&client, arxiv_id, no_refs).await
}

async fn extract_tex(
    client: &reqwest::Client,
    arxiv_id: &str,
    no_refs: bool,
) -> anyhow::Result<String> {
    let cache_dir = get_cache_dir()?;
    let paper_dir = cache_dir.join(arxiv_id.replace('/', "_"));

    tex::download_and_extract(client, arxiv_id, &paper_dir).await?;
    let main_tex = tex::find_main_tex(&paper_dir)?;
    let latex_source = tex::flatten_tex_content(&paper_dir, &main_tex)?;
    let text = latex2text::convert_latex_to_text(&latex_source, no_refs);
    Ok(text)
}

fn get_cache_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("无法获取 HOME 环境变量"))?;
    let cache_dir = PathBuf::from(home).join(".arxiv2text").join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}
