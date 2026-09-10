use std::io::{self, IsTerminal, Write};

use crate::cli::{Cli, Command};
use crate::codex::Codex;
use crate::config::{CODEX_CONFIG_FILE, MARKER_FILE};
use crate::error::{AppError, Result};
use crate::paths::{StatePaths, display_redacted};
use crate::permissions::private_mode;
use crate::profile::{Profile, ProfileStore};

pub fn run(cli: Cli) -> Result<i32> {
    let paths = StatePaths::resolve(cli.root)?;
    let store = ProfileStore::new(paths);

    match cli.command {
        Command::Create { name } => {
            let profile = store.create(&name)?;
            println!(
                "Created profile {:?} at {}",
                profile.name,
                profile.path.display()
            );
            Ok(0)
        }
        Command::List { check } => list_profiles(&store, check, cli.codex_bin),
        Command::Path { name } => {
            let profile = store.owned(&name)?;
            println!("{}", profile.path.display());
            Ok(0)
        }
        Command::Login { name, device_auth } => {
            let profile = store.open(&name)?;
            Codex::resolve(cli.codex_bin)?.login(&profile.path, device_auth)
        }
        Command::Status { name } => {
            let profile = store.open(&name)?;
            Codex::resolve(cli.codex_bin)?.login_status(&profile.path, false)
        }
        Command::Run { name, codex_args } => {
            let profile = store.open(&name)?;
            Codex::resolve(cli.codex_bin)?.run(&profile.path, &codex_args)
        }
        Command::Doctor { name } => doctor(&store, name.as_deref(), cli.codex_bin),
        Command::Remove { name, yes } => {
            let profile = store.owned(&name)?;
            confirm_removal(&profile, yes)?;
            let removed = store.remove(&name)?;
            println!("Removed profile {name:?} at {}", removed.display());
            Ok(0)
        }
    }
}

fn list_profiles(
    store: &ProfileStore,
    check: bool,
    codex_bin: Option<std::ffi::OsString>,
) -> Result<i32> {
    let listing = store.list()?;
    for (ignored, reason) in listing.ignored {
        eprintln!("warning: ignored unowned or invalid entry {ignored:?}: {reason}");
    }
    if listing.profiles.is_empty() {
        println!("No profiles found.");
        return Ok(0);
    }

    if !check {
        for profile in listing.profiles {
            println!("{}", profile.name);
        }
        return Ok(0);
    }

    let codex = Codex::resolve(codex_bin)?;
    for profile in listing.profiles {
        let profile = store.open(&profile.name)?;
        let status = codex.login_status(&profile.path, true)?;
        if status == 0 {
            println!("{}\tauthenticated", profile.name);
        } else {
            println!("{}\tnot authenticated (exit {status})", profile.name);
        }
    }

    Ok(0)
}

fn confirm_removal(profile: &Profile, yes: bool) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(AppError::NonInteractiveConfirmation);
    }

    eprint!(
        "Permanently remove profile {:?} and all Codex state at {}? [y/N] ",
        profile.name,
        profile.path.display()
    );
    io::stderr()
        .flush()
        .map_err(|error| AppError::io("could not display confirmation", error))?;

    let mut response = String::new();
    io::stdin()
        .read_line(&mut response)
        .map_err(|error| AppError::io("could not read confirmation", error))?;
    if matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        Err(AppError::RemovalCancelled)
    }
}

fn doctor(
    store: &ProfileStore,
    name: Option<&str>,
    codex_bin: Option<std::ffi::OsString>,
) -> Result<i32> {
    let mut failed = false;

    if store.root().exists() {
        match private_mode(store.root(), 0o700) {
            Ok(true) => print_ok(format!(
                "profile root {} is private",
                display_redacted(store.root())
            )),
            Ok(false) => {
                print_failure(format!(
                    "profile root {} has broader permissions than expected",
                    display_redacted(store.root())
                ));
                failed = true;
            }
            Err(error) => {
                print_failure(error);
                failed = true;
            }
        }
    } else {
        print_ok(format!(
            "profile root {} has not been created yet",
            display_redacted(store.root())
        ));
    }

    match Codex::resolve(codex_bin) {
        Ok(codex) => match codex.version() {
            Ok(version) => print_ok(format!(
                "found {version} at {}",
                display_redacted(codex.path())
            )),
            Err(error) => {
                print_failure(error);
                failed = true;
            }
        },
        Err(error) => {
            print_failure(error);
            failed = true;
        }
    }

    if let Some(name) = name {
        match store.open(name) {
            Ok(profile) => {
                print_ok(format!(
                    "profile {:?} has valid ownership metadata (created by {})",
                    profile.name, profile.marker.created_by_version
                ));
                failed |= !check_profile_permissions(&profile);
            }
            Err(error) => {
                print_failure(error);
                failed = true;
            }
        }
    }

    #[cfg(windows)]
    println!(
        "[warn] Windows profile privacy relies on the current user's inherited filesystem ACLs"
    );
    Ok(i32::from(failed))
}

fn check_profile_permissions(profile: &Profile) -> bool {
    let marker_path = profile.path.join(MARKER_FILE);
    let config_path = profile.path.join(CODEX_CONFIG_FILE);
    let checks = [
        (profile.path.as_path(), 0o700, "profile directory"),
        (marker_path.as_path(), 0o600, "ownership marker"),
        (config_path.as_path(), 0o600, "Codex config"),
    ];
    let mut valid = true;

    for (path, expected, label) in checks {
        match private_mode(path, expected) {
            Ok(true) => print_ok(format!("{label} permissions are private")),
            Ok(false) => {
                print_failure(format!("{label} permissions are broader than expected"));
                valid = false;
            }
            Err(error) => {
                print_failure(error);
                valid = false;
            }
        }
    }

    valid
}

fn print_ok(message: impl std::fmt::Display) {
    println!("[ok] {message}");
}

fn print_failure(message: impl std::fmt::Display) {
    println!("[fail] {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::StatePaths;

    #[test]
    fn yes_skips_confirmation() {
        let temporary = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(StatePaths::from_absolute(temporary.path().join("profiles")));
        let profile = store.create("work").unwrap();

        confirm_removal(&profile, true).unwrap();
    }

    #[test]
    fn profile_permissions_are_private() {
        let temporary = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(StatePaths::from_absolute(temporary.path().join("profiles")));
        let profile = store.create("work").unwrap();

        assert!(check_profile_permissions(&profile));
    }

    #[test]
    fn path_labels_are_stable() {
        assert_eq!(
            std::path::Path::new(MARKER_FILE).file_name().unwrap(),
            MARKER_FILE
        );
    }
}
