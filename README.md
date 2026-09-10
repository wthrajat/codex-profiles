# cxprof

[![Crates.io](https://img.shields.io/crates/v/cxprof.svg)](https://crates.io/crates/cxprof)
[![CI](https://github.com/wthrajat/codex-profiles/actions/workflows/ci.yml/badge.svg)](https://github.com/wthrajat/codex-profiles/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/cxprof.svg)](LICENSE)

Use more than one Codex account without signing in and out. Each profile gets
its own `CODEX_HOME`, so personal and work sessions can run side by side.

## Install

Install the [Codex CLI](https://github.com/openai/codex), then install `cxprof`:

```console
cargo install cxprof
```

Prebuilt binaries are available on the
[releases page](https://github.com/wthrajat/codex-profiles/releases).

## Quick start

Set up each account once:

```console
cxprof create personal
cxprof login personal

cxprof create work
cxprof login work
```

Start Codex with either account:

```console
cxprof run personal
cxprof run work
```

Anything after the profile name in `cxprof run` is passed to Codex:

```console
cxprof run work -- exec "review this repository"
```

## Commands

| Command | What it does |
| --- | --- |
| `cxprof create <name>` | Create a profile |
| `cxprof login <name>` | Sign in through Codex |
| `cxprof run <name>` | Run Codex with a profile |
| `cxprof list` | List profiles |
| `cxprof status <name>` | Check sign-in status |
| `cxprof path <name>` | Show the profile directory |
| `cxprof doctor [name]` | Check setup and isolation |
| `cxprof remove <name>` | Remove a profile |

Run `cxprof --help` or `cxprof <command> --help` for full usage.

## Security

Authentication follows the
[official OpenAI documentation](https://learn.chatgpt.com/docs/auth). `cxprof`
uses the documented `CODEX_HOME`, file-backed credential storage, and normal
`codex login` flow. It does not bypass authentication, usage limits, or
workspace policies.

Codex owns the credentials in every profile. `cxprof` never sees your password
and never reads, copies, or prints `auth.json`. Profile directories are private
to the current user on Unix, and removal is limited to directories created by
`cxprof`.

No third-party tool can guarantee that an account will never be suspended.
Using `cxprof` does not change the rules that apply to your OpenAI account.

Use `cxprof path <name>` to find a profile and `cxprof doctor <name>` to check
its setup. Managed Codex settings can override file-backed credentials, so
profile isolation may not be available on every company device. See
[SECURITY.md](SECURITY.md) for the full security boundary.
