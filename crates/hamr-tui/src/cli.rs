//! CLI argument parsing for hamr-tui.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hamr-tui")]
#[command(
    about = "Hamr TUI client - typically invoked via 'hamr' command",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable debug logging (logs to /tmp/hamr-tui.log)
    #[arg(short, long)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive TUI mode (default)
    Tui,

    /// List all plugins, or open a specific plugin
    Plugins {
        /// Plugin ID to open (optional, lists all if not provided)
        id: Option<String>,
    },

    /// Show index stats
    Index,

    /// One-shot search query (for testing)
    Query {
        /// Search query
        query: String,
    },

    /// Test a plugin with a query
    Test {
        /// Plugin ID
        plugin: String,
        /// Search query
        query: String,
    },
}
