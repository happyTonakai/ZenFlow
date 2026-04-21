//! LaTeX 到纯文本转换器
//!
//! 从 pylatexenc 和 arxiv-to-prompt 移植的核心逻辑。

use regex::Regex;
use std::collections::HashMap;

/// 将 LaTeX 源码转换为纯文本
pub fn convert_latex_to_text(latex: &str, no_refs: bool) -> String {
    let text = remove_comments(latex);
    let text = strip_preamble(&text);
    let text = if no_refs { strip_bibliography(&text) } else { text };
    let text = expand_macros(&text);
    let text = convert_environments(&text);
    let text = convert_macros(&text);
    let text = convert_accented_chars(&text);
    let text = convert_special_chars(&text);
    clean_whitespace(&text)
}

/// 移除 LaTeX 注释（尊重 \% 转义）
fn remove_comments(text: &str) -> String {
    // 移除 \iffalse...\fi 块
    let re = Regex::new(r"(?s)\\iffalse\b.*?\\fi\b").unwrap();
    let text = re.replace_all(text, "");

    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            continue;
        }
        result.push(strip_inline_comment(line));
    }
    result.join("\n")
}

/// 剥离行内注释
fn strip_inline_comment(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            i += 2; // 跳过转义的 \%
            continue;
        }
        if bytes[i] == b'%' {
            return line[..i].trim_end().to_string();
        }
        i += 1;
    }
    line.to_string()
}

/// 剥离 preamble（\documentclass 到 \begin{document} 之间）
fn strip_preamble(latex: &str) -> String {
    if let Some(pos) = latex.find("\\begin{document}") {
        let after_begin = &latex[pos + "\\begin{document}".len()..];
        // 移除 \end{document} 之后的内容
        if let Some(end_pos) = after_begin.find("\\end{document}") {
            return after_begin[..end_pos].to_string();
        }
        return after_begin.to_string();
    }
    latex.to_string()
}

/// 剥离参考文献部分
fn strip_bibliography(text: &str) -> String {
    let re = Regex::new(r"(?s)\\begin\{thebibliography\}.*?\\end\{thebibliography\}").unwrap();
    let result = re.replace_all(text, "");

    // 移除 \bibliography{...} 指令
    let re = Regex::new(r"\\bibliography\{[^}]*\}").unwrap();
    re.replace_all(&result, "").to_string()
}

/// 宏定义
struct MacroDef {
    name: String,
    num_args: usize,
    optional_default: Option<String>,
    body: String,
}

/// 查找匹配的花括号
fn find_matching_brace(text: &str, pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'{' {
        return None;
    }
    let mut depth = 0;
    let mut i = pos;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && (bytes[i + 1] == b'{' || bytes[i + 1] == b'}') {
            i += 2;
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 查找匹配的方括号
fn find_matching_bracket(text: &str, pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'[' {
        return None;
    }
    let mut depth = 0;
    let mut i = pos;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && (bytes[i + 1] == b'[' || bytes[i + 1] == b']') {
            i += 2;
            continue;
        }
        if bytes[i] == b'[' {
            depth += 1;
        } else if bytes[i] == b']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 解析宏定义
fn parse_macro_definitions(text: &str) -> (HashMap<String, MacroDef>, String) {
    let mut macros = HashMap::new();
    let mut regions_to_remove: Vec<(usize, usize)> = Vec::new();

    // \newcommand, \renewcommand, \providecommand
    let cmd_re = Regex::new(r"\\(newcommand|renewcommand|providecommand)\*?\s*").unwrap();
    for caps in cmd_re.captures_iter(text) {
        let m = caps.get(0).unwrap();
        let mut pos = m.end();

        // 跳过空白
        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
            pos += 1;
        }

        // 提取命令名
        let cmd_name = if pos < text.len() && text.as_bytes()[pos] == b'{' {
            match find_matching_brace(text, pos) {
                Some(close) => {
                    let name = text[pos + 1..close].trim().to_string();
                    pos = close + 1;
                    name
                }
                None => continue,
            }
        } else if pos < text.len() && text.as_bytes()[pos] == b'\\' {
            let name_re = Regex::new(r"\\([a-zA-Z@]+)").unwrap();
            if let Some(nm) = name_re.captures(&text[pos..]) {
                let name = format!("\\{}", nm.get(1).unwrap().as_str());
                pos += nm.get(0).unwrap().end();
                name
            } else {
                continue;
            }
        } else {
            continue;
        };

        // 跳过空白
        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
            pos += 1;
        }

        // 可选 [num_args]
        let mut num_args = 0;
        if pos < text.len() && text.as_bytes()[pos] == b'[' {
            if let Some(bracket_close) = find_matching_bracket(text, pos) {
                if let Ok(n) = text[pos + 1..bracket_close].trim().parse::<usize>() {
                    num_args = n;
                }
                pos = bracket_close + 1;
                while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
                    pos += 1;
                }
            }
        }

        // 可选 [default]
        let mut optional_default = None;
        if pos < text.len() && text.as_bytes()[pos] == b'[' {
            if let Some(bracket_close) = find_matching_bracket(text, pos) {
                optional_default = Some(text[pos + 1..bracket_close].to_string());
                pos = bracket_close + 1;
                while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
                    pos += 1;
                }
            }
        }

        // body in braces
        if pos >= text.len() || text.as_bytes()[pos] != b'{' {
            continue;
        }
        match find_matching_brace(text, pos) {
            Some(body_close) => {
                let body = text[pos + 1..body_close].to_string();
                macros.insert(cmd_name.clone(), MacroDef {
                    name: cmd_name,
                    num_args,
                    optional_default,
                    body,
                });
                let mut end = body_close + 1;
                if end < text.len() && text.as_bytes()[end] == b'\n' {
                    end += 1;
                }
                regions_to_remove.push((m.start(), end));
            }
            None => continue,
        }
    }

    // \DeclareMathOperator
    let decl_re = Regex::new(r"\\DeclareMathOperator(\*?)\s*").unwrap();
    for caps in decl_re.captures_iter(text) {
        let m = caps.get(0).unwrap();
        let starred = !caps.get(1).unwrap().as_str().is_empty();
        let mut pos = m.end();

        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
            pos += 1;
        }

        if pos >= text.len() || text.as_bytes()[pos] != b'{' {
            continue;
        }
        let close = match find_matching_brace(text, pos) {
            Some(c) => c,
            None => continue,
        };
        let mut cmd_name = text[pos + 1..close].trim().to_string();
        if !cmd_name.starts_with('\\') {
            cmd_name = format!("\\{}", cmd_name);
        }
        pos = close + 1;

        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
            pos += 1;
        }

        if pos >= text.len() || text.as_bytes()[pos] != b'{' {
            continue;
        }
        let body_close = match find_matching_brace(text, pos) {
            Some(c) => c,
            None => continue,
        };
        let op_text = &text[pos + 1..body_close];

        let body = if starred {
            format!("\\operatorname*{{{}}}", op_text)
        } else {
            format!("\\operatorname{{{}}}", op_text)
        };

        macros.insert(cmd_name.clone(), MacroDef {
            name: cmd_name,
            num_args: 0,
            optional_default: None,
            body,
        });

        let mut end = body_close + 1;
        if end < text.len() && text.as_bytes()[end] == b'\n' {
            end += 1;
        }
        regions_to_remove.push((m.start(), end));
    }

    // \def\cmd{body}
    let def_re = Regex::new(r"\\def\s*(\\[a-zA-Z@]+)\s*").unwrap();
    for caps in def_re.captures_iter(text) {
        let m = caps.get(0).unwrap();
        let cmd_name = caps.get(1).unwrap().as_str().to_string();
        let mut pos = m.end();

        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
            pos += 1;
        }

        if pos >= text.len() || text.as_bytes()[pos] != b'{' {
            continue;
        }
        match find_matching_brace(text, pos) {
            Some(body_close) => {
                let body = text[pos + 1..body_close].to_string();
                macros.insert(cmd_name.clone(), MacroDef {
                    name: cmd_name,
                    num_args: 0,
                    optional_default: None,
                    body,
                });
                let mut end = body_close + 1;
                if end < text.len() && text.as_bytes()[end] == b'\n' {
                    end += 1;
                }
                regions_to_remove.push((m.start(), end));
            }
            None => continue,
        }
    }

    // 移除定义区域
    regions_to_remove.sort_by_key(|&(s, _)| s);
    // 合并重叠区域
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in regions_to_remove {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }

    let mut cleaned = text.to_string();
    for &(s, e) in merged.iter().rev() {
        cleaned.replace_range(s..e, "");
    }

    (macros, cleaned)
}

/// 展开单个宏的所有用法
fn expand_single_macro(text: &str, mac: &MacroDef) -> String {
    // mac.name 如 "\R" 或 "\argmax"，需要匹配字面反斜杠
    // 注意：Rust regex 不支持 look-ahead，需要手动检查边界
    let name_escaped = regex::escape(&mac.name);
    // 先匹配 \cmd 后面是任何字符的位置
    let pattern = format!(r"{}\b", name_escaped);
    let re = match Regex::new(&pattern) {
        Ok(re) => re,
        Err(_) => return text.to_string(),
    };

    if mac.num_args == 0 {
        let mut result = text.to_string();
        let matches: Vec<_> = re.find_iter(text).collect();
        // 从后往前替换，避免偏移
        for m in matches.iter().rev() {
            // 检查 \cmd 后面是否紧跟字母（不允许扩展）
            let after = &text[m.end()..];
            let next_char = after.chars().next();
            if let Some(c) = next_char {
                if c.is_ascii_alphabetic() || c == '@' {
                    continue;
                }
            }
            result.replace_range(m.start()..m.end(), &mac.body);
        }
        return result;
    }

    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for caps in re.captures_iter(text) {
        let m = caps.get(0).unwrap();
        // 检查边界：\cmd 后面不能紧跟字母
        let after = &text[m.end()..];
        if let Some(c) = after.chars().next() {
            if c.is_ascii_alphabetic() || c == '@' {
                continue;
            }
        }
        let mut pos = m.end();

        // 跳过空白
        while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t' | b'\n') {
            pos += 1;
        }

        let mut args: Vec<String> = Vec::new();
        let has_optional = mac.optional_default.is_some();

        if has_optional {
            if pos < text.len() && text.as_bytes()[pos] == b'[' {
                if let Some(bracket_close) = find_matching_bracket(text, pos) {
                    args.push(text[pos + 1..bracket_close].to_string());
                    pos = bracket_close + 1;
                } else {
                    continue;
                }
            } else {
                args.push(mac.optional_default.clone().unwrap_or_default());
            }
        }

        let remaining = if has_optional { mac.num_args - 1 } else { mac.num_args };
        let mut success = true;

        for _ in 0..remaining {
            while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t' | b'\n') {
                pos += 1;
            }
            if pos >= text.len() || text.as_bytes()[pos] != b'{' {
                success = false;
                break;
            }
            match find_matching_brace(text, pos) {
                Some(brace_close) => {
                    args.push(text[pos + 1..brace_close].to_string());
                    pos = brace_close + 1;
                }
                None => {
                    success = false;
                    break;
                }
            }
        }

        if !success || args.len() != mac.num_args {
            continue;
        }

        let mut result = mac.body.clone();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("#{}", i + 1), arg);
        }

        replacements.push((m.start(), pos, result));
    }

    let mut result = text.to_string();
    for (start, end, replacement) in replacements.iter().rev() {
        result.replace_range(*start..*end, replacement);
    }
    result
}

/// 展开所有自定义宏
fn expand_macros(text: &str) -> String {
    let (macros, mut text) = parse_macro_definitions(text);

    if macros.is_empty() {
        return text;
    }

    // 迭代展开直到稳定（处理嵌套宏）
    for _ in 0..10 {
        let prev = text.clone();
        for mac in macros.values() {
            text = expand_single_macro(&text, mac);
        }
        if text == prev {
            break;
        }
    }

    text
}

/// 转换 LaTeX 环境为纯文本
fn convert_environments(text: &str) -> String {
    let mut result = text.to_string();

    // abstract 环境
    let re = Regex::new(r"(?s)\\begin\{abstract\}(.*?)\\end\{abstract\}").unwrap();
    result = re.replace_all(&result, "Abstract\n$1").to_string();

    // itemize 环境
    let re = Regex::new(r"(?s)\\begin\{itemize\}(.*?)\\end\{itemize\}").unwrap();
    result = re.replace_all(&result, |caps: &regex::Captures| {
        let content = caps.get(1).unwrap().as_str();
        let re_item = Regex::new(r"\\item\s*").unwrap();
        let items = re_item.replace_all(content, "\n- ");
        items.to_string()
    }).to_string();

    // enumerate 环境
    let re = Regex::new(r"(?s)\\begin\{enumerate\}(.*?)\\end\{enumerate\}").unwrap();
    result = re.replace_all(&result, |caps: &regex::Captures| {
        let content = caps.get(1).unwrap().as_str();
        let re_item = Regex::new(r"\\item\s*").unwrap();
        let mut counter = 1;
        let items = re_item.replace_all(content, |_caps: &regex::Captures| {
            let replacement = format!("\n{}. ", counter);
            counter += 1;
            replacement
        });
        items.to_string()
    }).to_string();

    // equation/align/gather 等数学环境 → 保留内容
    for env in &["equation", "equation*", "align", "align*", "gather", "gather*", "multline", "multline*"] {
        let pattern = format!(r"(?s)\\begin\{{{}}}(.*?)\\end\{{{}}}", regex::escape(env), regex::escape(env));
        let re = Regex::new(&pattern).unwrap();
        result = re.replace_all(&result, "$$ $1 $$").to_string();
    }

    // figure/table 环境 → 提取 caption
    for env in &["figure", "figure*", "table", "table*"] {
        let pattern = format!(r"(?s)\\begin\{{{}}}(.*?)\\end\{{{}}}", regex::escape(env), regex::escape(env));
        let re = Regex::new(&pattern).unwrap();
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let content = caps.get(1).unwrap().as_str();
            let caption_re = Regex::new(r"(?s)\\caption\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
            let mut captions = Vec::new();
            for cc in caption_re.captures_iter(content) {
                captions.push(cc.get(1).unwrap().as_str().to_string());
            }
            if captions.is_empty() {
                String::new()
            } else {
                captions.iter().map(|c| format!("Caption: {}", c)).collect::<Vec<_>>().join("\n")
            }
        }).to_string();
    }

    result
}

/// 转换常见的 LaTeX 宏为纯文本
fn convert_macros(text: &str) -> String {
    let mut result = text.to_string();

    // \label{...} → 移除
    let re = Regex::new(r"\\label\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \index{...} → 移除
    let re = Regex::new(r"\\index\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \footnote{...} → (note: ...)
    let re = Regex::new(r"(?s)\\footnote\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, " ($1)").to_string();

    // \section{...} → === ... ===
    let re = Regex::new(r"(?s)\\section\*?\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "\n\n=== $1 ===\n").to_string();

    // \subsection{...} → --- ... ---
    let re = Regex::new(r"(?s)\\subsection\*?\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "\n--- $1 ---\n").to_string();

    // \subsubsection{...} → ... ...
    let re = Regex::new(r"(?s)\\subsubsection\*?\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "\n$1\n").to_string();

    // \caption{...} → Caption: ...
    let re = Regex::new(r"(?s)\\caption\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "Caption: $1").to_string();

    // \textbf{...} → ...
    let re = Regex::new(r"(?s)\\textbf\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \textit{...} / \emph{...} → ...
    let re = Regex::new(r"(?s)\\textit\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();
    let re = Regex::new(r"(?s)\\emph\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \texttt{...} → ...
    let re = Regex::new(r"(?s)\\texttt\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \textsc{...} → ...
    let re = Regex::new(r"(?s)\\textsc\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \textsf{...} → ...
    let re = Regex::new(r"(?s)\\textsf\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \cite{...} → [ref]
    let re = Regex::new(r"\\cite\*?\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "[ref]").to_string();

    // \ref{...} → ?
    let re = Regex::new(r"\\ref\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "?").to_string();

    // \eqref{...} → (?)
    let re = Regex::new(r"\\eqref\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "(?)").to_string();

    // \url{...} → URL
    let re = Regex::new(r"\\url\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \href{url}{text} → text (url)
    let re = Regex::new(r"(?s)\\href\{([^}]*)\}\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$2 ($1)").to_string();

    // \item → -
    let re = Regex::new(r"\\item\s*").unwrap();
    result = re.replace_all(&result, "- ").to_string();

    // \title{...}
    let re = Regex::new(r"(?s)\\title\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \author{...}
    let re = Regex::new(r"(?s)\\author\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \date{...}
    let re = Regex::new(r"(?s)\\date\{((?:[^{}]|\{[^}]*\})*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \maketitle → 移除
    let re = Regex::new(r"\\maketitle\b").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \centering → 移除
    let re = Regex::new(r"\\centering\b").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \noindent → 移除
    let re = Regex::new(r"\\noindent\b").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \hspace{...}, \vspace{...} → 移除
    let re = Regex::new(r"\\[hv]space\*?\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \newpage, \clearpage, \cleardoublepage → 移除
    let re = Regex::new(r"\\(newpage|clearpage|cleardoublepage)\b").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \\ → 换行
    result = result.replace("\\\\", "\n");

    // \par → 换行
    let re = Regex::new(r"\\par\b").unwrap();
    result = re.replace_all(&result, "\n\n").to_string();

    // \quad, \qquad → 空格
    result = result.replace("\\qquad", "  ");
    result = result.replace("\\quad", " ");

    // \left, \right → 移除（数学分隔符修饰）
    let re = Regex::new(r"\\(left|right)\b").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \mathbb{X}, \mathcal{X}, \mathbf{X}, \mathit{X}, \mathrm{X} → 保留参数
    let re = Regex::new(r"\\math(bb|cal|bf|it|rm|frak|sf|tt)\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "$2").to_string();

    // \operatorname{...} → ...
    let re = Regex::new(r"\\operatorname\*?\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    // \overline{...}, \underline{...} → ...
    let re = Regex::new(r"\\(over|under)line\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "$2").to_string();

    // \sqrt[...]{...} / \sqrt{...}
    let re = Regex::new(r"\\sqrt(?:\[[^\]]*\])?\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "sqrt($1)").to_string();

    // \frac{a}{b} → (a/b)
    let re = Regex::new(r"\\frac\{([^}]*)\}\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "($1/$2)").to_string();

    // \begin{...} / \end{...} → 移除（剩余的环境标记）
    let re = Regex::new(r"\\(begin|end)\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \includegraphics[...]{...} → [Figure: filename]
    let re = Regex::new(r"\\includegraphics(?:\[[^\]]*\])?\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "[Figure: $1]").to_string();

    // \input{...} / \include{...} → 移除（如果展平后残留）
    let re = Regex::new(r"\\(input|include)\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \usepackage{...} → 移除（如果展平后残留）
    let re = Regex::new(r"\\usepackage(?:\[[^\]]*\])?\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \newcommand / \renewcommand / \def → 移除残留
    let re = Regex::new(r"\\(newcommand|renewcommand|providecommand|def)\*?\s*\\?[a-zA-Z@]+(?:\[[^\]]*\])*(?:\[[^\]]*\])?\s*\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \DeclareMathOperator → 移除残留
    let re = Regex::new(r"\\DeclareMathOperator\*?\s*\{[^}]*\}\s*\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \bibitem{...} → 移除
    let re = Regex::new(r"\\bibitem(?:\[[^\]]*\])?\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // \bibliographystyle{...} → 移除
    let re = Regex::new(r"\\bibliographystyle\{[^}]*\}").unwrap();
    result = re.replace_all(&result, "").to_string();

    // 残留的 \textXX{...} 格式化命令（通用兜底）
    let re = Regex::new(r"\\text[a-z]+\{([^}]*)\}").unwrap();
    result = re.replace_all(&result, "$1").to_string();

    result
}

/// 转换 LaTeX 重音字符为 Unicode
fn convert_accented_chars(text: &str) -> String {
    let accents: &[(&str, &str)] = &[
        // 急音符 \'{}
        ("\\'{a}", "á"), ("\\'{A}", "Á"), ("\\'{e}", "é"), ("\\'{E}", "É"),
        ("\\'{i}", "í"), ("\\'{I}", "Í"), ("\\'{o}", "ó"), ("\\'{O}", "Ó"),
        ("\\'{u}", "ú"), ("\\'{U}", "Ú"), ("\\'{y}", "ý"), ("\\'{Y}", "Ý"),
        ("\\'{c}", "ć"), ("\\'{C}", "Ć"), ("\\'{n}", "ń"), ("\\'{N}", "Ń"),
        ("\\'{s}", "ś"), ("\\'{S}", "Ś"), ("\\'{z}", "ź"), ("\\'{Z}", "Ź"),
        // 重音符 \`{}
        ("\\`{a}", "à"), ("\\`{A}", "À"), ("\\`{e}", "è"), ("\\`{E}", "È"),
        ("\\`{i}", "ì"), ("\\`{I}", "Ì"), ("\\`{o}", "ò"), ("\\`{O}", "Ò"),
        ("\\`{u}", "ù"), ("\\`{U}", "Ù"),
        // 抑扬符 \^{}
        ("\\^{a}", "â"), ("\\^{A}", "Â"), ("\\^{e}", "ê"), ("\\^{E}", "Ê"),
        ("\\^{i}", "î"), ("\\^{I}", "Î"), ("\\^{o}", "ô"), ("\\^{O}", "Ô"),
        ("\\^{u}", "û"), ("\\^{U}", "Û"), ("\\^{c}", "ĉ"), ("\\^{C}", "Ĉ"),
        // 分音符 \u{0022}{} (using unicode escape for quote)
        ("\u{5c}\u{22}{a}", "ä"), ("\u{5c}\u{22}{A}", "Ä"), ("\u{5c}\u{22}{e}", "ë"), ("\u{5c}\u{22}{E}", "Ë"),
        ("\u{5c}\u{22}{i}", "ï"), ("\u{5c}\u{22}{I}", "Ï"), ("\u{5c}\u{22}{o}", "ö"), ("\u{5c}\u{22}{O}", "Ö"),
        ("\u{5c}\u{22}{u}", "ü"), ("\u{5c}\u{22}{U}", "Ü"), ("\u{5c}\u{22}{y}", "ÿ"), ("\u{5c}\u{22}{Y}", "Ÿ"),
        // 波浪符 \~{}
        ("\\~{a}", "ã"), ("\\~{A}", "Ã"), ("\\~{n}", "ñ"), ("\\~{N}", "Ñ"),
        ("\\~{o}", "õ"), ("\\~{O}", "Õ"),
        // 软音符 \c{}
        ("\\c{c}", "ç"), ("\\c{C}", "Ç"), ("\\c{s}", "ş"), ("\\c{S}", "Ş"),
        // 变音符 \v{}
        ("\\v{c}", "č"), ("\\v{C}", "Č"), ("\\v{e}", "ě"), ("\\v{E}", "Ě"),
        ("\\v{r}", "ř"), ("\\v{R}", "Ř"), ("\\v{s}", "š"), ("\\v{S}", "Š"),
        ("\\v{z}", "ž"), ("\\v{Z}", "Ž"), ("\\v{n}", "ň"), ("\\v{N}", "Ň"),
        // 杠音符 \H{}
        ("\\H{o}", "ő"), ("\\H{O}", "Ő"), ("\\H{u}", "ű"), ("\\H{U}", "Ű"),
        // 圈符 \r{}
        ("\\r{a}", "å"), ("\\r{A}", "Å"), ("\\r{u}", "ů"), ("\\r{U}", "Ů"),
        // \t{}
        ("\\t{oo}", "o͡o"),
        // \d{}
        ("\\d{t}", "ṭ"),
        // 特殊字符
        ("\\ss", "ß"), ("\\SS", "SS"),
        ("\\ae", "æ"), ("\\AE", "Æ"),
        ("\\oe", "œ"), ("\\OE", "Œ"),
        ("\\aa", "å"), ("\\AA", "Å"),
        ("\\o", "ø"), ("\\O", "Ø"),
        ("\\i", "ı"), ("\\j", "ȷ"),
        ("\\l", "ł"), ("\\L", "Ł"),
        // 简写形式 \'e (无花括号)
    ];

    let mut result = text.to_string();

    // 先处理带花括号的版本
    for &(from, to) in accents {
        result = result.replace(from, to);
    }

    // 处理不带花括号的简写形式 \'e → é 等
    let shorthand_re = Regex::new(r#"\\(['`^"~Hcdr])\{?([a-zA-Z])\}?"#).unwrap();
    result = shorthand_re.replace_all(&result, |caps: &regex::Captures| {
        let accent = caps.get(1).unwrap().as_str();
        let ch = caps.get(2).unwrap().as_str();
        resolve_accent(accent, ch)
    }).to_string();

    result
}

fn resolve_accent(accent: &str, ch: &str) -> String {
    let lower = ch.to_lowercase();
    let is_upper = ch.chars().next().map_or(false, |c| c.is_uppercase());
    let c = if is_upper { ch } else { &lower };

    // 组合 accent + char 查找
    let q = "\u{22}";
    let key = format!("\\{}{{{}{}{}}}", accent, q, c, q);
    let key_bare = format!("\\{}{{{}}}", accent, c);

    // 分音符用 \" 其他用普通引号
    let table: &[(&str, &str)] = &[
        ("\\'{a}", "\u{e1}"), ("\\'{e}", "\u{e9}"), ("\\'{i}", "\u{ed}"), ("\\'{o}", "\u{f3}"), ("\\'{u}", "\u{fa}"), ("\\'{y}", "\u{fd}"),
        ("\\'{A}", "\u{c1}"), ("\\'{E}", "\u{c9}"), ("\\'{I}", "\u{cd}"), ("\\'{O}", "\u{d3}"), ("\\'{U}", "\u{da}"), ("\\'{Y}", "\u{dd}"),
        ("\\`{a}", "\u{e0}"), ("\\`{e}", "\u{e8}"), ("\\`{i}", "\u{ec}"), ("\\`{o}", "\u{f2}"), ("\\`{u}", "\u{f9}"),
        ("\\`{A}", "\u{c0}"), ("\\`{E}", "\u{c8}"), ("\\`{I}", "\u{cc}"), ("\\`{O}", "\u{d2}"), ("\\`{U}", "\u{d9}"),
        ("\\^{a}", "\u{e2}"), ("\\^{e}", "\u{ea}"), ("\\^{i}", "\u{ee}"), ("\\^{o}", "\u{f4}"), ("\\^{u}", "\u{fb}"),
        ("\\^{A}", "\u{c2}"), ("\\^{E}", "\u{ca}"), ("\\^{I}", "\u{ce}"), ("\\^{O}", "\u{d4}"), ("\\^{U}", "\u{db}"),
        ("\\~{a}", "\u{e3}"), ("\\~{n}", "\u{f1}"), ("\\~{o}", "\u{f5}"),
        ("\\~{A}", "\u{c3}"), ("\\~{N}", "\u{d1}"), ("\\~{O}", "\u{d5}"),
    ];

    // 先查普通 accent
    for &(k, v) in table {
        if k == key_bare {
            return v.to_string();
        }
    }

    // 分音符 (需要引号)
    let diaeresis: &[(&str, &str)] = &[
        ("\\\"\u{7b}a\u{7d}", "\u{e4}"), ("\\\"\u{7b}e\u{7d}", "\u{eb}"), ("\\\"\u{7b}i\u{7d}", "\u{ef}"),
        ("\\\"\u{7b}o\u{7d}", "\u{f6}"), ("\\\"\u{7b}u\u{7d}", "\u{fc}"), ("\\\"\u{7b}y\u{7d}", "\u{ff}"),
        ("\\\"\u{7b}A\u{7d}", "\u{c4}"), ("\\\"\u{7b}E\u{7d}", "\u{cb}"), ("\\\"\u{7b}I\u{7d}", "\u{cf}"),
        ("\\\"\u{7b}O\u{7d}", "\u{d6}"), ("\\\"\u{7b}U\u{7d}", "\u{dc}"),
    ];

    for &(k, v) in diaeresis {
        if k == key {
            return v.to_string();
        }
    }

    ch.to_string()
}

/// 转换 LaTeX 特殊字符
fn convert_special_chars(text: &str) -> String {
    let mut result = text.to_string();

    let specials: &[(&str, &str)] = &[
        ("\\&", "&"), ("\\$", "$"), ("\\%", "%"), ("\\_", "_"),
        ("\\#", "#"), ("\u{5c}\u{7b}", "{"), ("\u{5c}\u{7d}", "}"),
        ("\\ldots", "..."), ("\\cdots", "⋯"), ("\\vdots", "⋮"), ("\\ddots", "⋱"),
        ("\\dots", "..."), ("\\textellipsis", "..."),
        ("\\textasciitilde", "~"), ("\\textasciicircum", "^"),
        ("\\textbackslash", "\\"),
        ("\\textbar", "|"), ("\\textless", "<"), ("\\textgreater", ">"),
        ("\\copyright", "©"), ("\\textregistered", "®"), ("\\texttrademark", "™"),
        ("\\P", "¶"), ("\\S", "§"),
        ("\\dag", "†"), ("\\ddag", "‡"),
        ("\\bullet", "•"), ("\\circ", "∘"), ("\\degree", "°"),
        ("\\times", "×"), ("\\div", "÷"), ("\\pm", "±"), ("\\mp", "∓"),
        ("\\leq", "≤"), ("\\geq", "≥"), ("\\neq", "≠"), ("\\approx", "≈"),
        ("\\equiv", "≡"), ("\\sim", "∼"), ("\\simeq", "≃"), ("\\cong", "≅"),
        ("\\propto", "∝"),
        ("\\infty", "∞"), ("\\partial", "∂"), ("\\nabla", "∇"),
        ("\\sum", "∑"), ("\\prod", "∏"), ("\\int", "∫"),
        ("\\in", "∈"), ("\\notin", "∉"), ("\\subset", "⊂"), ("\\supset", "⊃"),
        ("\\cup", "∪"), ("cap", "∩"), ("\\emptyset", "∅"),
        ("\\forall", "∀"), ("\\exists", "∃"),
        ("\\rightarrow", "→"), ("\\leftarrow", "←"), ("\\Rightarrow", "⇒"), ("\\Leftarrow", "⇐"),
        ("\\leftrightarrow", "↔"), ("\\Leftrightarrow", "⇔"), ("\\uparrow", "↑"), ("\\downarrow", "↓"),
        ("\\alpha", "α"), ("\\beta", "β"), ("\\gamma", "γ"), ("\\delta", "δ"),
        ("\\epsilon", "ε"), ("\\varepsilon", "ε"), ("\\zeta", "ζ"), ("\\eta", "η"),
        ("\\theta", "θ"), ("\\iota", "ι"), ("\\kappa", "κ"), ("\\lambda", "λ"),
        ("\\mu", "μ"), ("\\nu", "ν"), ("\\xi", "ξ"), ("\\pi", "π"),
        ("\\rho", "ρ"), ("\\sigma", "σ"), ("\\tau", "τ"), ("\\upsilon", "υ"),
        ("\\phi", "φ"), ("\\varphi", "φ"), ("\\chi", "χ"), ("\\psi", "ψ"), ("\\omega", "ω"),
        ("\\Gamma", "Γ"), ("\\Delta", "Δ"), ("\\Theta", "Θ"), ("\\Lambda", "Λ"),
        ("\\Xi", "Ξ"), ("\\Pi", "Π"), ("\\Sigma", "Σ"), ("\\Phi", "Φ"),
        ("\\Psi", "Ψ"), ("\\Omega", "Ω"),
        // 空格相关
        ("~", " "),  // 不换行空格 → 普通空格
        ("\\,", " "), ("\\;", " "), ("\\!", ""),
    ];

    for &(from, to) in specials {
        result = result.replace(from, to);
    }

    result
}

/// 清理多余空白
fn clean_whitespace(text: &str) -> String {
    let re = Regex::new(r"[ \t]+").unwrap();
    let text = re.replace_all(text, " ");

    let re = Regex::new(r"\n{3,}").unwrap();
    let text = re.replace_all(&text, "\n\n");

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_comments_line() {
        assert_eq!(remove_comments("hello % comment\nworld"), "hello\nworld");
    }

    #[test]
    fn test_remove_comments_escaped_percent() {
        assert_eq!(remove_comments("100\\% of things"), "100\\% of things");
    }

    #[test]
    fn test_remove_comments_pure_comment_line() {
        assert_eq!(remove_comments("% full comment\nvisible"), "visible");
    }

    #[test]
    fn test_remove_iffalse_block() {
        let input = "before\n\\iffalse\nhidden block\n\\fi\nafter";
        let result = remove_comments(input);
        assert!(!result.contains("hidden block"), "结果: {}", result);
        assert!(result.contains("before"), "结果: {}", result);
        assert!(result.contains("after"), "结果: {}", result);
    }

    #[test]
    fn test_strip_preamble() {
        let input = "\\documentclass{article}\n\\title{Test}\n\\begin{document}\nHello World\n\\end{document}";
        let result = strip_preamble(input);
        assert!(result.trim().starts_with("Hello World"), "结果: {}", result);
        assert!(!result.contains("documentclass"), "结果: {}", result);
    }

    #[test]
    fn test_strip_preamble_no_document() {
        let input = "Just some text without preamble";
        assert_eq!(strip_preamble(input), input);
    }

    #[test]
    fn test_strip_bibliography() {
        let input = "Main text.\n\\begin{thebibliography}{99}\n\\bibitem{ref1} Some ref\n\\end{thebibliography}\nEnd";
        let result = strip_bibliography(input);
        assert!(!result.contains("thebibliography"), "结果: {}", result);
        assert!(result.contains("Main text."), "结果: {}", result);
        assert!(result.contains("End"), "结果: {}", result);
    }

    #[test]
    fn test_strip_bibliography_command() {
        let input = "Text\n\\bibliography{refs}";
        let result = strip_bibliography(input);
        assert!(!result.contains("\\bibliography"), "结果: {}", result);
        assert!(result.contains("Text"), "结果: {}", result);
    }

    #[test]
    fn test_expand_macros_simple() {
        let input = "\\newcommand{\\R}{\\mathbb{R}}\nConsider $\\R^n$";
        let result = expand_macros(input);
        assert!(result.contains("\\mathbb{R}^n"), "结果: {}", result);
        assert!(!result.contains("\\newcommand"), "结果: {}", result);
    }

    #[test]
    fn test_expand_macros_with_args() {
        let input = "\\newcommand{\\norm}[1]{\\|#1\\|}\n$\\norm{x}$";
        let result = expand_macros(input);
        assert!(result.contains("\\|x\\|"), "结果: {}", result);
    }

    #[test]
    fn test_expand_macros_with_optional() {
        let input = "\\newcommand{\\hello}[2][world]{Hello #1 #2}\n\\hello{there} and \\hello[universe]{there}";
        let result = expand_macros(input);
        assert!(result.contains("Hello world there"), "结果: {}", result);
        assert!(result.contains("Hello universe there"), "结果: {}", result);
    }

    #[test]
    fn test_expand_def_macro() {
        let input = "\\def\\mybar{BAR}\nText \\mybar here";
        let result = expand_macros(input);
        assert!(result.contains("BAR"), "结果: {}", result);
        assert!(!result.contains("\\def"), "结果: {}", result);
    }

    #[test]
    fn test_expand_declare_math_operator() {
        let input = "\\DeclareMathOperator{\\argmax}{arg\\,max}\nWe use $\\argmax$";
        let result = expand_macros(input);
        assert!(result.contains("\\operatorname{arg\\,max}"), "结果: {}", result);
    }

    #[test]
    fn test_convert_environment_abstract() {
        let input = "\\begin{abstract}\nThis is the abstract.\n\\end{abstract}";
        let result = convert_environments(input);
        assert!(result.contains("Abstract"), "结果: {}", result);
        assert!(result.contains("This is the abstract."), "结果: {}", result);
    }

    #[test]
    fn test_convert_environment_itemize() {
        let input = "\\begin{itemize}\n\\item First\n\\item Second\n\\end{itemize}";
        let result = convert_environments(input);
        assert!(result.contains("First"), "结果: {}", result);
        assert!(result.contains("Second"), "结果: {}", result);
    }

    #[test]
    fn test_convert_macros_textbf() {
        let result = convert_macros("\\textbf{bold text}");
        assert_eq!(result.trim(), "bold text");
    }

    #[test]
    fn test_convert_macros_textit() {
        let result = convert_macros("\\textit{italic text}");
        assert_eq!(result.trim(), "italic text");
    }

    #[test]
    fn test_convert_macros_emph() {
        let result = convert_macros("\\emph{emphasized}");
        assert_eq!(result.trim(), "emphasized");
    }

    #[test]
    fn test_convert_macros_section() {
        let result = convert_macros("\\section{Introduction}");
        assert!(result.contains("Introduction"), "结果: {}", result);
    }

    #[test]
    fn test_convert_macros_cite() {
        let result = convert_macros("As shown in \\cite{smith2020}.");
        assert!(!result.contains("\\cite"), "结果: {}", result);
        assert!(result.contains("[ref]"), "结果: {}", result);
    }

    #[test]
    fn test_convert_macros_ref() {
        let result = convert_macros("See \\ref{fig:1}.");
        assert!(!result.contains("\\ref"), "结果: {}", result);
        assert!(result.contains("?"), "结果: {}", result);
    }

    #[test]
    fn test_convert_macros_label_removed() {
        let result = convert_macros("Text \\label{sec:intro} more");
        assert!(!result.contains("\\label"), "结果: {}", result);
        assert!(result.contains("Text"), "结果: {}", result);
    }

    #[test]
    fn test_convert_accented_chars() {
        assert_eq!(convert_accented_chars("\\'e"), "é");
        assert_eq!(convert_accented_chars("\u{5c}\u{22}{u}"), "ü");
        assert_eq!(convert_accented_chars("\\^{a}"), "â");
        assert_eq!(convert_accented_chars("\\ss"), "ß");
        assert_eq!(convert_accented_chars("\\ae"), "æ");
    }

    #[test]
    fn test_convert_special_chars() {
        assert_eq!(convert_special_chars("\\&"), "&");
        assert_eq!(convert_special_chars("\\%"), "%");
        assert_eq!(convert_special_chars("\\ldots"), "...");
        assert_eq!(convert_special_chars("\\rightarrow"), "→");
        assert_eq!(convert_special_chars("\\alpha"), "α");
    }

    #[test]
    fn test_convert_inline_math_preserved() {
        let result = convert_latex_to_text("Let $x = 1$ be a solution.", false);
        assert!(result.contains("$"), "结果: {}", result);
    }

    #[test]
    fn test_full_conversion_sample() {
        let input = r#"
\documentclass{article}
\usepackage{amsmath}
\newcommand{\R}{\mathbb{R}}
\begin{document}
\begin{abstract}
We study the properties of $\R^n$.
\end{abstract}
\section{Introduction}
Let $f: \R \to \R$ be a continuous function \cite{ref1}.
\textbf{Main result:} $f$ is bounded.
\begin{thebibliography}{9}
\bibitem{ref1} Author, Title, 2020.
\end{thebibliography}
\end{document}
"#;
        let result = convert_latex_to_text(input, true);
        assert!(result.contains("Introduction"), "结果: {}", result);
        assert!(result.contains("bounded"), "结果: {}", result);
        assert!(result.contains("We study"), "结果: {}", result);
        assert!(!result.contains("documentclass"), "结果: {}", result);
        assert!(!result.contains("thebibliography"), "结果: {}", result);
        assert!(!result.contains("\\textbf"), "结果: {}", result);
        assert!(!result.contains("\\section{"), "结果: {}", result);
    }

    #[test]
    fn test_clean_whitespace() {
        let input = "  hello   world  \n\n\n\nextra";
        let result = clean_whitespace(input);
        assert!(!result.contains("   "), "结果: {}", result);
        assert!(!result.contains("\n\n\n"), "结果: {}", result);
    }

    #[test]
    fn test_convert_frac() {
        let result = convert_macros("\\frac{1}{2}");
        assert!(result.contains("(1/2)"), "结果: {}", result);
    }

    #[test]
    fn test_convert_sqrt() {
        let result = convert_macros("\\sqrt{x}");
        assert!(result.contains("sqrt(x)"), "结果: {}", result);
    }

    #[test]
    fn test_find_matching_brace() {
        let text = "{hello {nested} world}";
        assert_eq!(find_matching_brace(text, 0), Some(21));
        assert_eq!(find_matching_brace(text, 7), Some(14));
    }

    #[test]
    fn test_find_matching_bracket() {
        let text = "[hello [nested] world]";
        assert_eq!(find_matching_bracket(text, 0), Some(21));
    }
}
