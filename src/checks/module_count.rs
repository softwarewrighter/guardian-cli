//! Check that crates don't have too many modules.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Check module count per crate.
pub fn check(project_dir: &Path, max_modules: usize) -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check if this is a workspace
    let cargo_path = project_dir.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if content.contains("[workspace]") {
                // It's a workspace - check each member
                check_workspace(project_dir, max_modules, &mut results);
                return results;
            }
        }
    }

    // Single crate
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        let crate_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "crate".to_string());
        results.push(check_crate(&src_dir, &crate_name, max_modules));
    }

    results
}

fn check_workspace(workspace_dir: &Path, max_modules: usize, results: &mut Vec<CheckResult>) {
    // Check root src if it exists (for workspace with root crate)
    let root_src = workspace_dir.join("src");
    if root_src.exists() {
        results.push(check_crate(&root_src, "root", max_modules));
    }

    // Check member directories
    let entries = match fs::read_dir(workspace_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && !is_ignored_dir(&path) {
            let src_dir = path.join("src");
            if src_dir.exists() {
                let crate_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                results.push(check_crate(&src_dir, &crate_name, max_modules));
            }
        }
    }
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name, "target" | ".git" | "node_modules" | ".cargo")
}

fn check_crate(src_dir: &Path, crate_name: &str, max_modules: usize) -> CheckResult {
    let module_count = count_modules(src_dir);

    if module_count > max_modules {
        CheckResult::fail(
            "module-count",
            Severity::Error,
            &format!("{crate_name}: {module_count} modules exceeds max {max_modules}"),
        )
        .with_file(&src_dir.display().to_string())
        .with_fix("Consider splitting into multiple crates or reducing module count")
    } else {
        CheckResult::pass(
            "module-count",
            &format!("{crate_name}: {module_count} modules (OK)"),
        )
        .with_file(&src_dir.display().to_string())
    }
}

/// Count modules in a src directory.
///
/// A module is either:
/// - A .rs file (except main.rs, lib.rs, mod.rs)
/// - A directory containing mod.rs
fn count_modules(src_dir: &Path) -> usize {
    let mut count = 0;

    let entries = match fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_file() && name.ends_with(".rs") {
            // Count .rs files except entry points
            if !matches!(name, "main.rs" | "lib.rs" | "mod.rs") {
                count += 1;
            }
        } else if path.is_dir() {
            // Count directories that are modules (contain mod.rs or have .rs files)
            let mod_rs = path.join("mod.rs");
            if mod_rs.exists() || has_rs_files(&path) {
                count += 1;
            }
        }
    }

    count
}

fn has_rs_files(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_src_structure(dir: &Path, modules: &[&str]) {
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();

        for module in modules {
            fs::write(src.join(format!("{module}.rs")), "// module").unwrap();
        }
    }

    #[test]
    fn test_count_under_limit() {
        let temp = TempDir::new().unwrap();
        create_src_structure(temp.path(), &["config", "utils"]);

        let results = check(temp.path(), 4);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_count_over_limit() {
        let temp = TempDir::new().unwrap();
        create_src_structure(temp.path(), &["a", "b", "c", "d", "e"]);

        let results = check(temp.path(), 4);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert!(results[0].message.contains("5 modules"));
    }

    #[test]
    fn test_excludes_main_lib() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "").unwrap();
        fs::write(src.join("lib.rs"), "").unwrap();
        fs::write(src.join("mod.rs"), "").unwrap();

        let count = count_modules(&src);
        assert_eq!(count, 0);
    }
}
