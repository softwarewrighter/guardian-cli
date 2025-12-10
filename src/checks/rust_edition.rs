//! Check that Cargo.toml uses the required Rust edition.

use super::{CheckResult, Severity};
use std::fs;
use std::path::Path;

/// Check Rust edition in Cargo.toml files.
pub fn check(project_dir: &Path, required_edition: &str) -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Find all Cargo.toml files
    let cargo_files = find_cargo_files(project_dir);

    if cargo_files.is_empty() {
        results.push(CheckResult::fail(
            "rust-edition",
            Severity::Warning,
            "No Cargo.toml found",
        ));
        return results;
    }

    for cargo_path in cargo_files {
        let result = check_cargo_toml(&cargo_path, required_edition);
        results.push(result);
    }

    results
}

fn find_cargo_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    // Check root Cargo.toml
    let root_cargo = dir.join("Cargo.toml");
    if root_cargo.exists() {
        files.push(root_cargo);
    }

    // Check subdirectories for workspace members
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !is_ignored_dir(&path) {
                let sub_cargo = path.join("Cargo.toml");
                if sub_cargo.exists() {
                    files.push(sub_cargo);
                }
            }
        }
    }

    files
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name, "target" | ".git" | "node_modules" | ".cargo")
}

fn check_cargo_toml(cargo_path: &Path, required_edition: &str) -> CheckResult {
    let content = match fs::read_to_string(cargo_path) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "rust-edition",
                Severity::Error,
                &format!("Failed to read: {e}"),
            )
            .with_file(&cargo_path.display().to_string());
        }
    };

    let rel_path = cargo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| cargo_path.display().to_string());

    // Parse TOML to find edition
    let parsed: Result<toml::Value, _> = content.parse();
    match parsed {
        Ok(toml) => check_edition_value(&toml, &rel_path, cargo_path, required_edition),
        Err(e) => CheckResult::fail(
            "rust-edition",
            Severity::Error,
            &format!("Invalid TOML: {e}"),
        )
        .with_file(&cargo_path.display().to_string()),
    }
}

fn check_edition_value(
    toml: &toml::Value,
    rel_path: &str,
    cargo_path: &Path,
    required: &str,
) -> CheckResult {
    let edition = toml
        .get("package")
        .and_then(|p| p.get("edition"))
        .and_then(|e| e.as_str());

    match edition {
        Some(e) if e == required => CheckResult::pass(
            "rust-edition",
            &format!("{rel_path}: Using Rust {required} edition"),
        )
        .with_file(&cargo_path.display().to_string()),

        Some(e) => CheckResult::fail(
            "rust-edition",
            Severity::Error,
            &format!("{rel_path}: Using edition '{e}', expected '{required}'"),
        )
        .with_file(&cargo_path.display().to_string())
        .with_fix(&format!(
            "Change edition = \"{e}\" to edition = \"{required}\""
        )),

        None => {
            // Check if it's a workspace root (no package section)
            if toml.get("workspace").is_some() && toml.get("package").is_none() {
                CheckResult::pass(
                    "rust-edition",
                    &format!("{rel_path}: Workspace root (no edition required)"),
                )
                .with_file(&cargo_path.display().to_string())
            } else {
                CheckResult::fail(
                    "rust-edition",
                    Severity::Error,
                    &format!("{rel_path}: No edition specified"),
                )
                .with_file(&cargo_path.display().to_string())
                .with_fix(&format!(
                    "Add edition = \"{required}\" to [package] section"
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_correct_edition() {
        let temp = TempDir::new().unwrap();
        let cargo = temp.path().join("Cargo.toml");
        fs::write(
            &cargo,
            r#"
[package]
name = "test"
edition = "2024"
"#,
        )
        .unwrap();

        let results = check(temp.path(), "2024");
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_wrong_edition() {
        let temp = TempDir::new().unwrap();
        let cargo = temp.path().join("Cargo.toml");
        fs::write(
            &cargo,
            r#"
[package]
name = "test"
edition = "2021"
"#,
        )
        .unwrap();

        let results = check(temp.path(), "2024");
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert!(results[0].message.contains("2021"));
        assert!(results[0].fix.is_some());
    }

    #[test]
    fn test_missing_edition() {
        let temp = TempDir::new().unwrap();
        let cargo = temp.path().join("Cargo.toml");
        fs::write(
            &cargo,
            r#"
[package]
name = "test"
"#,
        )
        .unwrap();

        let results = check(temp.path(), "2024");
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert!(results[0].message.contains("No edition"));
    }
}
