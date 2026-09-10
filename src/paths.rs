use std::env;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, Result};

const APPLICATION_NAME: &str = "codex-profile-switcher";

#[derive(Clone, Debug)]
pub struct StatePaths {
    root: PathBuf,
}

impl StatePaths {
    pub fn resolve(override_root: Option<PathBuf>) -> Result<Self> {
        let root = match override_root {
            Some(path) => make_absolute(path)?,
            None => default_root()?,
        };

        Ok(Self { root })
    }

    #[cfg(test)]
    pub fn from_absolute(root: PathBuf) -> Self {
        debug_assert!(root.is_absolute());
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn profile(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

fn make_absolute(path: PathBuf) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| AppError::io("could not resolve the current directory", error))?
            .join(path)
    };

    normalize_absolute(&absolute)
}

fn normalize_absolute(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(AppError::UnsafeProfile {
                        path: path.to_path_buf(),
                        reason: "path escapes the filesystem root".to_owned(),
                    });
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    if !normalized.is_absolute() {
        return Err(AppError::UnsafeProfile {
            path: path.to_path_buf(),
            reason: "storage root must be absolute".to_owned(),
        });
    }

    Ok(normalized)
}

#[cfg(target_os = "macos")]
fn default_root() -> Result<PathBuf> {
    let home = home_directory()?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join(APPLICATION_NAME)
        .join("profiles"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_root() -> Result<PathBuf> {
    if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
        let state_home = PathBuf::from(state_home);
        if state_home.is_absolute() {
            return Ok(state_home.join(APPLICATION_NAME).join("profiles"));
        }
    }

    Ok(home_directory()?
        .join(".local")
        .join("state")
        .join(APPLICATION_NAME)
        .join("profiles"))
}

#[cfg(windows)]
fn default_root() -> Result<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join(APPLICATION_NAME).join("profiles"))
        .ok_or(AppError::LocalAppDataUnavailable)
}

#[cfg(unix)]
fn home_directory() -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(AppError::HomeDirectoryUnavailable)
}

pub fn display_redacted(path: &Path) -> String {
    let home = platform_home_directory();
    if let Some(home) = home {
        if let Ok(relative) = path.strip_prefix(&home) {
            return if relative.as_os_str().is_empty() {
                "~".to_owned()
            } else {
                format!("~{}{}", std::path::MAIN_SEPARATOR, relative.display())
            };
        }
    }

    path.display().to_string()
}

fn platform_home_directory() -> Option<PathBuf> {
    #[cfg(unix)]
    let variable: Option<OsString> = env::var_os("HOME");
    #[cfg(windows)]
    let variable: Option<OsString> = env::var_os("USERPROFILE");

    variable
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_override_becomes_absolute_and_normalized() {
        let paths = StatePaths::resolve(Some(PathBuf::from("one/../profiles"))).unwrap();

        assert!(paths.root().is_absolute());
        assert!(paths.root().ends_with("profiles"));
        assert!(!paths.root().to_string_lossy().contains(".."));
    }
}
