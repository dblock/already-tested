# Changelog

All notable changes are documented here using [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- [#23](https://github.com/dblock/already-tested/pull/23): Add shared `--ci` selection with GitHub Actions and automatic defaults, a matching Rust API, and a neutral CI command - [@dblock](https://github.com/dblock).

### Fixed

- [#22](https://github.com/dblock/already-tested/pull/22): Verify claims against the queued PR merge instead of PR head so untested base-branch changes run CI - [@dblock](https://github.com/dblock).

### Changed

- [#26](https://github.com/dblock/already-tested/pull/26): Require measured elapsed time in claim labels, such as `lint - passed (29s)`, in contributor and user agent instructions - [@dblock](https://github.com/dblock).
- [#25](https://github.com/dblock/already-tested/pull/25): Require agents to publish eligible local CI claims after validation instead of omitting portable lint claims when a platform-specific environment is uncertain - [@dblock](https://github.com/dblock).
- [#19](https://github.com/dblock/already-tested/pull/19): Upgrade rust-crate-release to v0.1.2 to cache release-tool compilation dependencies without skipping release validation - [@dblock](https://github.com/dblock).

## [0.2.0] - 2026-11-13

### Added

- [#17](https://github.com/dblock/already-tested/pull/17): Add read-only local claim verification with text/JSON reports and distinct match, no-match, and error exit codes - [@dblock](https://github.com/dblock).
- [#16](https://github.com/dblock/already-tested/pull/16): Add automatic GitHub job summaries explaining matching claims and when to run normal CI without changing skip decisions - [@dblock](https://github.com/dblock).
- [#14](https://github.com/dblock/already-tested/pull/14): Add reproducible claim-generation and verification benchmarks using real Git repositories - [@dblock](https://github.com/dblock).
- [#12](https://github.com/dblock/already-tested/pull/12): Support check globs in claims and workflow gates, requiring each claim to cover the gate's entire scope - [@dblock](https://github.com/dblock).
- [#11](https://github.com/dblock/already-tested/pull/11): Default to a blanket workflow claim and document a prerequisite job that skips dependent CI jobs before runner allocation - [@dblock](https://github.com/dblock).

### Changed

- [#18](https://github.com/dblock/already-tested/pull/18): Document environment matching, explicit project-approved version differences, and honest agent disclosures - [@dblock](https://github.com/dblock).
- [#15](https://github.com/dblock/already-tested/pull/15): Organize detailed user documentation and complete workflow examples under `docs/`, preserving root design and contributor guides - [@dblock](https://github.com/dblock).
- [#14](https://github.com/dblock/already-tested/pull/14): Simplify scope verification and reuse committed snapshots across candidate claims to avoid repeated glob compilation and Git reads without changing matching behavior - [@dblock](https://github.com/dblock).
- [#10](https://github.com/dblock/already-tested/pull/10): Explain agent test-run claims in the README introduction and link to the introductory blog post - [@dblock](https://github.com/dblock).

## [0.1.1] - 2026-11-12

### Fixed

- [#9](https://github.com/dblock/already-tested/pull/9): Allow release tests to run against an already-dated changelog during release validation - [@dblock](https://github.com/dblock).

### Added

- [#8](https://github.com/dblock/already-tested/pull/8): Show the published crates.io version in a README badge - [@dblock](https://github.com/dblock).

### Changed

- [#3](https://github.com/dblock/already-tested/pull/3): Lead README setup with GitHub Actions and agent instructions, and document version-pinned release installers - [@dblock](https://github.com/dblock).
- [#6](https://github.com/dblock/already-tested/pull/6): Adopt rust-crate-release 0.1.1 for crates.io/GitHub publication and validated version/changelog updates, with Rust tasks for verified Dylint setup and native asset publication - [@dblock](https://github.com/dblock).
- [#7](https://github.com/dblock/already-tested/pull/7): Use the released verifier for repository CI, prepare the next patch development version, and adopt Keep a Changelog with synchronized release metadata - [@dblock](https://github.com/dblock).

## [0.1.0] - 2026-11-12

### Added

- [#1](https://github.com/dblock/already-tested/pull/1): Port the complete JavaScript project to Rust with compatible claims, native platform binaries, automatic installation, separate CI workflows, and general-purpose/spelling Dylint checks following `excalc-rs` - [@dblock](https://github.com/dblock).
- [#2](https://github.com/dblock/already-tested/pull/2): Simplify release-ready installation, GitHub Actions setup, and local claims, with AI agent instructions - [@dblock](https://github.com/dblock).

### Fixed

- [#4](https://github.com/dblock/already-tested/pull/4): Fix native Windows ARM64 selection under Git Bash and replacement of installed Windows binaries - [@dblock](https://github.com/dblock).

### Changed

- [#5](https://github.com/dblock/already-tested/pull/5): Speed up Dylint CI with verified upstream binaries and cached tooling, and fix driver builds after the Clippy branch rename - [@dblock](https://github.com/dblock).

[Unreleased]: https://github.com/dblock/already-tested/compare/v0.2.0...master
[0.2.0]: https://github.com/dblock/already-tested/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/dblock/already-tested/compare/v0.1.0...v0.1.1

[0.1.0]: https://github.com/dblock/already-tested/releases/tag/v0.1.0
