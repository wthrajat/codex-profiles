# Releasing

Releases are driven by version tags. A normal push to `main` runs CI but cannot
publish the crate.

## One-time setup

1. Create `https://github.com/wthrajat/codex-profiles` and add it as the
   `origin` remote.
2. Create a crates.io API token scoped to `cxprof` and add it to
   the GitHub repository as an Actions secret named `CRATES_IO_TOKEN`.
3. Protect `main` and require the `CI` workflow before merging.

The first token may need broader crates.io scope because the crate does not
exist until its first publish. Replace it with a crate-scoped token afterward.

## Release checklist

1. Update `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` with the release
   version and date.
2. Run the complete local validation suite:

   ```console
   cargo fmt --all --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets --locked
   cargo doc --no-deps --locked
   cargo package --locked
   cargo publish --dry-run --locked
   ```

3. Merge the release change to `main`, then create and push the matching tag:

   ```console
   git tag -s v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

The release workflow verifies that the tag matches `Cargo.toml`, repeats the
package checks, builds four platform binaries, publishes to crates.io, and
creates a GitHub release with archives and SHA-256 checksums.

If publishing succeeds but GitHub release creation fails, rerun the failed
workflow. Do not reuse or move an existing version tag.
