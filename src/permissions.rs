use std::fs::{self, File, OpenOptions};
use std::path::Path;

use crate::error::{AppError, Result};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

pub fn create_private_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    let mut builder = fs::DirBuilder::new();
    #[cfg(not(unix))]
    let builder = fs::DirBuilder::new();

    #[cfg(unix)]
    builder.mode(0o700);

    builder
        .create(path)
        .map_err(|error| AppError::io(format!("could not create {}", path.display()), error))?;
    harden_directory(path)
}

pub fn create_private_dir_all(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);

    #[cfg(unix)]
    builder.mode(0o700);

    builder
        .create(path)
        .map_err(|error| AppError::io(format!("could not create {}", path.display()), error))?;
    harden_directory(path)
}

pub fn create_private_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    options.mode(0o600);

    let file = options
        .open(path)
        .map_err(|error| AppError::io(format!("could not create {}", path.display()), error))?;
    harden_file(path)?;
    Ok(file)
}

pub fn harden_directory(_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o700);
        fs::set_permissions(_path, permissions).map_err(|error| {
            AppError::io(
                format!("could not set private permissions on {}", _path.display()),
                error,
            )
        })?;
    }

    Ok(())
}

fn harden_file(_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(_path, permissions).map_err(|error| {
            AppError::io(
                format!("could not set private permissions on {}", _path.display()),
                error,
            )
        })?;
    }

    Ok(())
}

#[cfg(unix)]
pub fn private_mode(path: &Path, expected: u32) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| AppError::io(format!("could not inspect {}", path.display()), error))?;
    Ok(metadata.permissions().mode() & 0o777 == expected)
}

#[cfg(not(unix))]
pub fn private_mode(_path: &Path, _expected: u32) -> Result<bool> {
    Ok(true)
}
