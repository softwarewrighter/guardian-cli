//! Check that source files don't exceed line count limits.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Check LOC limits for all Rust source files.
pub fn check(project_dir: &Path, max_loc: usize, warn_loc: usize) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let src_dir = project_dir.join("src");

    if !src_dir.exists() {
        return results;
    }

    check_directory(&src_dir, max_loc, warn_loc, &mut results);
    results
}

fn check_directory(dir: &Path, max_loc: usize, warn_loc: usize, results: &mut Vec<CheckResult>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            check_directory(&path, max_loc, warn_loc, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            results.push(check_file(&path, max_loc, warn_loc));
        }
    }
}

fn check_file(file_path: &Path, max_loc: usize, warn_loc: usize) -> CheckResult {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail("loc-limits", Severity::Warning, &format!("Read error: {e}"))
                .with_file(&file_path.display().to_string());
        }
    };

    let loc = content.lines().count();
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    if loc > max_loc {
        CheckResult::fail(
            "loc-limits",
            Severity::Error,
            &format!("{file_name}: {loc} lines exceeds max {max_loc}"),
        )
        .with_file(&file_path.display().to_string())
        .with_fix(&format!(
            "Split {file_name} into smaller modules (each under {max_loc} lines)"
        ))
    } else if loc > warn_loc {
        CheckResult::fail(
            "loc-limits",
            Severity::Warning,
            &format!("{file_name}: {loc} lines exceeds warning threshold {warn_loc}"),
        )
        .with_file(&file_path.display().to_string())
        .with_fix("Consider splitting into smaller modules")
    } else {
        CheckResult::pass("loc-limits", &format!("{file_name}: {loc} lines (OK)"))
            .with_file(&file_path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_file_with_lines(dir: &Path, name: &str, lines: usize) {
        let content: String = (0..lines).map(|i| format!("// line {i}\n")).collect();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src").join(name), content).unwrap();
    }

    #[test]
    fn test_file_under_limit() {
        let temp = TempDir::new().unwrap();
        create_file_with_lines(temp.path(), "main.rs", 100);

        let results = check(temp.path(), 500, 350);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_file_over_warning() {
        let temp = TempDir::new().unwrap();
        create_file_with_lines(temp.path(), "main.rs", 400);

        let results = check(temp.path(), 500, 350);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn test_file_over_max() {
        let temp = TempDir::new().unwrap();
        create_file_with_lines(temp.path(), "main.rs", 600);

        let results = check(temp.path(), 500, 350);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert_eq!(results[0].severity, Severity::Error);
    }
}
