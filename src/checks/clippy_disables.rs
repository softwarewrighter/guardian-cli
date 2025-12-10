//! Check for clippy lint suppressions in source code.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Patterns that suppress clippy or rustc lints.
const SUPPRESS_PATTERNS: &[&str] = &[
    "#[allow(",
    "#![allow(",
    "#[cfg_attr(", // May contain allow
];

/// Allowed suppressions (legitimate uses).
const ALLOWED_SUPPRESSIONS: &[&str] = &[
    "dead_code", // Often needed during development
    "unused",    // Often needed during development
];

/// Check for clippy disable patterns in all Rust source files.
pub fn check(project_dir: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let src_dir = project_dir.join("src");

    if !src_dir.exists() {
        return results;
    }

    check_directory(&src_dir, &mut results);
    results
}

fn check_directory(dir: &Path, results: &mut Vec<CheckResult>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            check_directory(&path, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            results.extend(check_file(&path));
        }
    }
}

fn check_file(file_path: &Path) -> Vec<CheckResult> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return vec![
                CheckResult::fail(
                    "clippy-disables",
                    Severity::Warning,
                    &format!("Read error: {e}"),
                )
                .with_file(&file_path.display().to_string()),
            ];
        }
    };

    let mut results = Vec::new();
    let mut in_raw_string = false;
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_number = line_num + 1;

        // Track raw string boundaries (r#" ... "#)
        if trimmed.contains("r#\"") {
            in_raw_string = true;
        }
        if in_raw_string {
            if trimmed.contains("\"#") {
                in_raw_string = false;
            }
            continue;
        }

        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Check for suppression patterns
        for pattern in SUPPRESS_PATTERNS {
            if trimmed.contains(pattern) {
                // Check if it's a cfg_attr with allow inside
                if pattern == &"#[cfg_attr(" && !trimmed.contains("allow(") {
                    continue;
                }

                // Extract the lint name if possible
                let lint_name = extract_lint_name(trimmed);

                // Check if it's an allowed suppression
                if is_allowed_suppression(&lint_name) {
                    continue;
                }

                // Check if it's in a test module (more lenient)
                // We still report but as info
                let severity = if lint_name.starts_with("clippy::") {
                    Severity::Error
                } else {
                    Severity::Warning
                };

                results.push(
                    CheckResult::fail(
                        "clippy-disables",
                        severity,
                        &format!("{file_name}: Lint suppression found: {}", trimmed),
                    )
                    .with_file(&file_path.display().to_string())
                    .with_line(line_number)
                    .with_fix("Remove the #[allow(...)] and fix the underlying issue instead"),
                );
            }
        }
    }

    // If no issues found, return a pass
    if results.is_empty() {
        results.push(
            CheckResult::pass(
                "clippy-disables",
                &format!("{file_name}: No lint suppressions found"),
            )
            .with_file(&file_path.display().to_string()),
        );
    }

    results
}

fn extract_lint_name(line: &str) -> String {
    // Extract lint name from #[allow(lint_name)] or similar
    if let Some(start) = line.find("allow(") {
        let rest = &line[start + 6..];
        if let Some(end) = rest.find(')') {
            return rest[..end].trim().to_string();
        }
    }
    String::new()
}

fn is_allowed_suppression(lint_name: &str) -> bool {
    ALLOWED_SUPPRESSIONS
        .iter()
        .any(|allowed| lint_name.contains(allowed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) {
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join(name), content).unwrap();
    }

    #[test]
    fn test_detects_clippy_allow() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#[allow(clippy::unwrap_used)]
fn risky() {
    let x: Option<i32> = None;
    x.unwrap();
}
"#,
        );

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("clippy::unwrap_used"));
    }

    #[test]
    fn test_allows_dead_code() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#[allow(dead_code)]
fn unused_function() {}
"#,
        );

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_detects_module_level_allow() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#![allow(clippy::all)]

fn main() {}
"#,
        );

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_clean_file_passes() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
fn clean_code() -> i32 {
    42
}
"#,
        );

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }
}
