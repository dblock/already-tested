# Releasing

Releases publish native binaries and installer scripts to GitHub. Registry publication is not required for installation. Publishing, tags, credentials, and release dispatch each require explicit authorization.

## Version and Validation

Check GitHub Releases before choosing a version. Follow [versioning rules](AGENTS.md#versioning-and-releases) and synchronize Cargo manifest/lock, Action binary version, and changelog.

Validate the exact revision with tests, lint, coverage, native package installation, and `cargo package --locked --allow-dirty`. Package CI must build all six target archives, verify checksums, and exercise each installed executable on its native OS/CPU. Linux uses musl and Windows uses static CRT linking; inspect unexpected runtime dependencies.

Retain the six exact `.tar.gz` files, adjacent `.sha256` files, and both installer scripts from the same commit. Never modify or rebuild artifacts between validation and upload. Local archive/installer tests use `ALREADY_TESTED_DOWNLOAD_BASE_URL=file:///...`.

## GitHub Release

Only after explicit authorization, create a matching immutable version tag at the validated commit and configure the `release` environment restricted to the default branch. Dispatch Release at that same revision with `publish=true`. With `publish=false`, it only builds artifacts.

The workflow requires the existing matching tag, checks six archive checksums, and publishes those exact artifacts using the short-lived GitHub token. Verify the tag points to the built commit before dispatch; never move an existing tag. Do not store registry tokens or request credentials in chat.

Upload `scripts/install.sh` and `scripts/install.ps1` from the validated commit as release assets named `install.sh` and `install.ps1`. The README pins both the installer URL (`/releases/download/v<version>/<asset>`) and its binary version argument. Latest-release URLs (`/releases/latest/download/<asset>`) are also available.

After publication, verify both versioned installer URLs, confirm the scripts match the tagged sources, and check every archive checksum. Install with both automatic selection and a pinned version. Verify Action installation against that same release, not a rebuilt tree or moving HEAD.

If publication is interrupted, inspect the existing release, tag, asset list, and retained checksums; complete only missing steps. Never overwrite published artifacts or repeat publication blindly. Crates.io publication is optional, separate, and requires explicit authorization and verified name availability.
