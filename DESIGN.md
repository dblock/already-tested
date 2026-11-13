# Design

## Trust and Interfaces

Trusted PR-description claims avoid duplicate CI against matching committed source and workflows. They are not signing, authentication, execution evidence, or a security boundary. Anyone who can edit the description can forge them.

The stable interfaces are `already-tested claim`, `already-tested verify`, `already-tested ci`, the v1 claim/hash protocol, and Action inputs `check`/`files` with string output `skip=true` or `skip=false`. Project languages and toolchains are independent of this implementation. The JavaScript predecessor is [already-tested-js](https://github.com/dblock/already-tested-js).

Rust implements the shared library, CLI, and CI adapters. `already-tested ci` is the neutral CI entry point; `already-tested action` remains a fixed GitHub Actions compatibility entry point, including for wrappers selecting older binary releases. Git 2.29+ is the only external runtime command. No Node/npm, Rust runtime, network service, automatic test execution, or test-runner adapter is required by the executable.

`Options.ci` and CLI `--ci` share `auto`, `github-actions`, and `azure-pipelines` selection. The Rust `Ci` type defaults to `Auto`; resolution uses only case-insensitive true values in `GITHUB_ACTIONS` and `TF_BUILD`. One enabled signal selects its system; neither falls back to GitHub Actions; both require an explicit choice. Explicit variants bypass detection. Never infer the system from remotes, workflow filenames, or repository contents. Selection controls workflow validation and `ci` adapter routing, not canonical claim bytes or hash framing.

## CLI and Claim Format

```sh
already-tested claim --workflow .github/workflows/test.yml --check unit
```

Paths are repository-relative even when called from a subdirectory. Workflows must be tracked `.yml`/`.yaml` files; GitHub Actions retains its restriction to files directly under `.github/workflows/`, while Azure Pipelines permits paths such as `azure-pipelines.yml`. Absolute paths, traversal, backslashes, colons, and control characters are rejected. Named check IDs use 1-128 ASCII letters, numbers, periods, underscores, colons, slashes, or hyphens, beginning with a letter/number. Checks may also be case-sensitive globs using [globset syntax](https://docs.rs/globset/latest/globset/#syntax): `*`, `?`, character classes/ranges/negation, and brace alternatives. Backslash escaping is enabled on every platform; ordinary wildcards match `/` too. Patterns must be 1-128 ASCII characters without whitespace or control characters and match at least one valid named ID. Omitting `--check` defaults to `*`: a trusted statement that all work covered by that workflow's gate ran.

| Option | Meaning |
| --- | --- |
| `--ci <system>` | CI system; default `auto` with GitHub Actions fallback |
| `--check <identifier>` | Named check or glob; default: `*` |
| `--files <pattern>` | Repeat to select tracked files; default: all |
| `--summary <text>` | Expandable label; default: `<check> - passed`, or `All workflow checks - passed` for `*` |
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

Verification searches for the exact expected canonical string among any description text. If a candidate claim has a different check scope, it must cover the gate's entire scope, and its canonical hash is recomputed using the claim's own pattern. Alternate JSON formatting, unknown versions, malformed claims, and changed hashes cannot match. Named-check v1 bytes and the hash algorithm are unchanged.

`already-tested verify` reads a claim or description from standard input or `--body-file` and checks committed HEAD source and workflow. Input is UTF-8, capped at 1 MiB. It ignores dirty working-tree/index content and never fetches or edits PRs. Exit codes distinguish match (`0`), no match (`1`), and input/verification error (`2`). Optional JSON reports include `skip`, a reason/message, and the matched scope; verdicts also identify the requested workflow/check. Error JSON goes to stdout rather than mixing human diagnostics into the stream. See [local verification](docs/cli.md#verify-a-claim).

The shared `verify_committed_claim` entry point validates the repository and policy even when no supported claim is supplied. Existing `verify_claim` and `verify_claim_details` retain their missing-claim fast path for CI. Both use the same canonical matching logic, committed snapshots, and resource limits. A non-match does not identify individual changed files or distinguish changed content from changed selection policy.

## File Selection and Hashing

Omitting filters selects all tracked files. Otherwise use the same Git pathspecs on both sides: repeated CLI `--files` and newline-separated Action `files`. Trim ECMAScript whitespace, deduplicate, and sort by UTF-16 code units, matching JavaScript. Equivalent-looking policies remain distinct. Attribute-based pathspecs and selections matching no tracked files are errors.

```sh
already-tested claim --workflow .github/workflows/test.yml --check unit \
  --files 'src/**' --files 'test/**'
```

Always include the executing workflow. Hash JSON `["already-tested",1,workflow,check,patterns]` plus LF. For each file sorted by UTF-16 path order, hash JSON `[path,mode,byteLength]`, LF, then exactly that many raw blob bytes. Use SHA-256 and JavaScript-compatible compact JSON escaping. Commit IDs are not hash inputs; identical inputs across commits or Git object formats can reuse claims.

The CLI rejects all staged/unstaged tracked changes, regardless of filters, and hashes HEAD. Ignored/untracked files are excluded: commit all intended inputs. The Action hashes the queued source commit from `GITHUB_SHA`, normally GitHub's synthetic PR merge, and uses the actual executing workflow commit from `GITHUB_WORKFLOW_SHA`. Base-branch changes to selected files or the executing workflow invalidate old claims. Different commit IDs alone do not invalidate identical covered content.

Temporary Git indexes evaluate pathspecs without changing the caller's index. Read committed blobs, disable replacement objects, never execute claim text or working-tree filters, and never fetch automatically. Each Git command's output is capped at 64 MiB. Regular/executable/binary files work; selected symlinks, submodules, and invalid UTF-8 paths are errors.

Each verification loads one committed snapshot and reuses its entries and blob bytes across candidate scopes. Each candidate is still hashed with its own check identity. Snapshots are never cached across calls, and a distinct executing-workflow revision still supplies the workflow bytes regardless of source filters.

## GitHub Action

The composite Action invokes a platform-selected native binary. No separate CLI installation step is needed in the workflow.

| Input/Output | Meaning |
| --- | --- |
| `check` input | Named check or glob; omitted or empty defaults to `*`; a claim must cover its entire scope |
| `files` input | Optional newline-separated Git pathspecs; must match CLI `--files` |
| `version` input | Binary release; defaults to the Action's version |
| `skip` output | String `true` for a matching claim; otherwise `false` |

Use `if: steps.tested.outputs.skip != 'true'` on each replaceable step, with `tested` as the Action step ID. Pin a reviewed Action commit for immutable workflow dependencies.

Omit `check`, leave it empty, or explicitly set `check: '*'` to require coverage of all valid check IDs. A `test-*` claim covers a `test-linux` or `test-l*` gate, but a `test-linux` claim does not cover `test-*`, and `test-*` does not cover `*`. Mere overlap is insufficient. One claim must cover the entire gate; separate partial claims are not combined. Agents must run every check and environment covered by their claim's scope.

Scope containment uses bounded deterministic automata over valid named IDs, not matching glob text against other glob text or enumerating claims in the PR as an inventory. Compilation is limited to 1 MiB each for the DFA and determinization workspace; comparisons visit at most 16,384 state pairs and consider IDs up to 128 characters. Verification examines at most 32 distinct relevant scopes including the gate itself. Invalid or empty scopes and exceeded resource limits request normal CI.

For all-or-nothing job skipping, expose `skip` as a prerequisite job output and gate each covered job with `needs` and `if`. Include a status expression such as `!cancelled()` and test the prerequisite result so a failed verifier runs normal CI instead of silently skipping dependent jobs. Existing dependencies and conditions must remain intact. This pattern avoids allocating dependent runners but adds a prerequisite runner and scheduling delay when CI must run; it does not cancel the workflow or skip independent jobs. See [Skip All Remaining Jobs](docs/skip-jobs.md) for a complete workflow and agent instructions.

Only `pull_request` events can skip CI. Trigger on `opened`, `synchronize`, `reopened`, and `edited`; use depth-2 checkout to retain queued source and executing-workflow commits. Custom/reusable workflows must make those specific commits available too and test that same queued source; checkout HEAD cannot override verification. Missing or invalid source IDs request normal CI without falling back to PR head. Distinguish check IDs across jobs and environments.

Missing/stale/malformed claims, missing objects, unsupported inputs, installation/download failures, and other operational verification errors request normal CI. Failure to write `GITHUB_OUTPUT` fails the Action instead of pretending to deliver a decision. Matching claims skip conditioned steps or dependent jobs, never the enclosing workflow itself.

`verify_claim_details` retains the canonical matching claim's scope; `verify_claim` preserves the Boolean interface. The Action appends an escaped, bounded report to `GITHUB_STEP_SUMMARY` after writing its decision to `GITHUB_OUTPUT`. Reports show the gate, matched scope, workflow, and source commit when verification completes, or a normal-CI fallback reason otherwise. They never treat display metadata as evidence or infer which downstream jobs actually skipped. Summary failures only warn; installer failures also produce a report without weakening the normal-CI fallback. See [job summaries](docs/github-actions.md#job-summaries).

Hashes do not prove execution, environment equivalence, external state, dependencies outside selected files, or compatibility with a newer base. Preserve independent integration/platform checks and branch protection.

## Azure Pipelines

The native `ci --ci azure-pipelines` adapter reads a supplied description from `stdin` or `--body-file` with the same UTF-8/1 MiB limit as `verify`. It only permits skips for `BUILD_REASON=PullRequest` and `BUILD_REPOSITORY_PROVIDER=TfsGit`. The queued build's `SYSTEM_PULLREQUEST_SOURCECOMMITID` supplies source and `BUILD_SOURCEVERSION` supplies the executing pipeline revision; both must be full commit IDs available locally. It never uses the current source revision from the PR API, fetches Git objects, changes checkout, or executes claim metadata. The explicit `--workflow`, `--check`, and repeated `--files` inputs feed the existing v1 verifier without changing GitHub claim bytes.

`scripts/azure-pipelines.ps1` retrieves the Azure Repos PR description using the explicitly mapped `System.AccessToken`, then invokes the installed native binary. It sets the named step's `skip` output to `false` before any work and only publishes `true` for one exact successful native decision. Missing binaries, REST failures, invalid context, and verification errors request normal CI. The native adapter also returns success with `skip=false` on operational errors; failure to write its output is an error. Argument parsing errors remain nonzero and the wrapper retains its initial false output.

Initial support is limited to Azure DevOps Services, Azure Repos Git, and a single-repository YAML pipeline with an explicitly supplied executing pipeline path. There is no Marketplace task, automatic binary installation, job summary, description-edit trigger, template expansion, or external configuration hashing. See the [Azure guide](docs/azure-pipelines.md) for setup and unsupported configurations.

## Binary Distribution

`already-tested agent-instructions show` prints version-matched consumer guidance embedded from `assets/agent-instructions.md`, with no repository or network access. The source crate includes the template; native archives contain the rendered document alongside the executable and license. Release publication renders standalone instructions and their checksum from the tagged template. Project contributor instructions are separate; upgrading a binary does not edit downstream policy.

`already-tested agent-instructions update` is an explicit offline operation on the current Git working tree. When `.github/copilot-instructions.md` exists, it writes `.github/instructions/already-tested.instructions.md` and adds or refreshes a managed reference in the Copilot file, leaving `AGENTS.md` untouched. Otherwise it writes `.already-tested/agent-instructions.md` and updates `AGENTS.md`, creating that entry point if absent. Handwritten content outside the marked reference stays unchanged.

`--path` overrides the guide's repository-relative destination without changing entry-point detection. Files ending in `.instructions.md` include Copilot's repository-wide `applyTo: "**"` front matter. Paths cannot escape the working tree, target Git metadata, or replace either policy entry point. Symbolic links and targets that are not regular files are rejected. Each file is replaced atomically, and identical contents are a no-op. A wrapper checksum identifies unedited managed guides from older releases offline; this is edit detection, not a security boundary. `--force` deliberately replaces customized guides, never handwritten policy. Changing destinations does not delete old copies or references in other entry points.

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

Each GitHub Release also includes `install.sh` and `install.ps1` as downloadable assets. Use versioned URLs and version arguments to pin both the installer and binary.

Release orchestration uses rust-crate-release for exact Cargo archive publication, durable preservation, changelog updates, Git tags/commits, GitHub hosting, and next-development validation. A project metadata hook keeps Action defaults synchronized while pinning user references to the published version. Native archives are independently validated before publication, retained outside the checkout, and attached unchanged afterward with tagged installer scripts. Dylint setup and native asset publication live in Rust `xtask` commands; Actions retain cache handling and thin task calls. See [Releasing](RELEASING.md) for authorization, registry setup, direct-push requirements, and partial-release recovery.

POSIX installation accepts `--version VERSION`, `--bin-dir PATH`, and `--print-target`; Windows accepts `-Version`, `-BinDir`, and `-PrintTarget`. Defaults: latest release and `~/.local/bin`. They do not modify shell profiles. Ordinary use requires Git; installation uses OS shell, HTTPS download, archive, and checksum facilities.

For a mirror or local archive test, set `ALREADY_TESTED_DOWNLOAD_BASE_URL` to an HTTPS directory or `file://` URL containing the archive and checksum. Other schemes are rejected; empty overrides use defaults. POSIX also accepts `ALREADY_TESTED_VERSION` and `ALREADY_TESTED_BIN_DIR`.

See the [CLI guide](docs/cli.md#installation) for version-pinned installation examples and the [documentation index](docs/README.md) for detailed user guides.

`ALREADY_TESTED_BINARY` selects an already-built verifier for Action integration tests; ordinary users leave it unset.

## Development and Artwork

Use the pinned stable Rust toolchain, committed Cargo.lock, `cargo test --locked --workspace --all-targets`, and `cargo xtask lint`. The `xtask` maintenance crate combines formatting, Clippy, structural document checks, and Dylint's pinned general-purpose and spelling suites. Dylint is the preferred framework for additional lint rules; specialized Markdown/YAML/shell checks remain where no library covers them. Separate Test, Coverage, Lint, and Package workflows follow `excalc-rs`, with distinct Dylint and formatting/Clippy jobs and parallel Coveralls reporting. Package checks exercise real installed archives. See [Contributing](CONTRIBUTING.md).

Preserve the original editable MIT-licensed [green SVG](assets/logo.svg) from already-tested-js: `#28a745`, transparent background, accessible title/description, system fonts, and outer margin. Keep it first in README at width 160. It is not a certification mark. No artwork changes or third-party fonts were introduced by the port.

Keep this document focused on current behavior and constraints, not implementation diaries.
