//! 获取 arXiv HTML 版本并转换为纯文本

use regex::Regex;

/// 获取 arXiv HTML 版本并转换为纯文本
pub async fn fetch_and_convert(
    client: &reqwest::Client,
    arxiv_id: &str,
    no_refs: bool,
) -> anyhow::Result<String> {
    let url = format!("https://arxiv.org/html/{}", arxiv_id);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("获取 HTML 失败: HTTP {}", resp.status());
    }
    let html = resp.text().await?;
    Ok(parse_html_to_text(&html, no_refs))
}

/// 将 arXiv HTML 转换为纯文本
fn parse_html_to_text(html: &str, no_refs: bool) -> String {
    let html = convert_mathml_to_latex(html);
    let mut html = strip_unwanted_elements(&html);

    if no_refs {
        html = strip_references(&html);
    }

    let mut parts: Vec<String> = Vec::new();

    // 提取标题
    if let Some(title) = extract_ltx_text(&html, "ltx_title_document") {
        parts.push(title);
    }

    // 提取作者
    if let Some(authors) = extract_ltx_authors(&html) {
        if !authors.is_empty() {
            parts.push(format!("Authors: {}", authors));
        }
    }

    // 提取摘要
    if let Some(abstract_text) = extract_ltx_abstract(&html) {
        parts.push(String::from("Abstract"));
        parts.push(abstract_text);
    }

    // 提取正文各节
    let sections = extract_sections(&html);
    parts.extend(sections);

    parts.join("\n\n")
}

/// 将 MathML <math> 标签替换为 LaTeX 代码
fn convert_mathml_to_latex(html: &str) -> String {
    let re = Regex::new(
        r#"(?s)<math[^>]*>.*?<annotation[^>]*encoding="application/x-tex"[^>]*>(.*?)</annotation>.*?</math>"#
    ).unwrap();
    let result = re.replace_all(html, |caps: &regex::Captures| {
        let latex = caps.get(1).unwrap().as_str().trim();
        format!("${}$", latex)
    });
    result.to_string()
}

/// 剥离不需要的 HTML 元素
fn strip_unwanted_elements(html: &str) -> String {
    // 移除 script/style/noscript/footer 标签及内容（不使用 backreference，逐个处理）
    let mut result = html.to_string();
    for tag in &["script", "style", "noscript", "footer"] {
        let pattern = format!(r"(?is)<{}[^>]*>.*?</{}>", tag, tag);
        let re = Regex::new(&pattern).unwrap();
        result = re.replace_all(&result, "").to_string();
    }
    // 移除 navbar
    let re = Regex::new(r#"(?is)<nav[^>]*class="[^"]*ltx_page_navbar[^"]*"[^>]*>.*?</nav>"#).unwrap();
    let result = re.replace_all(&result, "");
    // 移除 TOC nav
    let re = Regex::new(r#"(?is)<nav[^>]*class="[^"]*ltx_TOC[^"]*"[^>]*>.*?</nav>"#).unwrap();
    let result = re.replace_all(&result, "");
    // 移除 pagination
    let re = Regex::new(r#"(?is)<div[^>]*class="[^"]*ltx_pagination[^"]*"[^>]*>.*?</div>"#).unwrap();
    let result = re.replace_all(&result, "");
    result.to_string()
}

/// 从指定 CSS class 的标签中提取纯文本
fn extract_ltx_text(html: &str, class_name: &str) -> Option<String> {
    let pattern = format!(
        r#"(?is)<[^>]*class="[^"]*{}[^"]*"[^>]*>(.*?)</[^>]+>"#,
        regex::escape(class_name)
    );
    let re = Regex::new(&pattern).ok()?;
    let caps = re.captures(html)?;
    let content = caps.get(1)?.as_str();
    Some(strip_tags(content).trim().to_string())
}

/// 提取作者信息
fn extract_ltx_authors(html: &str) -> Option<String> {
    let re = Regex::new(r#"(?is)<div[^>]*class="[^"]*ltx_authors[^"]*"[^>]*>(.*?)</div>"#).ok()?;
    let caps = re.captures(html)?;
    let content = caps.get(1)?.as_str();
    let text = strip_tags(content);
    let joined = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned: Vec<&str> = joined
        .split(", ")
        .flat_map(|s| s.split(" and "))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.len() < 80 && !s.contains('@'))
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join(", "))
    }
}

/// 提取摘要
fn extract_ltx_abstract(html: &str) -> Option<String> {
    let re = Regex::new(r#"(?is)<div[^>]*class="[^"]*ltx_abstract[^"]*"[^>]*>(.*?)</div>"#).ok()?;
    let caps = re.captures(html)?;
    let content = caps.get(1)?.as_str();
    let text = strip_tags(content);
    let text = collapse_whitespace(&text);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// 从 HTML 中提取各节内容
fn extract_sections(html: &str) -> Vec<String> {
    let mut result = Vec::new();

    // 匹配 section 标签及其内容
    let section_re = Regex::new(r#"(?is)<section[^>]*>(.*?)</section>"#).unwrap();
    for caps in section_re.captures_iter(html) {
        let section_content = caps.get(1).unwrap().as_str();

        // 提取标题 (h1-h6) — 不使用 backreference（Rust regex 不支持）
        let heading_re = Regex::new(r#"(?is)<h[1-6][^>]*>(.*?)</h[1-6]>"#).unwrap();
        let mut heading_text = String::new();
        if let Some(h_caps) = heading_re.captures(section_content) {
            heading_text = strip_tags(h_caps.get(1).unwrap().as_str());
            heading_text = collapse_whitespace(&heading_text);
        }

        // 提取段落内容
        let para_re = Regex::new(r#"(?is)<p[^>]*>(.*?)</p>"#).unwrap();
        let mut paragraphs = Vec::new();
        for p_caps in para_re.captures_iter(section_content) {
            let p_text = strip_tags(p_caps.get(1).unwrap().as_str());
            let p_text = collapse_whitespace(&p_text);
            if !p_text.is_empty() {
                paragraphs.push(p_text);
            }
        }

        if !heading_text.is_empty() || !paragraphs.is_empty() {
            if !heading_text.is_empty() {
                result.push(heading_text);
            }
            result.extend(paragraphs);
        }
    }

    result
}

/// 剥离所有 HTML 标签，保留文本内容
fn strip_tags(html: &str) -> String {
    let re = Regex::new(r"(?s)<[^>]+>").unwrap();
    let text = re.replace_all(html, " ");
    collapse_whitespace(&html_escape_decode(&text))
}

/// 解码常见的 HTML 实体
fn html_escape_decode(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// 折叠多余空白
fn collapse_whitespace(text: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text, " ").trim().to_string()
}

/// 剥离参考文献/引用相关的 section 和 div
///
/// 参考 arxiv2md 的做法：移除 class 包含 ltx_bibliography 的 section，
/// 以及行内引用链接 (href="#bib.bib...")。
fn strip_references(html: &str) -> String {
    let mut result = html.to_string();

    // 移除 ltx_bibliography section（参考文献章节）
    // 用 r##"..."## 因为内容包含 "# 序列
    let re = Regex::new(
        r##"(?is)<section[^>]*class="[^"]*ltx_bibliography[^"]*"[^>]*>.*?</section>"##,
    )
    .unwrap();
    result = re.replace_all(&result, "").to_string();

    // 移除 ltx_bibitem 引用条目
    let re =
        Regex::new(r##"(?is)<div[^>]*class="[^"]*ltx_bibitem[^"]*"[^>]*>.*?</div>"##).unwrap();
    result = re.replace_all(&result, "").to_string();

    // 移除行内引用标签 [1], [2] 等 (链接到 #bib.bib...)
    let re = Regex::new(r##"(?is)<a[^>]*href="#bib[^"]*"[^>]*>.*?</a>"##).unwrap();
    result = re.replace_all(&result, "").to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags_basic() {
        assert_eq!(strip_tags("<p>Hello <b>World</b></p>"), "Hello World");
        assert_eq!(strip_tags("plain text"), "plain text");
    }

    #[test]
    fn test_convert_mathml_to_latex() {
        let input = r#"<math><mi>x</mi><mo>=</mo><mn>1</mn><annotation encoding="application/x-tex">x = 1</annotation></math>"#;
        let result = convert_mathml_to_latex(input);
        assert!(result.contains("$x = 1$"), "结果: {}", result);
    }

    #[test]
    fn test_html_escape_decode() {
        assert_eq!(html_escape_decode("A &amp; B"), "A & B");
        assert_eq!(html_escape_decode("&lt;tag&gt;"), "<tag>");
    }

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
    }

    #[test]
    fn test_parse_html_to_text_basic() {
        let html = r#"
        <html><body>
        <article class="ltx_document">
            <h1 class="ltx_title ltx_title_document">Test Paper Title</h1>
            <div class="ltx_authors">
                <span class="ltx_text ltx_font_bold">John Doe</span>
            </div>
            <div class="ltx_abstract">
                <p>This is the abstract of the paper.</p>
            </div>
            <section>
                <h2>Introduction</h2>
                <p>This is the introduction paragraph.</p>
            </section>
        </article>
        </body></html>
        "#;
        let result = parse_html_to_text(html, false);
        assert!(result.contains("Test Paper Title"), "结果: {}", result);
        assert!(result.contains("Introduction"), "结果: {}", result);
        assert!(result.contains("introduction paragraph"), "结果: {}", result);
    }

    #[test]
    fn test_parse_html_with_math() {
        let html = r#"
        <html><body>
        <article class="ltx_document">
            <h1 class="ltx_title ltx_title_document">Math Paper</h1>
            <section>
                <h2>Methods</h2>
                <p>We define <math><annotation encoding="application/x-tex">f(x) = x^2</annotation></math> as the function.</p>
            </section>
        </article>
        </body></html>
        "#;
        let result = parse_html_to_text(html, false);
        assert!(result.contains("Math Paper"), "结果: {}", result);
        assert!(result.contains("$f(x) = x^2$"), "结果: {}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_and_convert_attention_paper() {
        // 1706.03762 (Attention Is All You Need)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ZenFlow/0.1.0")
            .build()
            .unwrap();
        let text = fetch_and_convert(&client, "1706.03762", false).await.unwrap();
        assert!(
            text.to_lowercase().contains("attention"),
            "应该包含 'Attention', 实际长度: {}",
            text.len()
        );
        assert!(text.len() > 1000, "应该包含大量文本, 实际长度: {}", text.len());
    }

    #[test]
    fn test_strip_references() {
        let html = r##"
        <article class="ltx_document">
            <h1>Test Paper</h1>
            <section>
                <h2>Introduction</h2>
                <p>Some text with a citation <a href="#bib.bib1">[1]</a>.</p>
            </section>
            <section class="ltx_bibliography">
                <h2>References</h2>
                <div class="ltx_bibitem">[1] Author, Title, 2020.</div>
            </section>
        </article>
        "##;
        let result = strip_references(html);
        assert!(!result.contains("ltx_bibliography"), "结果: {}", result);
        assert!(!result.contains("References"), "结果: {}", result);
        assert!(result.contains("Introduction"), "结果: {}", result);
        assert!(!result.contains("[1]"), "结果: {}", result);
    }

    #[test]
    fn test_parse_html_no_refs() {
        let html = r##"
        <html><body>
        <article class="ltx_document">
            <h1 class="ltx_title ltx_title_document">Paper Title</h1>
            <section>
                <h2>Intro</h2>
                <p>Content with ref <a href="#bib.bib2">[2]</a> inline.</p>
            </section>
            <section class="ltx_bibliography">
                <h2>References</h2>
                <p>[1] Ref A</p>
                <p>[2] Ref B</p>
            </section>
        </article>
        </body></html>
        "##;
        let result = parse_html_to_text(html, true);
        assert!(result.contains("Paper Title"), "结果: {}", result);
        assert!(result.contains("Intro"), "结果: {}", result);
        assert!(!result.contains("References"), "结果: {}", result);
    }
}
