# Working on already-tested

Read [Contributing](CONTRIBUTING.md) and [Design](DESIGN.md) before editing. Claims are trusted statements, not execution evidence. This Rust implementation must not require Node/npm or a compiler on end-user systems.

## Implementation and Tests

Keep shared protocol/verification logic in `src/`, integration fixtures in `tests/`, maintenance tooling in `xtask/`, and installers/Action adapter in `scripts/`. Use supported public Rust APIs and the pinned stable toolchain.

Preserve JavaScript v1 byte compatibility: compact JSON framing, UTF-16 path/pattern sort, ECMAScript trimming, committed blobs/modes, selection policy, and executing workflow regardless of filters. Use isolated temporary indexes, disable replacement objects, preserve caller state, and cap Git output. Never execute claim metadata or fetch silently.

Reject selected symlinks/submodules/invalid UTF-8/attribute pathspecs. Never skip CI for missing/stale/malformed claims, failed binary installation, or operational errors. Output-write failure must fail the Action.

Test behavior with real Git/CLI/Action processes and installed native archives. Keep conformance fixtures self-contained: tests must not invoke Node. Inspect coverage gaps without trivial assertions or excluding real code.

## Validation and Distribution

Before every development push, run `cargo test --locked --workspace --all-targets` and `cargo xtask lint` immediately before committing. For binary, installer, interface, or packaging changes also run `cargo xtask package`; reuse results only for unchanged content/configuration.

Use `cargo llvm-cov --locked --workspace --all-targets --summary-only` to inspect coverage. Follow [setup and test conventions](CONTRIBUTING.md#setup-and-validation), including verified current stable tools, separate Test/Coverage/Lint/Package workflows, explicit `matrix.entry` lists, and parallel Coveralls reporting.

Keep all six platform binaries, automatic native selection, HTTPS/checksum verification, and failure-safe replacement. Preserve CLI/claim/Action contracts across distribution changes. Do not claim cross-platform validation from only one local host; native CI validates other archives.

Follow [local claim mappings](CONTRIBUTING.md#local-ci-claims). Claim only complete checks actually run in the correct OS/CPU, with honest command/environment metadata. Use all tracked files; regenerate after changes. Paste default disclosures adjacent without outer bullets or blank separators, with canonical comments hidden inline.

## Documentation and Git

Keep user-facing content minimal and language-neutral; use `already-tested`, not implementation paths. Use single-line paragraphs and title-case headings/display names. Prefer Dylint libraries for new lint rules in this repository; keep its general-purpose and spelling suites in a separate CI job from formatting/Clippy/document checks, following `excalc-rs`. Retain specialized checks when no configured Dylint library covers them. Review `.spellcheck.txt` vocabulary instead of suppressing genuine typos. Preserve the editable MIT SVG and README placement; see [artwork instructions](DESIGN.md#development-and-artwork).

Use `<username>/<topic>` branches from `master` in the existing checkout; worktrees only for parallel tasks. Inspect status/worktrees and preserve unrelated work. Keep one coherent PR commit and squash merges. No direct default-branch development pushes, force-pushes to `master`, unauthorized merges/publication/credentials/tags/releases/dispatch.

Amend task-owned commits without asking after verifying ownership, recording remote tips, reconciling changes, and validating. Use `--force-with-lease` against the recorded SHA; never plain force or another task's work.

Append meaningful changes above the Next placeholder in linked PR/author format; predict then verify the PR number. Minor typos need no entry. Include the Copilot co-author trailer and disclose AI authorship on behalf of `@dblock` in PRs/comments. Close issues through PR references.

Report pending CI without waiting after local validation. After authorized merges, prune-fetch and verify merged state and no dirty worktrees/unmerged work/dependents before cleaning task-owned worktrees/branches. Never delete active PR branches, dirty worktrees, or `master`.

## Versioning and Releases

Verify publication before bumping; ask if uncertain. Keep unreleased `0.1.0` unchanged. After release, bump in the change PR: features increment minor, reset patch for `0.x`, preserve patch for `1.x` onward. Breaking `0.x` changes increment minor/reset patch; ask before breaking `1.x`+ major bumps. Synchronize Cargo manifest/lock, Action version, and Next heading.

For released breaking changes, including `0.x`, add `UPGRADING.md` with source/target versions, before/after examples, and the introducing PR; link from README.

[Release instructions](RELEASING.md) are provisional. Build and validate exact archives, but never publish crates/releases, create tags, dispatch publishing, configure credentials, or overwrite versions/tags without explicit authorization. Binary installation needs no registry publication.
