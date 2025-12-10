//! Code quality checks for Guardian CLI.
//!
//! Each check module implements specific validation rules that can be
//! run against a Rust project to enforce coding standards.

pub mod cache_busting;
pub mod clippy_disables;
pub mod function_count;
pub mod loc_limits;
pub mod module_count;
pub mod rust_edition;
pub mod test_quality;

/// Severity level for check results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational - no action required
    Info,
    /// Warning - should be addressed
    Warning,
    /// Error - must be fixed before proceeding
    Error,
}

/// Result of a single check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Name of the check
    pub check_name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Severity if failed
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// File path related to the issue (if applicable)
    pub file: Option<String>,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Suggested fix
    pub fix: Option<String>,
}

impl CheckResult {
    /// Create a passing result.
    pub fn pass(check_name: &str, message: &str) -> Self {
        Self {
            check_name: check_name.to_string(),
            passed: true,
            severity: Severity::Info,
            message: message.to_string(),
            file: None,
            line: None,
            fix: None,
        }
    }

    /// Create a failing result.
    pub fn fail(check_name: &str, severity: Severity, message: &str) -> Self {
        Self {
            check_name: check_name.to_string(),
            passed: false,
            severity,
            message: message.to_string(),
            file: None,
            line: None,
            fix: None,
        }
    }

    /// Add file location to result.
    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }

    /// Add line number to result.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add suggested fix to result.
    pub fn with_fix(mut self, fix: &str) -> Self {
        self.fix = Some(fix.to_string());
        self
    }
}

/// Configuration for checks with thresholds.
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Maximum lines of code per file
    pub max_file_loc: usize,
    /// Warning threshold for file LOC
    pub warn_file_loc: usize,
    /// Maximum functions per module
    pub max_functions_per_module: usize,
    /// Maximum modules per crate
    pub max_modules_per_crate: usize,
    /// Required Rust edition
    pub required_edition: String,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            max_file_loc: 500,
            warn_file_loc: 350,
            max_functions_per_module: 7,
            max_modules_per_crate: 4,
            required_edition: "2024".to_string(),
        }
    }
}
