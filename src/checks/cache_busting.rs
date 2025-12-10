//! Check that README image links use cache-busting query parameters.

use super::{CheckResult, Severity};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Image extensions to check for.
const IMAGE_EXTENSIONS: &[&str] = &[".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"];

/// Check README files for image links without cache-busting.
pub fn check(project_dir: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let mut checked_files = HashSet::new();

    // Check common README file names (dedupe for case-insensitive filesystems)
    let readme_names = ["README.md", "readme.md", "Readme.md"];

    for name in readme_names {
        let readme_path = project_dir.join(name);
        if readme_path.exists() {
            // Use canonical path to deduplicate on case-insensitive filesystems
            if let Ok(canonical) = readme_path.canonicalize() {
                if checked_files.insert(canonical) {
                    results.extend(check_readme(&readme_path));
                }
            }
        }
    }

    // Also check docs directory
    let docs_dir = project_dir.join("docs");
    if docs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&docs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(canonical) = path.canonicalize() {
                        if checked_files.insert(canonical) {
                            results.extend(check_readme(&path));
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        results.push(CheckResult::pass(
            "cache-busting",
            "No markdown files with images found",
        ));
    }

    results
}

fn check_readme(file_path: &Path) -> Vec<CheckResult> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return vec![
                CheckResult::fail(
                    "cache-busting",
                    Severity::Warning,
                    &format!("Read error: {e}"),
                )
                .with_file(&file_path.display().to_string()),
            ];
        }
    };

    let mut results = Vec::new();
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1;

        // Find image references: ![alt](path) or <img src="path">
        let image_links = extract_image_links(line);

        for link in image_links {
            // Skip external URLs
            if link.starts_with("http://") || link.starts_with("https://") {
                continue;
            }

            // Check if it's a local image path
            let is_image = IMAGE_EXTENSIONS
                .iter()
                .any(|ext| link.to_lowercase().contains(ext));

            if !is_image {
                continue;
            }

            // Check for cache-busting parameter
            if !has_cache_busting(&link) {
                results.push(
                    CheckResult::fail(
                        "cache-busting",
                        Severity::Warning,
                        &format!("{file_name}: Image link without cache-busting: {link}"),
                    )
                    .with_file(&file_path.display().to_string())
                    .with_line(line_number)
                    .with_fix(&format!(
                        "Add cache-busting parameter: {}?v=<version> or {}?ts=<timestamp>",
                        link, link
                    )),
                );
            }
        }
    }

    // If no issues found, return a pass
    if results.is_empty() {
        results.push(
            CheckResult::pass(
                "cache-busting",
                &format!("{file_name}: All image links have cache-busting"),
            )
            .with_file(&file_path.display().to_string()),
        );
    }

    results
}

fn extract_image_links(line: &str) -> Vec<String> {
    let mut links = Vec::new();

    // Markdown image syntax: ![alt](path)
    let mut remaining = line;
    while let Some(start) = remaining.find("![") {
        let after_alt = &remaining[start + 2..];
        if let Some(paren_start) = after_alt.find("](") {
            let path_start = &after_alt[paren_start + 2..];
            if let Some(paren_end) = path_start.find(')') {
                links.push(path_start[..paren_end].to_string());
            }
        }
        remaining = &remaining[start + 2..];
    }

    // HTML img syntax: <img src="path">
    remaining = line;
    while let Some(start) = remaining.find("src=\"") {
        let path_start = &remaining[start + 5..];
        if let Some(quote_end) = path_start.find('"') {
            links.push(path_start[..quote_end].to_string());
        }
        remaining = &remaining[start + 5..];
    }

    // Also handle single quotes
    remaining = line;
    while let Some(start) = remaining.find("src='") {
        let path_start = &remaining[start + 5..];
        if let Some(quote_end) = path_start.find('\'') {
            links.push(path_start[..quote_end].to_string());
        }
        remaining = &remaining[start + 5..];
    }

    links
}

fn has_cache_busting(link: &str) -> bool {
    // Check for common cache-busting patterns
    let patterns = ["?v=", "?ts=", "?t=", "?version=", "?hash=", "&v=", "&ts="];
    patterns.iter().any(|p| link.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detects_missing_cache_busting() {
        let temp = TempDir::new().unwrap();
        let readme = temp.path().join("README.md");
        fs::write(
            &readme,
            r#"
# My Project

![Screenshot](./images/screenshot.png)
"#,
        )
        .unwrap();

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("screenshot.png"));
    }

    #[test]
    fn test_passes_with_cache_busting() {
        let temp = TempDir::new().unwrap();
        let readme = temp.path().join("README.md");
        fs::write(
            &readme,
            r#"
# My Project

![Screenshot](./images/screenshot.png?v=1.0.0)
"#,
        )
        .unwrap();

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_passes_with_timestamp() {
        let temp = TempDir::new().unwrap();
        let readme = temp.path().join("README.md");
        fs::write(
            &readme,
            r#"
# My Project

![Screenshot](./images/screenshot.png?ts=1699999999)
"#,
        )
        .unwrap();

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_ignores_external_urls() {
        let temp = TempDir::new().unwrap();
        let readme = temp.path().join("README.md");
        fs::write(
            &readme,
            r#"
# My Project

![Badge](https://example.com/badge.png)
"#,
        )
        .unwrap();

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_detects_html_img() {
        let temp = TempDir::new().unwrap();
        let readme = temp.path().join("README.md");
        fs::write(
            &readme,
            r#"
# My Project

<img src="./images/logo.png" alt="Logo">
"#,
        )
        .unwrap();

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_extract_image_links() {
        let line = "![Alt](./img.png) and <img src=\"./other.jpg\">";
        let links = extract_image_links(line);
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"./img.png".to_string()));
        assert!(links.contains(&"./other.jpg".to_string()));
    }
}
