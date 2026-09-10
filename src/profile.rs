use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::{
    CODEX_CONFIG, CODEX_CONFIG_FILE, MARKER_FILE, ProfileMarker, verify_codex_config,
};
use crate::error::{AppError, Result};
use crate::paths::StatePaths;
use crate::permissions::{
    create_private_dir, create_private_dir_all, create_private_file, private_mode,
};

#[derive(Debug)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub marker: ProfileMarker,
}

#[derive(Debug, Default)]
pub struct ProfileListing {
    pub profiles: Vec<Profile>,
    pub ignored: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct ProfileStore {
    paths: StatePaths,
}

impl ProfileStore {
    pub fn new(paths: StatePaths) -> Self {
        Self { paths }
    }

    pub fn root(&self) -> &Path {
        self.paths.root()
    }

    pub fn create(&self, name: &str) -> Result<Profile> {
        validate_profile_name(name)?;
        self.ensure_root()?;

        let destination = self.paths.profile(name);
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(AppError::ProfileExists(name.to_owned()));
        }

        let temporary_path = self.create_temporary_directory(name)?;
        let mut guard = TemporaryProfile::new(temporary_path.clone());

        let marker = ProfileMarker::new(name);
        write_private_file(
            &temporary_path.join(MARKER_FILE),
            marker.serialize()?.as_bytes(),
        )?;
        write_private_file(
            &temporary_path.join(CODEX_CONFIG_FILE),
            CODEX_CONFIG.as_bytes(),
        )?;

        match fs::rename(&temporary_path, &destination) {
            Ok(()) => guard.disarm(),
            Err(_error) if destination.exists() => {
                return Err(AppError::ProfileExists(name.to_owned()));
            }
            Err(error) => {
                return Err(AppError::io(
                    format!("could not finish profile {name:?}"),
                    error,
                ));
            }
        }

        self.open(name)
    }

    pub fn open(&self, name: &str) -> Result<Profile> {
        let profile = self.owned(name)?;
        ensure_private_mode(&profile.path, 0o700, "profile directory")?;
        ensure_private_mode(&profile.path.join(MARKER_FILE), 0o600, "ownership marker")?;
        let config_path = profile.path.join(CODEX_CONFIG_FILE);
        let config_contents = read_regular_text_file(name, &config_path, false)?;
        ensure_private_mode(&config_path, 0o600, "Codex config")?;
        verify_codex_config(name, &config_contents)?;
        Ok(profile)
    }

    pub fn owned(&self, name: &str) -> Result<Profile> {
        validate_profile_name(name)?;
        let path = self.paths.profile(name);
        self.ensure_owned_profile(name, &path)
    }

    pub fn list(&self) -> Result<ProfileListing> {
        if !self.root().exists() {
            return Ok(ProfileListing::default());
        }
        self.validate_root()?;

        let entries = fs::read_dir(self.root()).map_err(|error| {
            AppError::io(format!("could not list {}", self.root().display()), error)
        })?;
        let mut listing = ProfileListing::default();

        for entry in entries {
            let entry = entry.map_err(|error| {
                AppError::io(
                    format!("could not read an entry in {}", self.root().display()),
                    error,
                )
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".tmp-") {
                continue;
            }

            match self.owned(&name) {
                Ok(profile) => listing.profiles.push(profile),
                Err(error) => listing.ignored.push((name, error.to_string())),
            }
        }

        listing
            .profiles
            .sort_by(|left, right| left.name.cmp(&right.name));
        listing.ignored.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(listing)
    }

    pub fn remove(&self, name: &str) -> Result<PathBuf> {
        let profile = self.owned(name)?;
        fs::remove_dir_all(&profile.path).map_err(|error| {
            AppError::io(
                format!(
                    "could not remove profile {name:?} at {}",
                    profile.path.display()
                ),
                error,
            )
        })?;
        Ok(profile.path)
    }

    fn ensure_root(&self) -> Result<()> {
        if self.root().exists() {
            return self.validate_root();
        }

        create_private_dir_all(self.root())?;
        self.validate_root()
    }

    fn validate_root(&self) -> Result<()> {
        ensure_real_directory(self.root())?;
        ensure_private_mode(self.root(), 0o700, "profile storage root")
    }

    fn ensure_owned_profile(&self, name: &str, path: &Path) -> Result<Profile> {
        if !path.exists() {
            return Err(AppError::ProfileMissing(name.to_owned()));
        }
        self.validate_root()?;
        ensure_real_directory(path)?;
        ensure_direct_child(self.root(), path)?;

        let marker_path = path.join(MARKER_FILE);
        let marker_contents = read_regular_text_file(name, &marker_path, true)?;
        let marker = ProfileMarker::parse(name, &marker_contents)?;

        Ok(Profile {
            name: name.to_owned(),
            path: path.to_path_buf(),
            marker,
        })
    }

    fn create_temporary_directory(&self, name: &str) -> Result<PathBuf> {
        for attempt in 0..100 {
            let temporary_name = format!(".tmp-{name}-{}-{attempt}", std::process::id());
            let temporary_path = self.root().join(temporary_name);
            match create_private_dir(&temporary_path) {
                Ok(()) => return Ok(temporary_path),
                Err(AppError::Io { source, .. })
                    if source.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }

        Err(AppError::UnsafeProfile {
            path: self.root().to_path_buf(),
            reason: "could not allocate a temporary profile directory".to_owned(),
        })
    }
}

pub fn validate_profile_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return Err(invalid_name(name, "use between 1 and 64 ASCII characters"));
    }
    if !bytes[0].is_ascii_alphanumeric() {
        return Err(invalid_name(
            name,
            "the first character must be a letter or digit",
        ));
    }
    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid_name(
            name,
            "use only ASCII letters, digits, underscores, and hyphens",
        ));
    }
    if is_windows_reserved_name(name) {
        return Err(invalid_name(name, "the name is reserved by Windows"));
    }

    Ok(())
}

fn invalid_name(name: &str, reason: &'static str) -> AppError {
    AppError::InvalidProfileName {
        name: name.to_owned(),
        reason,
    }
}

fn is_windows_reserved_name(name: &str) -> bool {
    let uppercase = name.to_ascii_uppercase();
    matches!(uppercase.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (uppercase.len() == 4
            && matches!(&uppercase[..3], "COM" | "LPT")
            && matches!(uppercase.as_bytes()[3], b'1'..=b'9'))
}

fn ensure_real_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| AppError::io(format!("could not inspect {}", path.display()), error))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::UnsafeProfile {
            path: path.to_path_buf(),
            reason: "directory is a symbolic link".to_owned(),
        });
    }
    if !metadata.is_dir() {
        return Err(AppError::UnsafeProfile {
            path: path.to_path_buf(),
            reason: "path is not a directory".to_owned(),
        });
    }

    Ok(())
}

fn ensure_direct_child(root: &Path, profile: &Path) -> Result<()> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| AppError::io(format!("could not resolve {}", root.display()), error))?;
    let canonical_profile = fs::canonicalize(profile)
        .map_err(|error| AppError::io(format!("could not resolve {}", profile.display()), error))?;
    if canonical_profile.parent() != Some(canonical_root.as_path()) {
        return Err(AppError::UnsafeProfile {
            path: profile.to_path_buf(),
            reason: "resolved path escapes the profile storage root".to_owned(),
        });
    }

    Ok(())
}

fn ensure_private_mode(path: &Path, expected: u32, kind: &'static str) -> Result<()> {
    if private_mode(path, expected)? {
        Ok(())
    } else {
        Err(AppError::InsecurePermissions {
            path: path.to_path_buf(),
            kind,
            expected,
        })
    }
}

fn read_regular_text_file(name: &str, path: &Path, marker: bool) -> Result<String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if marker {
            AppError::InvalidMarker {
                name: name.to_owned(),
                reason: format!("could not inspect {}: {error}", path.display()),
            }
        } else {
            AppError::UnsafeCredentialConfig {
                name: name.to_owned(),
                reason: format!("could not inspect {}: {error}", path.display()),
            }
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        let reason = format!("{} is not a regular file", path.display());
        return if marker {
            Err(AppError::InvalidMarker {
                name: name.to_owned(),
                reason,
            })
        } else {
            Err(AppError::UnsafeCredentialConfig {
                name: name.to_owned(),
                reason,
            })
        };
    }

    fs::read_to_string(path).map_err(|error| {
        if marker {
            AppError::InvalidMarker {
                name: name.to_owned(),
                reason: format!("could not read {}: {error}", path.display()),
            }
        } else {
            AppError::UnsafeCredentialConfig {
                name: name.to_owned(),
                reason: format!("could not read {}: {error}", path.display()),
            }
        }
    })
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = create_private_file(path)?;
    file.write_all(contents)
        .map_err(|error| AppError::io(format!("could not write {}", path.display()), error))?;
    file.sync_all()
        .map_err(|error| AppError::io(format!("could not sync {}", path.display()), error))
}

struct TemporaryProfile {
    path: PathBuf,
    armed: bool,
}

impl TemporaryProfile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryProfile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_portable_profile_names() {
        for valid in ["work", "Personal_2", "client-one"] {
            validate_profile_name(valid).unwrap();
        }

        for invalid in ["", "../work", "with space", "é", "CON", "com1"] {
            assert!(validate_profile_name(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn creates_and_reopens_owned_profile() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("profiles");
        let store = ProfileStore::new(StatePaths::from_absolute(root));

        let created = store.create("work").unwrap();
        let reopened = store.open("work").unwrap();

        assert_eq!(created.path, reopened.path);
        assert_eq!(reopened.marker.name, "work");
        assert_eq!(
            fs::read_to_string(created.path.join(CODEX_CONFIG_FILE)).unwrap(),
            CODEX_CONFIG
        );
    }

    #[test]
    fn refuses_unowned_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("profiles");
        fs::create_dir_all(root.join("work")).unwrap();
        let store = ProfileStore::new(StatePaths::from_absolute(root));

        assert!(store.open("work").is_err());
        assert!(store.remove("work").is_err());
    }
}
