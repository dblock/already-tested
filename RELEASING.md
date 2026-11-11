# Releasing

## Status

Unreleased. No GitHub Release or crates.io publication has been performed. These instructions authorize neither. CI builds installable archives without publishing.

## Version and Validation

Keep unreleased `0.1.0` unchanged. Before later bumps, verify existing GitHub/crate releases and follow [versioning rules](AGENTS.md#versioning-and-releases). Synchronize Cargo manifest/lock, Action binary version, and changelog.

Validate the exact revision with tests, lint, coverage, native package installation, and `cargo package --locked --allow-dirty`. Package CI must build all six target archives, verify checksums, and exercise each installed executable on its native OS/CPU. Linux uses musl and Windows uses static CRT linking; inspect unexpected runtime dependencies.

Retain the six exact `.tar.gz` files and adjacent `.sha256` files from the same commit. Never modify or rebuild archives between validation and upload. Local archive/installer tests use `ALREADY_TESTED_DOWNLOAD_BASE_URL=file:///...`.

## GitHub Release

Only after explicit authorization, create a matching immutable version tag at the validated commit and configure the `release` environment restricted to the default branch. Dispatch Release at that same revision with `publish=true`. With `publish=false`, it only builds artifacts.

The workflow requires the existing matching tag, checks six archive checksums, and publishes those exact artifacts using the short-lived GitHub token. Verify the tag points to the built commit before dispatch; never move an existing tag. Do not store registry tokens or request credentials in chat.

After publication, verify every release asset checksum and install with both automatic selection and a pinned version. Verify Action installation against that same release, not a newly rebuilt tree or moving HEAD. Only then update release status/documentation. Until publication, installer downloads fail clearly and the Action falls back to normal CI.

If publication is interrupted, inspect the existing release, tag, asset list, and retained checksums; complete only missing steps. Never overwrite published artifacts or repeat publication blindly. Crates.io publication is optional, separate, and requires explicit authorization and verified name availability.
