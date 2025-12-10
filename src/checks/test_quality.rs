//! Check that tests are not trivial or placeholder tests.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Patterns that indicate a trivial or placeholder test.
const TRIVIAL_PATTERNS: &[&str] = &[
    "assert!(true)",
    "assert_eq!(1, 1)",
    "assert_eq!(true, true)",
    "assert_ne!(1, 2)",
    "assert_ne!(true, false)",
    "todo!()",
    "unimplemented!()",
    "panic!(\"not implemented\")",
    "panic!(\"not yet implemented\")",
];

/// Check test quality in all Rust source files.
pub fn check(project_dir: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let src_dir = project_dir.join("src");

    if !src_dir.exists() {
        return results;
    }

    collect_results(&src_dir, &mut results);
    results
}

fn collect_results(dir: &Path, results: &mut Vec<CheckResult>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_results(&path, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    results.push(
                        CheckResult::fail("test-quality", Severity::Warning, &format!("Read error: {e}"))
                            .with_file(&path.display().to_string()),
                    );
                    continue;
                }
            };

            let file_results = analyze_file(&content, &path);
            if file_results.is_empty() {
                let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                results.push(
                    CheckResult::pass("test-quality", &format!("{file_name}: No trivial tests found"))
                        .with_file(&path.display().to_string()),
                );
            } else {
                results.extend(file_results);
            }
        }
    }
}

fn analyze_file(content: &str, file_path: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let mut in_test_function = false;
    let mut in_raw_string = false;
    let mut test_start_line = 0;
    let mut test_name = String::new();
    let mut brace_depth = 0;
    let mut test_brace_depth = 0;

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

        if trimmed == "#[test]" {
            in_test_function = true;
            test_start_line = line_number;
            continue;
        }

        // Capture test function name
        if in_test_function && test_name.is_empty() && trimmed.contains("fn ") {
            if let Some(start) = trimmed.find("fn ").map(|p| p + 3) {
                if let Some(end) = trimmed[start..].find('(') {
                    test_name = trimmed[start..start + end].trim().to_string();
                    test_brace_depth = brace_depth;
                }
            }
        }

        // Track brace depth
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if in_test_function && !test_name.is_empty() && brace_depth <= test_brace_depth {
                        in_test_function = false;
                        test_name.clear();
                    }
                }
                _ => {}
            }
        }

        // Check for trivial patterns inside test functions
        if in_test_function && !test_name.is_empty() {
            for pattern in TRIVIAL_PATTERNS {
                if trimmed.contains(pattern) {
                    results.push(
                        CheckResult::fail(
                            "test-quality",
                            Severity::Warning,
                            &format!("Trivial test pattern '{pattern}' in test '{test_name}'"),
                        )
                        .with_file(&file_path.display().to_string())
                        .with_line(line_number)
                        .with_fix(&format!(
                            "Replace trivial assertion with meaningful test logic in '{test_name}' (started at line {test_start_line})"
                        )),
                    );
                }
            }
        }
    }

    results
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
    fn test_detects_assert_true() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
"#,
        );

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("assert!(true)"));
    }

    #[test]
    fn test_detects_trivial_eq() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#[test]
fn bad_test() {
    assert_eq!(1, 1);
}
"#,
        );

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("assert_eq!(1, 1)"));
    }

    #[test]
    fn test_passes_real_tests() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
fn add(a: i32, b: i32) -> i32 { a + b }

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}
"#,
        );

        let results = check(temp.path());
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_detects_todo() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "lib.rs",
            r#"
#[test]
fn unfinished_test() {
    todo!();
}
"#,
        );

        let results = check(temp.path());
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].message.contains("todo!()"));
    }
}
