//! Check that modules don't have too many functions.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Check function count per module.
pub fn check(project_dir: &Path, max_functions: usize) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let src_dir = project_dir.join("src");

    if !src_dir.exists() {
        return results;
    }

    collect_results(&src_dir, max_functions, &mut results);
    results
}

fn collect_results(dir: &Path, max_functions: usize, results: &mut Vec<CheckResult>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_results(&path, max_functions, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    results.push(
                        CheckResult::fail(
                            "function-count",
                            Severity::Warning,
                            &format!("Read error: {e}"),
                        )
                        .with_file(&path.display().to_string()),
                    );
                    continue;
                }
            };

            let function_count = count_functions(&content);
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let result = if function_count > max_functions {
                CheckResult::fail(
                    "function-count",
                    Severity::Error,
                    &format!("{file_name}: {function_count} functions exceeds max {max_functions}"),
                )
                .with_file(&path.display().to_string())
                .with_fix(&format!(
                    "Split {file_name} into smaller modules with fewer functions"
                ))
            } else {
                CheckResult::pass(
                    "function-count",
                    &format!("{file_name}: {function_count} functions (OK)"),
                )
                .with_file(&path.display().to_string())
            };
            results.push(result);
        }
    }
}

/// Count function definitions in Rust source code, excluding test modules and string literals.
fn count_functions(content: &str) -> usize {
    let mut count = 0;
    let mut in_test_module = false;
    let mut in_raw_string = false;
    let mut brace_depth = 0;
    let mut test_module_depth = 0;

    let fn_patterns = [
        "fn ", "pub fn ", "pub(crate) fn ", "pub(super) fn ",
        "async fn ", "pub async fn ", "const fn ", "pub const fn ",
        "unsafe fn ", "pub unsafe fn ",
    ];

    for line in content.lines() {
        let trimmed = line.trim();

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

        // Track test module boundaries
        if trimmed.contains("#[cfg(test)]") {
            in_test_module = true;
            test_module_depth = brace_depth;
        }

        // Count braces to track scope
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' if in_test_module && brace_depth - 1 <= test_module_depth => {
                    brace_depth -= 1;
                    in_test_module = false;
                }
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        // Skip test module content and comments
        if in_test_module
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }

        // Count function definitions
        if fn_patterns.iter().any(|p| trimmed.contains(p)) && trimmed.contains('(') {
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_simple_functions() {
        let content = r#"
fn foo() {}
fn bar() {}
pub fn baz() {}
"#;
        assert_eq!(count_functions(content), 3);
    }

    #[test]
    fn test_excludes_test_functions() {
        let content = r#"
fn production_fn() {}

#[cfg(test)]
mod tests {
    fn test_helper() {}

    #[test]
    fn test_something() {}
}
"#;
        assert_eq!(count_functions(content), 1);
    }

    #[test]
    fn test_count_async_functions() {
        let content = r#"
async fn async_foo() {}
pub async fn async_bar() {}
"#;
        assert_eq!(count_functions(content), 2);
    }

    #[test]
    fn test_ignores_comments() {
        let content = r#"
// fn not_a_function() {}
fn real_function() {}
"#;
        assert_eq!(count_functions(content), 1);
    }
}
