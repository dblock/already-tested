# Releasing

The manual Release workflow uses [rust-crate-release](https://github.com/dblock/rust-crate-release) to publish the exact verified Cargo archive to crates.io, push its release commit/tag, create a GitHub Release, and push the next development version. It then attaches all six native archives, their checksums, and both installer scripts. Installation still requires only the native binary, not Cargo.

Publishing, tags, credentials, protection changes, and release dispatch each require explicit authorization. Adoption does not configure or authorize any of them.

## One-Time Setup

Follow upstream [first-publication and trusted-publishing instructions](https://github.com/dblock/rust-crate-release/blob/master/docs/actions.md). crates.io requires an initial publication with a short-lived API token before GitHub OIDC trusted publishing can be configured. Use Cargo's interactive login, never credentials in commands, repository files, or chat. Confirm publication and revoke the token. Do not dispatch Release for that already-published version; advance and synchronize the next unpublished version first.

On crates.io, configure a trusted publisher for owner `dblock`, repository `already-tested`, workflow `release.yml`, and environment `release`. Restrict that GitHub environment to the default branch and require maintainer approval. The release job requests `actions: read`, `contents: write`, and `id-token: write`; no long-lived Cargo token secret is used.

**The standard release engine pushes release and development commits directly to `master`. PR-only protection currently blocks this.** Before authorizing a release, a maintainer must separately approve and configure a narrowly scoped release-identity exception and allow `vX.Y.Z` tags. Workflow permissions alone do not bypass rules. No protection exception or credential setup is included in this PR; normal development remains PR-only.

## Version and Validation

Check crates.io and GitHub Releases before choosing a version. Keep an unpublished version unchanged. Follow [versioning rules](AGENTS.md#versioning-and-releases) and synchronize Cargo manifest/lock, Action binary version, and the Next changelog heading.

`rcr.toml` selects the Intridea changelog format, all workspace tests, strict lint, and native package validation. Its `next-version` starts at `0.2.0` after `0.1.0`. Review this setting whenever manually choosing a release version; it must exceed that release. The metadata hook advances the setting for future releases using the project's next-minor policy.

The workflow builds and validates all six native archives before registry publication. Each native job exercises its installed executable on the matching OS/CPU; Linux uses musl and Windows uses static CRT linking. The release job installs Rust components, Dylint, and musl tooling before invoking the Action, so it does not need the optional `setup-command` added in rust-crate-release 0.1.1. Rust release validation additionally tests the prepared source and its actual `.crate`. Never substitute rebuilt archives for the retained validated artifacts.

## Run a Release

Merge the intended version and meaningful changelog notes onto `master`. Verify the one-time setup, current registry state, release rules, and next-version setting. Only after explicit authorization, dispatch Release at that revision with `publish=true`. With `publish=false`, it only builds native artifacts; it does not publish or create tags.

Do not create the tag in advance: rust-crate-release dates the changelog and creates the release commit/tag itself. It preserves and independently verifies the uploaded recovery archive, Git bundle, and source inputs before crates.io publication. The Action is pinned to the full 0.1.1 release commit, including the [validation/recovery fixes](https://github.com/dblock/rust-crate-release/pull/27) and [project setup/failure-reporting fixes](https://github.com/dblock/rust-crate-release/pull/28). No custom terminal callbacks are configured; internal failure reporting uses the upstream correction without a project-side migration.

Native artifacts remain outside the checkout so next-development validation cannot replace them. After the crate release succeeds, `cargo xtask publish-native-assets` checks that the release tag differs from the native-build source only in `CHANGELOG.md`, verifies all six checksums, and uploads the retained archives and installers read from the tag. A missing or draft GitHub Release, or one marked `prerelease`, is rejected. Existing native assets must match downloaded bytes; changed assets are never overwritten.

After publication, verify the released version and checksum on crates.io, all native assets, both versioned installer URLs, and the tag. Install using automatic selection and an explicit version. Verify Action installation against that same release, not a rebuilt tree or moving HEAD.

## Next Development Version

rust-crate-release updates Cargo versions, dates the released changelog, and creates a new Next section. The `cargo xtask release-metadata` bump hook synchronizes the Action's development defaults, keeps README/installer examples and `.github/ci-version` on the version actually published, and advances the configured future development version.

These edits are validated before the engine commits and pushes them directly to `master`; there is no next-version PR. Token-created pushes do not trigger ordinary CI, so release validation runs all configured tests, lint, and native package checks before that push. Independent platform checks still run for normal PRs.

## Recovery

Publication can succeed before a later push, GitHub Release, development update, or native upload fails. Check actual crates.io, Git, and GitHub state before retrying. Do not blindly start a fresh run against the advanced development version.

Follow upstream [recovery instructions](https://github.com/dblock/rust-crate-release/blob/master/docs/recovery.md) using the original run's retained crate, bundle, and source inputs. Recovery reconciles exact artifacts and immutable tags; never move a published tag, discard unrelated changes, or overwrite published versions.

If only native attachment failed, retain the original run's twelve native artifact files and use the original source checkout with its release tag available. After explicit authorization, rerun `cargo xtask publish-native-assets` with `NATIVE_ARCHIVE_DIRECTORY` pointing to those files, `RELEASED_VERSION` set to the released numeric version, the original build's `GITHUB_SHA`, and the usual `GH_REPO`, `GH_TOKEN`, and optional `GITHUB_STEP_SUMMARY` values. It uploads only missing assets and verifies existing ones; it never creates a release or republishes the crate.
