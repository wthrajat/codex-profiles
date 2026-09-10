# Security

`codex-profile-switcher` sits next to authentication state, so its security
boundary must remain deliberately narrow.

## Invariants

- The application must never read or parse `auth.json`.
- The application must never accept credentials as command-line arguments.
- Login and logout must be delegated to the installed Codex executable.
- Profile homes must be private to the current user.
- Child processes must receive exactly one intended `CODEX_HOME`.
- Profile names must not be usable for path traversal or shell injection.
- Removal must be restricted to directories created and marked by this tool.
- Diagnostics and errors must not include tokens or complete environments.

Any change that weakens an invariant requires a documented security review.

## Reporting a vulnerability

Do not include credentials, `auth.json` contents, access tokens, or private
environment data in a public issue.

Once the public repository exists, use its private security-advisory channel.
Until then, report vulnerabilities to the maintainer through a private channel
and include only the minimum redacted reproduction necessary.

## Supported versions

Security fixes are provided for the latest crates.io release. Please update to
the newest version before reporting a vulnerability.
