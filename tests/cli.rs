use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn switcher(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cxprof"));
    command.arg("--root").arg(root);
    command
}

fn run_success(command: &mut Command) -> Output {
    let output = command.output().expect("switcher should start");
    assert!(
        output.status.success(),
        "status: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn create_profile(root: &Path, name: &str) {
    run_success(switcher(root).args(["create", name]));
}

fn read_recording(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

#[test]
fn profile_lifecycle_is_safe_and_visible() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");

    create_profile(&root, "work");

    let config = fs::read_to_string(root.join("work").join("config.toml")).unwrap();
    assert_eq!(config, "cli_auth_credentials_store = \"file\"\n");
    assert!(!root.join("work").join("auth.json").exists());

    let list = run_success(switcher(&root).arg("list"));
    assert_eq!(
        String::from_utf8(list.stdout).unwrap(),
        "work\n",
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );

    let path = run_success(switcher(&root).args(["path", "work"]));
    assert_eq!(
        String::from_utf8(path.stdout).unwrap().trim(),
        root.join("work").display().to_string()
    );

    run_success(switcher(&root).args(["remove", "work", "--yes"]));
    assert!(!root.join("work").exists());
}

#[test]
fn rejects_traversal_reserved_names_and_duplicate_profiles() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");

    for name in ["../escape", "with space", "CON", "é"] {
        let output = switcher(&root).args(["create", name]).output().unwrap();
        assert!(!output.status.success(), "accepted {name:?}");
    }

    create_profile(&root, "work");
    let duplicate = switcher(&root).args(["create", "work"]).output().unwrap();
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already exists"));
}

#[test]
fn refuses_to_remove_an_unowned_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    fs::create_dir_all(root.join("not-ours")).unwrap();
    fs::write(root.join("not-ours").join("keep.txt"), "keep").unwrap();

    let output = switcher(&root)
        .args(["remove", "not-ours", "--yes"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(root.join("not-ours").join("keep.txt").exists());
}

#[test]
fn noninteractive_removal_requires_yes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    create_profile(&root, "work");

    let output = switcher(&root).args(["remove", "work"]).output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("rerun with --yes"));
    assert!(root.join("work").exists());
}

#[test]
fn run_overrides_codex_home_forwards_arguments_and_exit_code() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let output_path = temporary.path().join("fake-output");
    let injection_marker = temporary.path().join("shell-was-used");
    let injection_argument = format!("$(touch {})", injection_marker.display());
    let fake_codex = make_fake_codex(temporary.path());
    create_profile(&root, "work");

    let output = switcher(&root)
        .arg("--codex-bin")
        .arg(&fake_codex)
        .args([
            "run",
            "work",
            "exec",
            "--model",
            "gpt-test",
            "argument with spaces",
            "*",
        ])
        .arg(&injection_argument)
        .env("CODEX_HOME", temporary.path().join("wrong-home"))
        .env("FAKE_CODEX_OUTPUT", &output_path)
        .env("FAKE_CODEX_EXIT", "23")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(23));
    let recorded = read_recording(&output_path);
    assert!(recorded.contains(&format!("home={}\n", root.join("work").display())));
    assert!(recorded.contains("arg=<exec>\n"));
    assert!(recorded.contains("arg=<--model>\n"));
    assert!(recorded.contains("arg=<gpt-test>\n"));
    assert!(recorded.contains("arg=<argument with spaces>\n"));
    assert!(recorded.contains("arg=<*>\n"));
    assert!(recorded.contains(&format!("arg=<{injection_argument}>\n")));
    assert!(!injection_marker.exists());
}

#[test]
fn login_and_status_are_delegated_to_codex() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let output_path = temporary.path().join("fake-output");
    let fake_codex = make_fake_codex(temporary.path());
    create_profile(&root, "work");

    run_success(
        switcher(&root)
            .arg("--codex-bin")
            .arg(&fake_codex)
            .args(["login", "work", "--device-auth"])
            .env("FAKE_CODEX_OUTPUT", &output_path)
            .env("FAKE_CODEX_EXIT", "0"),
    );
    let login = read_recording(&output_path);
    assert!(login.contains("arg=<login>\n"));
    assert!(login.contains("arg=<--device-auth>\n"));

    run_success(
        switcher(&root)
            .arg("--codex-bin")
            .arg(&fake_codex)
            .args(["status", "work"])
            .env("FAKE_CODEX_OUTPUT", &output_path)
            .env("FAKE_CODEX_EXIT", "0"),
    );
    let status = read_recording(&output_path);
    assert!(status.contains("arg=<login>\n"));
    assert!(status.contains("arg=<status>\n"));
}

#[test]
fn unsafe_credential_config_blocks_codex_launch() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let output_path = temporary.path().join("fake-output");
    let fake_codex = make_fake_codex(temporary.path());
    create_profile(&root, "work");
    fs::write(
        root.join("work").join("config.toml"),
        "cli_auth_credentials_store = \"keyring\"\n",
    )
    .unwrap();

    let output = switcher(&root)
        .arg("--codex-bin")
        .arg(&fake_codex)
        .args(["run", "work"])
        .env("FAKE_CODEX_OUTPUT", &output_path)
        .env("FAKE_CODEX_EXIT", "0")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!output_path.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("file-backed"));

    let checked_list = switcher(&root)
        .arg("--codex-bin")
        .arg(&fake_codex)
        .args(["list", "--check"])
        .env("FAKE_CODEX_OUTPUT", &output_path)
        .env("FAKE_CODEX_EXIT", "0")
        .output()
        .unwrap();
    assert!(!checked_list.status.success());
    assert!(!output_path.exists());
}

#[test]
fn owned_profile_can_be_listed_and_removed_when_codex_config_is_invalid() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    create_profile(&root, "work");
    fs::write(root.join("work").join("config.toml"), "not valid toml = [").unwrap();

    let list = run_success(switcher(&root).arg("list"));
    assert_eq!(
        String::from_utf8(list.stdout).unwrap(),
        "work\n",
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );

    run_success(switcher(&root).args(["remove", "work", "--yes"]));
    assert!(!root.join("work").exists());
}

#[test]
fn separate_profiles_can_run_concurrently() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let fake_codex = make_fake_codex(temporary.path());
    create_profile(&root, "work");
    create_profile(&root, "personal");

    let work_output = temporary.path().join("work-output");
    let personal_output = temporary.path().join("personal-output");
    let mut work = switcher(&root);
    work.arg("--codex-bin")
        .arg(&fake_codex)
        .args(["run", "work"])
        .env("FAKE_CODEX_OUTPUT", &work_output)
        .env("FAKE_CODEX_EXIT", "0");
    let mut personal = switcher(&root);
    personal
        .arg("--codex-bin")
        .arg(&fake_codex)
        .args(["run", "personal"])
        .env("FAKE_CODEX_OUTPUT", &personal_output)
        .env("FAKE_CODEX_EXIT", "0");

    let mut work_child = work.spawn().unwrap();
    let mut personal_child = personal.spawn().unwrap();
    assert!(work_child.wait().unwrap().success());
    assert!(personal_child.wait().unwrap().success());

    let work_record = read_recording(&work_output);
    let personal_record = read_recording(&personal_output);
    assert!(work_record.contains(&root.join("work").display().to_string()));
    assert!(personal_record.contains(&root.join("personal").display().to_string()));
}

#[cfg(unix)]
#[test]
fn preserves_non_utf8_codex_arguments() {
    use std::os::unix::ffi::OsStringExt;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let output_path = temporary.path().join("fake-output");
    let fake_codex = make_fake_codex(temporary.path());
    create_profile(&root, "work");
    let argument = std::ffi::OsString::from_vec(vec![b'f', 0x80, b'o']);

    let output = switcher(&root)
        .arg("--codex-bin")
        .arg(&fake_codex)
        .args(["run", "work", "--"])
        .arg(argument)
        .env("FAKE_CODEX_OUTPUT", &output_path)
        .env("FAKE_CODEX_EXIT", "0")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        fs::read(output_path)
            .unwrap()
            .windows(3)
            .any(|window| window == [b'f', 0x80, b'o'])
    );
}

#[cfg(unix)]
#[test]
fn refuses_symlinked_profile() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("profiles");
    let outside = temporary.path().join("outside");
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, root.join("work")).unwrap();

    let output = switcher(&root).args(["path", "work"]).output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("symbolic link"));
}

fn make_fake_codex(directory: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let path = directory.join("fake-codex");
        fs::write(
            &path,
            "#!/bin/sh\nif [ \"${1-}\" = \"--version\" ]; then\n  echo 'codex-cli 0.test'\n  exit 0\nfi\n{\n  printf 'home=%s\\n' \"$CODEX_HOME\"\n  for argument in \"$@\"; do\n    printf 'arg=<%s>\\n' \"$argument\"\n  done\n} > \"$FAKE_CODEX_OUTPUT\"\nexit \"${FAKE_CODEX_EXIT:-0}\"\n",
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[cfg(windows)]
    {
        let path = directory.join("fake-codex.cmd");
        fs::write(
            &path,
            "@echo off\r\nif \"%~1\"==\"--version\" (\r\n  echo codex-cli 0.test\r\n  exit /b 0\r\n)\r\n> \"%FAKE_CODEX_OUTPUT%\" echo home=%CODEX_HOME%\r\n:args\r\nif \"%~1\"==\"\" goto done\r\n>> \"%FAKE_CODEX_OUTPUT%\" echo arg=^<%~1^>\r\nshift\r\ngoto args\r\n:done\r\nexit /b %FAKE_CODEX_EXIT%\r\n",
        )
        .unwrap();
        path
    }
}
