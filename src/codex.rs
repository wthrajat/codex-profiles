use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use crate::error::{AppError, Result};

#[derive(Debug)]
pub struct Codex {
    executable: PathBuf,
}

impl Codex {
    pub fn resolve(override_executable: Option<OsString>) -> Result<Self> {
        let executable = match override_executable {
            Some(value) => resolve_requested(PathBuf::from(value))?,
            None => find_on_path(OsStr::new("codex")).ok_or(AppError::CodexNotFound)?,
        };
        reject_recursive_executable(&executable)?;

        Ok(Self { executable })
    }

    pub fn path(&self) -> &Path {
        &self.executable
    }

    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.executable)
            .arg("--version")
            .output()
            .map_err(|error| AppError::io("could not query the Codex version", error))?;
        if !output.status.success() {
            return Ok(format!(
                "unknown (exit {})",
                exit_status_code(output.status)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    pub fn login(&self, profile_home: &Path, device_auth: bool) -> Result<i32> {
        let mut arguments = vec![OsString::from("login")];
        if device_auth {
            arguments.push(OsString::from("--device-auth"));
        }
        self.wait(profile_home, &arguments, false)
    }

    pub fn login_status(&self, profile_home: &Path, quiet: bool) -> Result<i32> {
        let arguments = [OsString::from("login"), OsString::from("status")];
        self.wait(profile_home, &arguments, quiet)
    }

    pub fn run(&self, profile_home: &Path, arguments: &[OsString]) -> Result<i32> {
        let mut command = self.command(profile_home, arguments);

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;

            let error = command.exec();
            Err(AppError::io("could not launch Codex", error))
        }

        #[cfg(not(unix))]
        {
            let status = command
                .status()
                .map_err(|error| AppError::io("could not launch Codex", error))?;
            Ok(exit_status_code(status))
        }
    }

    fn wait(&self, profile_home: &Path, arguments: &[OsString], quiet: bool) -> Result<i32> {
        let mut command = self.command(profile_home, arguments);
        if quiet {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = command
            .status()
            .map_err(|error| AppError::io("could not launch Codex", error))?;
        Ok(exit_status_code(status))
    }

    fn command(&self, profile_home: &Path, arguments: &[OsString]) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(arguments).env("CODEX_HOME", profile_home);
        command
    }
}

fn resolve_requested(requested: PathBuf) -> Result<PathBuf> {
    if requested.components().count() == 1 {
        return find_on_path(requested.as_os_str()).ok_or(AppError::CodexNotFound);
    }
    resolve_executable_file(&requested).ok_or(AppError::CodexNotFound)
}

fn find_on_path(name: &OsStr) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        for candidate_name in executable_names(name) {
            let candidate = directory.join(candidate_name);
            if let Some(executable) = resolve_executable_file(&candidate) {
                return Some(executable);
            }
        }
    }
    None
}

fn resolve_executable_file(path: &Path) -> Option<PathBuf> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }

    fs::canonicalize(path).ok()
}

fn executable_names(name: &OsStr) -> Vec<OsString> {
    #[cfg(not(windows))]
    {
        vec![name.to_os_string()]
    }

    #[cfg(windows)]
    {
        let requested = Path::new(name);
        if requested.extension().is_some() {
            return vec![name.to_os_string()];
        }
        let extensions =
            env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
        extensions
            .to_string_lossy()
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| {
                let mut candidate = name.to_os_string();
                candidate.push(extension);
                candidate
            })
            .collect()
    }
}

fn reject_recursive_executable(executable: &Path) -> Result<()> {
    let current = env::current_exe()
        .map_err(|error| AppError::io("could not identify the running executable", error))?;
    let current = fs::canonicalize(current)
        .map_err(|error| AppError::io("could not resolve the running executable", error))?;
    if current == executable {
        return Err(AppError::RecursiveCodexExecutable);
    }

    Ok(())
}

fn exit_status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_explicit_executable_is_rejected() {
        let missing = std::env::temp_dir().join("missing-codex-for-switcher-test");
        assert!(Codex::resolve(Some(missing.into_os_string())).is_err());
    }
}
