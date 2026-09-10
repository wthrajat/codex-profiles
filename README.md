# cxprof

[![crates.io](https://img.shields.io/crates/v/cxprof.svg)](https://crates.io/crates/cxprof)
[![CI](https://github.com/wthrajat/codex-profiles/actions/workflows/ci.yml/badge.svg)](https://github.com/wthrajat/codex-profiles/actions/workflows/ci.yml)

Rust CLI for using multiple local Codex accounts without logging one account out to use another. Each named profile is a complete, isolated `CODEX_HOME`

## Install

```console
cargo install cxprof
```

Prebuilt binaries for Linux, macOS, and Windows are also attached to each
[GitHub release](https://github.com/wthrajat/codex-profiles/releases).

To uninstall the binary, run `cargo uninstall cxprof`. Cargo
does not remove profile data; remove each profile explicitly before uninstalling
if you no longer need its Codex state.

## Quick start

```console
# Create a private Codex home and authenticate it once.
cxprof create work
cxprof login work

# Start interactive Codex with that account.
cxprof run work

# Or forward any Codex command and arguments unchanged.
cxprof run work -- exec "review this repository"
```

Create another profile the same way. Processes using different profiles can
run concurrently because each child receives a different `CODEX_HOME`.

## Commands

```text
cxprof create <name>
cxprof list [--check]
cxprof path <name>
cxprof login <name> [--device-auth]
cxprof status <name>
cxprof run <name> [--] [codex arguments...]
cxprof doctor [name]
cxprof remove <name> [--yes]
```

- `create` accepts portable names containing 1–64 ASCII letters, digits,
  underscores, or hyphens. The first character must be alphanumeric.
- `list` reads only switcher-owned metadata. `list --check` delegates each
  authentication check to Codex.
- `login`, `status`, and `run` inherit the terminal, working directory, and
  environment. Only `CODEX_HOME` is replaced.
- `run` returns the Codex process's exit code. On Unix it replaces the switcher
  process so signals and terminal control go directly to Codex.
- `remove` requires an interactive confirmation unless `--yes` is supplied and
  refuses directories without a valid ownership marker.

Use `--codex-bin PATH` to choose a specific Codex executable and `--root PATH`
to override profile storage. Both are global options and can appear before a
subcommand.

## Storage and security

Profiles are stored under:

- macOS: `~/Library/Application Support/codex-profile-switcher/profiles`
- Linux: `$XDG_STATE_HOME/codex-profile-switcher/profiles`, or
  `~/.local/state/codex-profile-switcher/profiles`
- Windows: `%LOCALAPPDATA%\codex-profile-switcher\profiles`

Every profile contains an ownership marker and this Codex setting:

```toml
cli_auth_credentials_store = "file"
```

Official OpenAI documentation says file-backed credentials are stored in
`auth.json` under `CODEX_HOME`. The switcher creates the containing profile but
never opens, parses, copies, prints, or deletes an individual credential file;
authentication remains entirely owned by Codex.

On a managed company device, an administrator can override the credential
store. Codex remains responsible for enforcing that policy; if it forces a
shared keyring, isolated account homes are not available on that device.

On Unix, profile directories use mode `0700` and tool-owned files use `0600`.
On Windows, new profiles inherit the current user's `%LOCALAPPDATA%` ACL;
`doctor` reports this platform limitation.

See [SECURITY.md](SECURITY.md) for the complete security boundary. Treat every
profile's `auth.json` as a password: never commit or share it.

## References

- [Codex authentication and credential storage](https://learn.chatgpt.com/docs/auth)
- [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
- [Codex managed configuration](https://learn.chatgpt.com/docs/enterprise/managed-configuration)
