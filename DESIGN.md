# Design

## Trust and Interfaces

Trusted PR-description claims avoid duplicate CI against matching committed source and workflows. They are not signing, authentication, execution evidence, or a security boundary. Anyone who can edit the description can forge them.

The stable interfaces are `already-tested claim`, the v1 claim/hash protocol, and Action inputs `check`/`files` with string output `skip=true` or `skip=false`. Project languages and toolchains are independent of this implementation. The JavaScript predecessor is [already-tested-js](https://github.com/dblock/already-tested-js).

Rust implements the shared library, CLI, and `already-tested action` adapter. Git 2.29+ is the only external runtime command. No Node/npm, Rust runtime, network service, automatic test execution, or test-runner adapter is required by the executable.

## CLI and Claim Format

```sh
already-tested claim --workflow .github/workflows/test.yml --check unit
```

Paths are repository-relative even when called from a subdirectory. Workflows must be tracked `.yml`/`.yaml` files directly under `.github/workflows/`. Check IDs use 1-128 ASCII letters, numbers, periods, underscores, colons, slashes, or hyphens, beginning with a letter/number.

| Option | Meaning |
| --- | --- |
| `--files <pattern>` | Repeat to select tracked files; default: all |
| `--summary <text>` | Expandable label; default: `<check> - passed` |
| `--command <text>` | Command actually validated; recorded, never executed |
| `--environment <text>` | Tested project's toolchain/OS; never inferred |
| `--raw` | Canonical comment only; incompatible with display metadata |
| `--help` | Usage, also printed with no arguments |
| `--version` | Package version; must be used alone |

Default stdout is one `details` section, final newline, no progress logs. Its tight bullets contain supplied command/environment, workflow, check, commit, and visible content SHA-256. The canonical comment is hidden inline on the hash bullet. Concatenate sections without outer bullets or blank separators; GitHub controls visual margins.

Display metadata must be nonempty, single-line, and NUL-free. Summary HTML is escaped and inline code fences safely contain backticks. Metadata is trusted prose, not a hash input. Errors exit one on stderr and produce no claim; success exits zero. Help/version need no repository.

The canonical representation is byte-compatible with JavaScript v1:

```text
<!-- already-tested:v1 {"workflow":".github/workflows/test.yml","check":"unit","hash":"sha256:0000000000000000000000000000000000000000000000000000000000000000"} -->
```

Verification searches for the exact expected canonical string among any description text. Alternate JSON formatting, unknown versions, malformed claims, and changed hashes cannot match.

## File Selection and Hashing

Omitting filters selects all tracked files. Otherwise use the same Git pathspecs on both sides: repeated CLI `--files` and newline-separated Action `files`. Trim ECMAScript whitespace, deduplicate, and sort by UTF-16 code units, matching JavaScript. Equivalent-looking policies remain distinct. Attribute-based pathspecs and selections matching no tracked files are errors.

```sh
already-tested claim --workflow .github/workflows/test.yml --check unit \
  --files 'src/**' --files 'test/**'
```

Always include the executing workflow. Hash JSON `["already-tested",1,workflow,check,patterns]` plus LF. For each file sorted by UTF-16 path order, hash JSON `[path,mode,byteLength]`, LF, then exactly that many raw blob bytes. Use SHA-256 and JavaScript-compatible compact JSON escaping. Commit IDs are not hash inputs; identical inputs across commits or Git object formats can reuse claims.

The CLI rejects all staged/unstaged tracked changes, regardless of filters, and hashes HEAD. Ignored/untracked files are excluded: commit all intended inputs. The Action hashes PR-head source, not a synthetic merge tree, but uses the actual executing workflow commit. Base-workflow changes therefore invalidate old claims.

Temporary Git indexes evaluate pathspecs without changing the caller's index. Read committed blobs, disable replacement objects, never execute claim text or working-tree filters, and never fetch automatically. Each Git command's output is capped at 64 MiB. Regular/executable/binary files work; selected symlinks, submodules, and invalid UTF-8 paths are errors.

## GitHub Action

The composite Action invokes a platform-selected native binary. No separate CLI installation step is needed in the workflow.

| Input/Output | Meaning |
| --- | --- |
| `check` input | Required job/environment ID; must match CLI `--check` |
| `files` input | Optional newline-separated Git pathspecs; must match CLI `--files` |
| `version` input | Binary release; defaults to the Action's version, currently `v0.1.0` |
| `skip` output | String `true` for a matching claim; otherwise `false` |

Use `if: steps.tested.outputs.skip != 'true'` on each replaceable step, with `tested` as the Action step ID. Pin a reviewed Action commit for immutable workflow dependencies.

Only `pull_request` events can skip CI. Trigger on `opened`, `synchronize`, `reopened`, and `edited`; use depth-2 checkout to retain PR-head and executing-workflow commits. Custom/reusable workflows must make those specific commits available too. Distinguish check IDs across jobs and environments.

Missing/stale/malformed claims, missing objects, unsupported inputs, installation/download failures, and other operational verification errors request normal CI. Failure to write `GITHUB_OUTPUT` fails the Action instead of pretending to deliver a decision. A matching claim skips conditioned steps, not the enclosing workflow/check.

Hashes do not prove execution, environment equivalence, external state, dependencies outside selected files, or compatibility with a newer base. Preserve independent integration/platform checks and branch protection.

## Binary Distribution

| Platform | Target |
| --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-musl` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |
| Windows ARM64 | `aarch64-pc-windows-msvc` |

Linux uses musl static linking; Windows statically links the MSVC runtime. macOS uses OS-provided system libraries. There is no single universal binary. Installers detect OS/CPU, including Apple Silicon under Rosetta and native Windows architecture, then select `already-tested-<target>.tar.gz`.

Each archive contains the executable and MIT license, with an adjacent `.sha256` file. Installers verify SHA-256 and executable version before replacing an existing install. These checks detect corruption, not an untrusted release publisher.

POSIX installation accepts `--version v0.1.0`, `--bin-dir PATH`, and `--print-target`; Windows accepts `-Version`, `-BinDir`, and `-PrintTarget`. Defaults: latest release and `~/.local/bin`. They do not modify shell profiles. Ordinary use requires Git; installation uses OS shell, HTTPS download, archive, and checksum facilities.

For a mirror or local archive test, set `ALREADY_TESTED_DOWNLOAD_BASE_URL` to an HTTPS directory or `file://` URL containing the archive and checksum. Other schemes are rejected; empty overrides use defaults. POSIX also accepts `ALREADY_TESTED_VERSION` and `ALREADY_TESTED_BIN_DIR`.

For example, pin a CLI version or change its destination:

```sh
curl -fsSL https://raw.githubusercontent.com/dblock/already-tested/master/scripts/install.sh \
  | sh -s -- --version v0.1.0 --bin-dir "$HOME/.local/bin"
```

`ALREADY_TESTED_BINARY` selects an already-built verifier for Action integration tests; ordinary users leave it unset.

## Development and Artwork

Use the pinned stable Rust toolchain, committed Cargo.lock, `cargo test --locked --workspace --all-targets`, and `cargo xtask lint`. The `xtask` maintenance crate combines formatting, Clippy, structural document checks, and Dylint's pinned general-purpose and spelling suites. Dylint is the preferred framework for additional lint rules; specialized Markdown/YAML/shell checks remain where no library covers them. Separate Test, Coverage, Lint, and Package workflows follow `excalc-rs`, with distinct Dylint and formatting/Clippy jobs and parallel Coveralls reporting. Package checks exercise real installed archives. See [Contributing](CONTRIBUTING.md).

Preserve the original editable MIT-licensed [green SVG](assets/logo.svg) from already-tested-js: `#28a745`, transparent background, accessible title/description, system fonts, and outer margin. Keep it first in README at width 160. It is not a certification mark. No artwork changes or third-party fonts were introduced by the port.

Keep this document focused on current behavior and constraints, not implementation diaries.
