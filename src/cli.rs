use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "codex-profile-switcher",
    version,
    about = "Run Codex CLI with isolated local account profiles",
    long_about = "Create isolated CODEX_HOME directories and launch the installed Codex CLI without touching the default ~/.codex state.",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Override the profile storage root.
    #[arg(long, global = true, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Use this Codex executable instead of searching PATH.
    #[arg(long, global = true, value_name = "PATH")]
    pub codex_bin: Option<OsString>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new isolated profile.
    Create { name: String },

    /// List profiles without reading credential files.
    List {
        /// Ask Codex for each profile's login status.
        #[arg(long)]
        check: bool,
    },

    /// Print a profile's absolute CODEX_HOME path.
    Path { name: String },

    /// Authenticate a profile through Codex.
    Login {
        name: String,

        /// Use Codex's device-code authentication flow.
        #[arg(long)]
        device_auth: bool,
    },

    /// Show Codex login status for a profile.
    Status { name: String },

    /// Run Codex with a profile and forward every remaining argument.
    Run {
        name: String,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        codex_args: Vec<OsString>,
    },

    /// Diagnose Codex discovery, storage, and profile isolation.
    Doctor { name: Option<String> },

    /// Permanently remove a profile created by this tool.
    Remove {
        name: String,

        /// Skip the interactive confirmation.
        #[arg(long, short = 'y')]
        yes: bool,
    },
}
