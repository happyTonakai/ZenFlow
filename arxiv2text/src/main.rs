use arxiv2text::extractor;
use regex::Regex;
use std::io::{self, IsTerminal};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("arxiv2text=info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: arxiv2text <arxiv-id|arxiv-url> [--tex] [--no-refs]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --tex      Force TeX source extraction (skip HTML)");
        eprintln!("  --no-refs  Strip references/bibliography section");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  arxiv2text 1706.03762");
        eprintln!("  arxiv2text https://arxiv.org/abs/1706.03762");
        eprintln!("  arxiv2text https://arxiv.org/pdf/2305.15334");
        eprintln!("  arxiv2text https://arxiv.org/html/2305.15334v2");
        eprintln!("  arxiv2text 1706.03762 --tex --no-refs");
        std::process::exit(1);
    }

    let arxiv_id = extract_arxiv_id(&args[1]);
    let force_tex = args.iter().any(|a| a == "--tex");
    let no_refs = args.iter().any(|a| a == "--no-refs");

    let text = if force_tex {
        extractor::extract_paper_text_from_tex(&arxiv_id, no_refs).await
    } else {
        extractor::extract_paper_text(&arxiv_id, no_refs).await
    };

    match text {
        Ok(text) => {
            if io::stdout().is_terminal() {
                eprintln!("({} chars)", text.len());
            }
            print!("{text}");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

/// 从 arXiv URL 或 ID 中提取 arXiv ID
///
/// 支持:
///   - 纯 ID: 1706.03762, 1706.03762v2, hep-ph/0604001
///   - abs URL: https://arxiv.org/abs/1706.03762, https://arxiv.org/abs/1706.03762v2
///   - PDF URL: https://arxiv.org/pdf/1706.03762, https://arxiv.org/pdf/1706.03762v1.pdf
///   - HTML URL: https://arxiv.org/html/1706.03762, https://arxiv.org/html/2305.15334v2
///   - arxiv: ID 前缀: arxiv:1706.03762
fn extract_arxiv_id(input: &str) -> String {
    let input = input.trim();

    // 去掉 arxiv: 前缀
    let input = input
        .strip_prefix("arxiv:")
        .or_else(|| input.strip_prefix("ARXIV:"))
        .unwrap_or(input);

    // URL 模式: https://arxiv.org/{abs,pdf,html}/<id>[vN][.pdf]
    let url_re =
        Regex::new(r"(?i)https?://arxiv\.org/(abs|pdf|html)/([^\s/]+(?:/[^\s/]+)?)").unwrap();
    if let Some(caps) = url_re.captures(input) {
        let raw_id = caps.get(2).unwrap().as_str();
        // PDF URL 可能有 .pdf 后缀
        let id = raw_id.strip_suffix(".pdf").unwrap_or(raw_id);
        return normalize_arxiv_id(id);
    }

    // 直接是 ID (可能带版本号 v1/v2)
    normalize_arxiv_id(input)
}

/// 规范化 arXiv ID，保留版本号
///
/// 新格式: 4位年份.4-5位编号 (如 1706.03762, 1706.03762v2)
/// 旧格式: 分类/编号 (如 hep-ph/0604001)
fn normalize_arxiv_id(id: &str) -> String {
    let id = id.trim().trim_end_matches('/');

    // 新格式: 2305.15334v2
    let re_new = Regex::new(r"^(\d{4}\.\d{4,5})(v\d+)?$").unwrap();
    if re_new.is_match(id) {
        return id.to_string();
    }

    // 旧格式: hep-ph/0604001v1
    let re_old = Regex::new(r"^([a-zA-Z-]+/\d{7})(v\d+)?$").unwrap();
    if re_old.is_match(id) {
        return id.to_string();
    }

    // 无法识别，原样返回
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_arxiv_id_plain() {
        assert_eq!(extract_arxiv_id("1706.03762"), "1706.03762");
        assert_eq!(extract_arxiv_id("1706.03762v2"), "1706.03762v2");
        assert_eq!(extract_arxiv_id("hep-ph/0604001"), "hep-ph/0604001");
    }

    #[test]
    fn test_extract_arxiv_id_abs_url() {
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/abs/1706.03762"),
            "1706.03762"
        );
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/abs/1706.03762v2"),
            "1706.03762v2"
        );
        assert_eq!(
            extract_arxiv_id("http://arxiv.org/abs/2305.15334"),
            "2305.15334"
        );
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/abs/hep-ph/0604001"),
            "hep-ph/0604001"
        );
    }

    #[test]
    fn test_extract_arxiv_id_pdf_url() {
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/pdf/1706.03762"),
            "1706.03762"
        );
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/pdf/1706.03762v1.pdf"),
            "1706.03762v1"
        );
    }

    #[test]
    fn test_extract_arxiv_id_html_url() {
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/html/2305.15334v2"),
            "2305.15334v2"
        );
        assert_eq!(
            extract_arxiv_id("https://arxiv.org/html/1706.03762"),
            "1706.03762"
        );
    }

    #[test]
    fn test_extract_arxiv_id_arxiv_prefix() {
        assert_eq!(extract_arxiv_id("arxiv:1706.03762"), "1706.03762");
        assert_eq!(extract_arxiv_id("ARXIV:1706.03762v3"), "1706.03762v3");
    }
}
