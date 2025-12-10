//! Check-related commands: run_checks.

use super::output;
use crate::checks::{self, CheckConfig, CheckResult};
use anyhow::Result;
use std::path::Path;

/// Options for the check command.
pub struct CheckOptions<'a> {
    pub path: Option<&'a Path>,
    pub only: Option<&'a str>,
    pub max_loc: usize,
    pub warn_loc: usize,
    pub max_functions: usize,
    pub max_modules: usize,
    pub edition: &'a str,
    pub json_output: bool,
}

/// Run checklist validation on a project.
pub fn run_checks(opts: CheckOptions<'_>) -> Result<()> {
    let project_dir = opts.path.unwrap_or(Path::new("."));

    let config = CheckConfig {
        max_file_loc: opts.max_loc,
        warn_file_loc: opts.warn_loc,
        max_functions_per_module: opts.max_functions,
        max_modules_per_crate: opts.max_modules,
        required_edition: opts.edition.to_string(),
    };

    let results = run_selected_checks(project_dir, &config, opts.only);
    output::check_results(&results, opts.json_output)
}

pub(crate) fn run_selected_checks(
    project_dir: &Path,
    config: &CheckConfig,
    only: Option<&str>,
) -> Vec<CheckResult> {
    let filter: Option<Vec<&str>> = only.map(|s| s.split(',').map(str::trim).collect());
    let should_run = |name: &str| filter.as_ref().map_or(true, |f| f.contains(&name));

    let mut results = Vec::new();

    if should_run("rust-edition") {
        results.extend(checks::rust_edition::check(
            project_dir,
            &config.required_edition,
        ));
    }

    if should_run("loc-limits") {
        results.extend(checks::loc_limits::check(
            project_dir,
            config.max_file_loc,
            config.warn_file_loc,
        ));
    }

    if should_run("function-count") {
        results.extend(checks::function_count::check(
            project_dir,
            config.max_functions_per_module,
        ));
    }

    if should_run("module-count") {
        results.extend(checks::module_count::check(
            project_dir,
            config.max_modules_per_crate,
        ));
    }

    if should_run("test-quality") {
        results.extend(checks::test_quality::check(project_dir));
    }

    if should_run("clippy-disables") {
        results.extend(checks::clippy_disables::check(project_dir));
    }

    if should_run("cache-busting") {
        results.extend(checks::cache_busting::check(project_dir));
    }

    results
}
