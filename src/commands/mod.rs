//! Command implementations for Guardian CLI.
//!
//! Commands are organized into modules by function:
//! - `host`: Ollama host management (ping, list-models, select-host)
//! - `config_cmd`: Configuration display
//! - `llm`: LLM interaction (ask, evaluate)
//! - `checks`: Code quality checks
//! - `output`: Shared output formatting

mod checks;
mod config_cmd;
mod host;
mod llm;
mod output;

// Re-export public command functions
pub use checks::{run_checks, CheckOptions};
pub use config_cmd::{config_path, show_config};
pub use host::{list_models, ping_hosts, select_host};
pub use llm::{ask, evaluate};
