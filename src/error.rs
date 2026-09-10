use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[cfg(unix)]
    #[error("could not determine the current user's home directory; pass --root PATH")]
    HomeDirectoryUnavailable,

    #[cfg(windows)]
    #[error("could not determine LOCALAPPDATA; pass --root PATH")]
    LocalAppDataUnavailable,

    #[error("profile name {name:?} is invalid: {reason}")]
    InvalidProfileName { name: String, reason: &'static str },

    #[error("profile {0:?} already exists")]
    ProfileExists(String),

    #[error("profile {0:?} does not exist")]
    ProfileMissing(String),

    #[error("refusing unsafe profile at {path}: {reason}")]
    UnsafeProfile { path: PathBuf, reason: String },

    #[error("{path} has insecure permissions for {kind}; expected mode {expected:o}")]
    InsecurePermissions {
        path: PathBuf,
        kind: &'static str,
        expected: u32,
    },

    #[error("profile {name:?} has invalid ownership metadata: {reason}")]
    InvalidMarker { name: String, reason: String },

    #[error("profile {name:?} does not enforce file-backed Codex credentials: {reason}")]
    UnsafeCredentialConfig { name: String, reason: String },

    #[error("Codex CLI was not found; install `codex` or pass --codex-bin PATH")]
    CodexNotFound,

    #[error(
        "the resolved Codex executable is this profile switcher; pass --codex-bin PATH to the real Codex CLI"
    )]
    RecursiveCodexExecutable,

    #[error("removal was cancelled")]
    RemovalCancelled,

    #[error("refusing to prompt on non-interactive stdin; rerun with --yes")]
    NonInteractiveConfirmation,

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}

impl AppError {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}
