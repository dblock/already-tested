# Contributing

## Setup and Validation

Install Rust through rustup and Git 2.29+. `rust-toolchain.toml` selects stable Rust 1.99.1 with formatting/Clippy. No Node/npm tooling is needed.

```sh
cargo install --locked cargo-dylint dylint-link --version 6.4.0
cargo test --locked --workspace --all-targets
cargo xtask lint
cargo xtask package
```

Dylint is this repository's default framework for additional lint rules. CLI tools 6.4.0 fix driver builds after the Clippy default branch changed; the general-purpose suite and [dylint-spellcheck](https://github.com/dblock/dylint-spellcheck) remain independently pinned. Dylint provisions their nightly compiler without changing normal builds. `cargo xtask lint` runs formatting, Clippy, document checks, and all configured Dylint libraries, failing on warnings. Spelling covers Rust comments and root Markdown documents; review `.spellcheck.txt` vocabulary instead of accepting genuine typos.

`cargo xtask setup-dylint` installs checksum-verified Dylint 6.4.0 binaries into Cargo's bin directory using curl and tar. It supports hosts with published upstream binaries; use the source installation above on other hosts. Both archives are checked before replacing installed tools.

The Dylint CI job calls this task on a cache miss and caches the tools, nightly compiler, driver, and built lint libraries. Its cache key covers platform, manifests, `Cargo.lock`, toolchain, setup task, and workflow configuration. It does not cache lint results; Dylint still checks each revision.

CI follows [excalc-rs](https://github.com/dblock/excalc-rs): separate Test, Coverage, Lint, and Package workflows, with separate formatting/Clippy/document and Dylint jobs in Lint. Dylint runs `cargo dylint --all --fail-on-no-libraries -- --locked --workspace --all-targets --all-features` with `DYLINT_RUSTFLAGS="-D warnings"`; structural Markdown/YAML and shell checks remain in `cargo xtask docs` where these libraries have no equivalent. There are no optional application features, so duplicate feature-mode runs are unnecessary.

On Linux, package builds require the matching musl Rust target and `musl-tools`; the Package workflow demonstrates setup. On Windows, packaging uses PowerShell and the OS-provided tar command. `cargo xtask package` builds, inspects, installs, and exercises the actual native archive, including checksum rejection. Run it on the target OS/CPU; CI covers all six targets. `cargo package --locked --allow-dirty` verifies the source crate too.

Run tests and lint immediately before committing every development push. For distribution, installer, public-interface, or packaging changes, also run native package validation. Reuse results only while checked content/configuration are unchanged; focused tests use `cargo test <name>`.

For coverage, install `cargo-llvm-cov` 0.9.1 and the toolchain's `llvm-tools-preview` component, then run `cargo llvm-cov --locked --workspace --all-targets --summary-only`. Inspect meaningful line/branch gaps; do not exclude real code or add trivial assertions to inflate coverage.

## Tests

Core tests use isolated real Git repositories, actual CLI/Action processes, SHA-1/SHA-256 object formats, and committed protocol conformance fixtures. Node is not required by tests. Maintenance tests cover heading/paragraph rules, YAML names, and platform selection. Package checks install real checksummed archives and test installed CLI/Action behavior.

Release tests use rust-crate-release's `GitProject` and `ReleaseFixture` to copy the actual source, run its metadata hook through the real engine, and inspect released/development files in a disposable remote. Fixture validation uses a focused Cargo check; the outer suite runs full validation. Only native GitHub asset operations need a small local command fixture for interrupted uploads, retries, modified assets, and source/checksum mismatches. Dylint setup tests use real local archives and checksums, including rejected downloads preserving installed tools. Never test against live credentials or publish from validation.

Keep Test, Coverage, Lint, and Package workflows separate, with explicit `matrix.entry` lists of compact inline objects, no generated axes or include/exclude. Coverage runs on Linux, macOS, and Windows and completes parallel Coveralls reporting; it always runs in CI so local claims cannot substitute for uploaded coverage. Prefer verified current stable tools while preserving supported targets.

## Local CI Claims

Claim only complete commands actually passed, against exactly the committed source, on the matching OS/CPU. Use the built CLI after committing:

```sh
target/debug/already-tested claim --workflow .github/workflows/test.yml --check test-macos-latest \
  --command 'cargo test --locked --workspace --all-targets' --environment '<actual Rust version, OS, and CPU>'
```

Test check IDs are `test-ubuntu-latest`, `test-macos-latest`, and `test-windows-latest`. The aggregate `cargo xtask lint` covers both portable `lint` and `dylint` check IDs; Package uses `package-<target>` after native `cargo xtask package --target <target>`. Never claim other platforms, focused tests as full jobs, unrun archive validation, or CI-only Coverage.

Use no file filters: repository CI hashes all tracked files. Paste unmodified default disclosures adjacent, without blank separators or outer bullets. Include actual command/environment; keep canonical claims hidden inline. Regenerate after tracked changes.

## Pull Requests

Read [Design](DESIGN.md) and [Agent Instructions](AGENTS.md). Use `<username>/<topic>` branches from `master` in the existing checkout; worktrees are for parallel tasks only. Inspect status/worktrees and preserve unrelated changes.

Keep one coherent PR commit and squash-merge. Never develop/push directly on `master`, force-push it, or merge, publish, configure credentials, create tags/releases, or dispatch publication without explicit authorization.

Amend task-owned commits without asking after verifying ownership, recording the remote tip, reconciling changes, and validating locally. Push with `--force-with-lease` against that recorded SHA; never plain force or another task's work.

Keep Markdown paragraphs on one source line and headings/workflow display names in title case. Append meaningful changes to the Next changelog placeholder in linked PR/author format, predicting then verifying the PR number. Minor typos need no entry.

Include `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` in AI-assisted commits; disclose AI authorship on behalf of `@dblock` in PRs/comments. Close issues through PR references, not manually.

After local validation, report pending CI without waiting. After authorized merges, prune-fetch, verify the GitHub merge, clean worktrees, and lack of unmerged work/dependents; switch to `master` and remove clean task-owned worktrees/merged branches. Never delete dirty worktrees, active PR branches, or `master`.

Follow [versioning rules](AGENTS.md#versioning-and-releases) and [Releasing](RELEASING.md). The explicitly authorized standard release workflow publishes to crates.io and pushes validated release/development commits; it requires separately approved release-identity protection configuration. Normal development remains PR-only. Document released breaking changes, including `0.x`, in `UPGRADING.md` and link it from README.
