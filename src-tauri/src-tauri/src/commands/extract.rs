//! 论文提取 Tauri 命令

use tauri_plugin_clipboard_manager::ClipboardExt;

/// 提取 arXiv 论文纯文本
///
/// 优先使用 HTML 版本，如果没有则回退到 TeX 源码。
/// 默认剥离参考文献以节省 token。
#[tauri::command]
pub async fn extract_paper(arxiv_id: String) -> Result<String, String> {
    crate::extractor::extract_paper_text(&arxiv_id, /* no_refs */ true)
        .await
        .map_err(|e| format!("提取论文失败: {}", e))
}

/// 提取 arXiv 论文纯文本并复制到剪贴板
///
/// 返回提取的文本内容。默认剥离参考文献。
#[tauri::command]
pub async fn extract_paper_to_clipboard(arxiv_id: String, app: tauri::AppHandle) -> Result<String, String> {
    let text = crate::extractor::extract_paper_text(&arxiv_id, /* no_refs */ true)
        .await
        .map_err(|e| format!("提取论文失败: {}", e))?;

    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| format!("复制到剪贴板失败: {}", e))?;

    tracing::info!("论文 {} 已提取并复制到剪贴板 ({} 字符)", arxiv_id, text.len());
    Ok(text)
}

/// 仅提取 LaTeX 源码（用于调试）
///
/// 强制走 TeX 路径，跳过 HTML 检查。默认剥离参考文献。
#[tauri::command]
pub async fn extract_paper_latex(arxiv_id: String) -> Result<String, String> {
    crate::extractor::extract_paper_text_from_tex(&arxiv_id, /* no_refs */ true)
        .await
        .map_err(|e| format!("提取 LaTeX 源码失败: {}", e))
}
