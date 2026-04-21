//! 下载并处理 arXiv TeX 源码

use flate2::read::GzDecoder;
use regex::Regex;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

/// 下载并解压 arXiv TeX 源码到指定目录
pub async fn download_and_extract(
    client: &reqwest::Client,
    arxiv_id: &str,
    output_dir: &Path,
) -> anyhow::Result<()> {
    // 如果已经解压过，跳过
    if output_dir.exists() && find_main_tex(output_dir).is_ok() {
        tracing::info!("TeX 源码已缓存: {:?}", output_dir);
        return Ok(());
    }

    let url = format!("https://arxiv.org/e-print/{}", arxiv_id);
    tracing::info!("下载 TeX 源码: {}", url);

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("下载 TeX 源码失败: HTTP {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    std::fs::create_dir_all(output_dir)?;

    // 尝试 gzip 解压 + tar 解包
    let cursor = std::io::Cursor::new(&bytes);
    let mut decoder = GzDecoder::new(cursor);
    let mut tar_bytes = Vec::new();
    match decoder.read_to_end(&mut tar_bytes) {
        Ok(_) if !tar_bytes.is_empty() => {
            let mut archive = Archive::new(std::io::Cursor::new(tar_bytes));
            extract_tar_safely(&mut archive, output_dir)?;
        }
        _ => {
            // 不是 gzip 格式（或空解压结果），尝试直接作为 tar 解包
            let cursor = std::io::Cursor::new(&bytes);
            let mut archive = Archive::new(cursor);
            if let Err(e) = extract_tar_safely(&mut archive, output_dir) {
                // 也不是 tar，可能是单个 .tex 文件
                tracing::warn!("不是 tar.gz 格式，尝试作为单文件处理: {}", e);
                let tex_path = output_dir.join("main.tex");
                std::fs::create_dir_all(output_dir)?;
                std::fs::write(&tex_path, &bytes)?;
            }
        }
    }

    Ok(())
}

/// 安全地解包 tar 归档，防止路径穿越攻击
fn extract_tar_safely(archive: &mut Archive<impl std::io::Read>, output_dir: &Path) -> anyhow::Result<()> {
    let entries = archive.entries()?;
    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;
        let path_str = path.to_string_lossy();

        if !is_safe_path(&path_str) {
            anyhow::bail!("不安全的归档路径: {}", path_str);
        }

        entry.unpack_in(output_dir)?;
    }
    Ok(())
}

/// 检查路径是否安全（无路径穿越、无绝对路径）
fn is_safe_path(path: &str) -> bool {
    if path.starts_with('/') || path.contains("..") {
        return false;
    }
    true
}

/// 查找主 .tex 文件
///
/// 两轮搜索：
/// 1. 先检查常见文件名（main.tex, paper.tex, index.tex）
/// 2. 找包含 \documentclass 的最长 .tex 文件
pub fn find_main_tex(dir: &Path) -> anyhow::Result<PathBuf> {
    let common_names = ["main.tex", "paper.tex", "index.tex"];

    // 第一轮：常见文件名
    for name in &common_names {
        for entry in walkdir(dir) {
            if entry.file_name().map_or(false, |f| f == *name) {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    if content.contains("\\documentclass") || content.contains("\\documentstyle") {
                        return Ok(entry);
                    }
                }
            }
        }
    }

    // 第二轮：最长的包含 \documentclass 的文件
    let mut best: Option<(PathBuf, usize)> = None;
    for entry in walkdir(dir) {
        if entry.extension().map_or(false, |e| e == "tex") {
            if let Ok(content) = std::fs::read_to_string(&entry) {
                if content.contains("\\documentclass") || content.contains("\\documentstyle") {
                    let line_count = content.lines().count();
                    if best.as_ref().map_or(true, |(_, best_lines)| line_count > *best_lines) {
                        best = Some((entry, line_count));
                    }
                }
            }
        }
    }

    best.map(|(path, _)| path)
        .ok_or_else(|| anyhow::anyhow!("未找到主 .tex 文件（包含 \\documentclass）"))
}

/// 递归遍历目录，返回所有文件路径
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}

/// 展平多个 .tex 文件为单个字符串
///
/// 解析 \input{} 和 \include{} 指令，递归替换为文件内容。
pub fn flatten_tex_content(dir: &Path, main_file: &Path) -> anyhow::Result<String> {
    let mut visited = HashSet::new();
    flatten_file(dir, main_file, &mut visited)
}

fn flatten_file(dir: &Path, file_path: &Path, visited: &mut HashSet<PathBuf>) -> anyhow::Result<String> {
    let canonical = file_path.canonicalize().unwrap_or_else(|_| file_path.to_path_buf());
    if visited.contains(&canonical) {
        return Ok(String::new());
    }
    visited.insert(canonical);

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("无法读取文件 {:?}: {}", file_path, e))?;

    let re = Regex::new(r"\\(?:input|include)\{([^}]+)\}")?;
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;

    for caps in re.captures_iter(&content) {
        let full_match = caps.get(0).unwrap();
        let input_name = caps.get(1).unwrap().as_str().trim();

        // 检查是否被注释
        let line_start = content[..full_match.start()].rfind('\n').map_or(0, |i| i + 1);
        let line_prefix = &content[line_start..full_match.start()];
        if is_commented_out(line_prefix) {
            continue;
        }

        result.push_str(&content[last_end..full_match.start()]);

        // 查找被引用的文件
        if let Some(resolved) = resolve_input_path(dir, input_name) {
            match flatten_file(dir, &resolved, visited) {
                Ok(nested) => result.push_str(&nested),
                Err(e) => {
                    tracing::warn!("无法解析 \\input{{{}}}: {}", input_name, e);
                    // 保留原始指令
                    result.push_str(full_match.as_str());
                }
            }
        } else {
            // 文件不存在，保留原始指令
            result.push_str(full_match.as_str());
        }

        last_end = full_match.end();
    }

    result.push_str(&content[last_end..]);
    Ok(result)
}

/// 检查行前缀中是否有未转义的 %
fn is_commented_out(prefix: &str) -> bool {
    let mut i = 0;
    let bytes = prefix.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // 检查前面是否有奇数个反斜杠（转义）
            let mut backslashes = 0;
            let mut j = i;
            while j > 0 {
                j -= 1;
                if bytes[j] == b'\\' {
                    backslashes += 1;
                } else {
                    break;
                }
            }
            if backslashes % 2 == 0 {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// 解析 \input 引用的文件路径
fn resolve_input_path(base_dir: &Path, input_name: &str) -> Option<PathBuf> {
    // 尝试 filename.tex
    let with_ext = if input_name.ends_with(".tex") {
        base_dir.join(input_name)
    } else {
        base_dir.join(format!("{}.tex", input_name))
    };
    if with_ext.exists() {
        return Some(with_ext);
    }

    // 尝试不带 .tex 扩展名
    if !input_name.ends_with(".tex") {
        let bare = base_dir.join(input_name);
        if bare.exists() {
            return Some(bare);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_is_safe_path() {
        assert!(is_safe_path("paper.tex"));
        assert!(is_safe_path("sections/intro.tex"));
        assert!(!is_safe_path("../etc/passwd"));
        assert!(!is_safe_path("/absolute/path"));
        assert!(!is_safe_path("a/../b.tex"));
    }

    #[test]
    fn test_find_main_tex_common_names() {
        let dir = tempfile::tempdir().unwrap();
        let main_path = dir.path().join("main.tex");
        fs::write(&main_path, "\\documentclass{article}\n\\begin{document}\nHello\n\\end{document}").unwrap();
        fs::write(dir.path().join("other.tex"), "% helper file").unwrap();

        let found = find_main_tex(dir.path()).unwrap();
        assert_eq!(found, main_path);
    }

    #[test]
    fn test_find_main_tex_longest_file() {
        let dir = tempdir();
        // 短文件有 \documentclass
        fs::write(
            dir.path().join("short.tex"),
            "\\documentclass{article}\n\\begin{document}\nHi\n\\end{document}",
        ).unwrap();
        // 长文件有 \documentclass
        let long_content = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\nLong content\nMore lines\nEven more\n\\end{document}";
        fs::write(dir.path().join("long.tex"), long_content).unwrap();

        let found = find_main_tex(dir.path()).unwrap();
        assert_eq!(found, dir.path().join("long.tex"));
    }

    #[test]
    fn test_flatten_single_file() {
        let dir = tempdir();
        let main = dir.path().join("main.tex");
        fs::write(&main, "\\documentclass{article}\nHello World").unwrap();

        let result = flatten_tex_content(dir.path(), &main).unwrap();
        assert!(result.contains("Hello World"));
        assert!(result.contains("\\documentclass"));
    }

    #[test]
    fn test_flatten_with_input() {
        let dir = tempdir();
        let main = dir.path().join("main.tex");
        let intro = dir.path().join("intro.tex");

        fs::write(&main, "\\documentclass{article}\n\\input{intro}\nEnd").unwrap();
        fs::write(&intro, "Introduction content here").unwrap();

        let result = flatten_tex_content(dir.path(), &main).unwrap();
        assert!(result.contains("Introduction content here"), "结果: {}", result);
        assert!(result.contains("End"), "结果: {}", result);
    }

    #[test]
    fn test_flatten_circular_input() {
        let dir = tempdir();
        let a = dir.path().join("a.tex");
        let b = dir.path().join("b.tex");

        fs::write(&a, "\\documentclass{article}\n\\input{b}").unwrap();
        fs::write(&b, "From B\n\\input{a}").unwrap();

        // 不应该死循环
        let result = flatten_tex_content(dir.path(), &a).unwrap();
        assert!(result.contains("From B"));
    }

    #[test]
    fn test_flatten_commented_input() {
        let dir = tempdir();
        let main = dir.path().join("main.tex");
        let skip = dir.path().join("skip.tex");

        fs::write(&main, "\\documentclass{article}\n% \\input{skip}\nDone").unwrap();
        fs::write(&skip, "SHOULD NOT APPEAR").unwrap();

        let result = flatten_tex_content(dir.path(), &main).unwrap();
        assert!(!result.contains("SHOULD NOT APPEAR"), "结果: {}", result);
        assert!(result.contains("Done"), "结果: {}", result);
    }

    #[test]
    fn test_is_commented_out() {
        assert!(is_commented_out("% "));
        assert!(is_commented_out("  % "));
        assert!(!is_commented_out("text "));
        assert!(!is_commented_out("\\% ")); // escaped %
    }

    #[test]
    fn test_resolve_input_path() {
        let dir = tempdir();
        fs::write(dir.path().join("intro.tex"), "content").unwrap();

        assert!(resolve_input_path(dir.path(), "intro").is_some());
        assert!(resolve_input_path(dir.path(), "intro.tex").is_some());
        assert!(resolve_input_path(dir.path(), "nonexistent").is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_download_and_extract_single_tex() {
        // 1604.03121 - 单一 tex 文件
        let dir = tempdir();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("ZenFlow/0.1.0")
            .build()
            .unwrap();

        download_and_extract(&client, "1604.03121", dir.path()).await.unwrap();

        let main_tex = find_main_tex(dir.path()).unwrap();
        let content = fs::read_to_string(&main_tex).unwrap();
        assert!(content.contains("\\documentclass"), "应该包含 \\documentclass");
    }

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }
}
