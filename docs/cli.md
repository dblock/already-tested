# CLI Installation and Reference

[Documentation](README.md) | [Agent Instructions](agents.md)

The CLI generates claims after you have run tests and verifies claims against committed files; it never runs the command you supply. Ordinary use requires Git 2.29+, not Cargo, Rust, or Node/npm.

## Installation

Use the same version for the CLI and workflow Action. When changing the installation version, update both the download URL and installer argument.

### macOS and Linux

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.3.0/install.sh | sh -s -- --version v0.3.0
export PATH="$HOME/.local/bin:$PATH"
already-tested --version
already-tested agent-instructions show
```

### Windows

Run in PowerShell:

```powershell
$installer = "$env:TEMP\already-tested-install.ps1"
Invoke-WebRequest https://github.com/dblock/already-tested/releases/download/v0.3.0/install.ps1 -OutFile $installer
& $installer -Version v0.3.0
$env:PATH = "$HOME\.local\bin;$env:PATH"
already-tested --version
already-tested agent-instructions show
```

The installer selects Linux, macOS, or Windows on x86_64/ARM64 and verifies the archive checksum and executable version before replacing an install. PATH changes above affect the current shell; add the directory to your shell profile for future sessions. Archive checksums detect corruption, not an untrusted release publisher.

### Custom Installation Directory

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.3.0/install.sh |
  sh -s -- --version v0.3.0 --bin-dir "$HOME/tools/already-tested"
export PATH="$HOME/tools/already-tested:$PATH"
already-tested agent-instructions show
```

On Windows, pass `-BinDir "$HOME\tools\already-tested"` to the downloaded installer. The default directory is `~/.local/bin`. POSIX installers accept `--print-target`; Windows accepts `-PrintTarget` to inspect native target selection.

For mirrors or archive tests, `ALREADY_TESTED_DOWNLOAD_BASE_URL` accepts an HTTPS directory or `file://` URL containing the archive and checksum. Other schemes are rejected; an empty override uses the default release URL. POSIX also accepts `ALREADY_TESTED_VERSION` and `ALREADY_TESTED_BIN_DIR`. See [binary distribution](../DESIGN.md#binary-distribution) for platform targets.

## Agent Instructions

```sh
already-tested agent-instructions show
already-tested agent-instructions update
already-tested agent-instructions update --path docs/local-ci.md
already-tested agent-instructions update --force
```

`show` prints the consumer guidance bundled with the installed CLI version, offline and without Git or a repository. Read it before creating claims, and keep project-specific check mappings and environment policies in your own `AGENTS.md` or `.github/copilot-instructions.md`.

`update` detects `.github/copilot-instructions.md` at the Git working tree root. If present, it writes `.github/instructions/already-tested.instructions.md` and updates a marked reference in the Copilot file, leaving `AGENTS.md` untouched. Otherwise it writes `.already-tested/agent-instructions.md` and updates `AGENTS.md`, creating it if needed. Handwritten content is preserved; nothing is staged.

`--path` overrides the guide's repository-relative destination, not the detected entry point. Repeat the override on upgrades. Copilot `.instructions.md` files include `applyTo: "**"` front matter. Repeating unchanged updates is a no-op; older unedited guides are replaced atomically. Local edits and unrecognized guides require deliberate `--force`, which never overrides handwritten policy. Symbolic links, targets that are not regular files, parent traversal, Git metadata, and either policy entry point as the guide destination are refused. See [adoption and upgrades](agents.md#adopt-in-a-project).

## CI Selection

`claim`, `verify`, and `ci` accept `--ci auto`, `--ci github-actions`, or `--ci azure-pipelines`. The default is `auto`: `GITHUB_ACTIONS=true` selects GitHub Actions, `TF_BUILD=True` selects Azure Pipelines, and neither signal falls back to GitHub Actions. Signal values are case-insensitive. If both are enabled, specify `--ci` explicitly; a conflicting environment never silently chooses one.

An explicit choice overrides detection. Remotes, workflow filenames, and repository contents are not detection signals because a repository can use both systems. When generating or verifying Azure claims locally, pass `--ci azure-pipelines`:

```sh
already-tested claim --ci azure-pipelines --workflow azure-pipelines.yml --check test-linux
already-tested verify --ci azure-pipelines --workflow azure-pipelines.yml --check test-linux < claim.md
```

Only generate claims after running the named complete checks in their required environments. CI selection controls workflow validation and adapter routing, not the v1 claim/hash format. GitHub Actions requires a workflow directly under `.github/workflows/`; Azure Pipelines accepts safe repository-relative YAML paths.

### Rust API

The shared API uses the same policy through `Options.ci`, which defaults to `Ci::Auto`:

```rust
use already_tested::{Ci, Options, create_claim};

let mut options = Options::new(".", "azure-pipelines.yml", "test-linux");
options.ci = Ci::AzurePipelines;
let claim = create_claim(&options)?;
```

`Ci::Auto.resolve()` returns the detected system or GitHub Actions fallback, and returns an error for conflicting signals. Explicit variants resolve without reading the environment. The existing claim and verification functions apply this policy without fetching data or running tests.

## Create a Claim

Run your actual complete checks, commit the tested files, then generate a claim:

```sh
./script/ci
git add src tests .github/workflows/test.yml
git commit -m "Update tested source"
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --command './script/ci' \
  --environment '<actual OS, CPU, and toolchain versions>'
```

Adapt the paths to include every intended input and the command/environment to your actual run. The tracked working tree must be clean. The claim hashes committed HEAD source and workflow; untracked and ignored files are excluded.

Copy the entire expandable output into the PR description, not into a comment or a source file. Display metadata is explanatory only: it is not hashed and cannot substitute for actually running the checks. Environment versions are not compared; record actual versions even when a [project-approved difference](agents.md#environment-matching) allows them to differ from CI. The hidden canonical comment must remain unmodified. See the [claim protocol](../DESIGN.md#cli-and-claim-format) for its exact representation.

## Options

```text
already-tested claim --workflow <path> [--ci <system>] [--check <name-or-glob>] [options]
```

| Option | Meaning |
| --- | --- |
| `--ci <system>` | `auto` (default), `github-actions`, or `azure-pipelines`; automatic detection falls back to GitHub Actions |
| `--workflow <path>` | Required tracked `.yml`/`.yaml` file; GitHub Actions requires `.github/workflows/`, while Azure accepts safe repository-relative paths |
| `--check <name-or-glob>` | Named check or glob; defaults to `*` when omitted |
| `--files <pattern>` | Repeat to select tracked files; omitted means all tracked files |
| `--summary <text>` | Expandable label; default `<check> - passed`, or `All workflow checks - passed` for `*` |
| `--command <text>` | Command already validated; recorded, never executed |
| `--environment <text>` | Environment actually validated; recorded, not compared or inferred |
| `--raw` | Canonical comment only; incompatible with summary, command, and environment metadata |
| `--help` | Usage; no repository needed |
| `--version` | Package version; must be used alone; no repository needed |

Quote globs to prevent shell expansion. Explicit CLI `--check ''` is invalid; only omission defaults to `*`. The Action also defaults an empty input to `*`.

Metadata must be nonempty, single-line, and NUL-free. Summary HTML is escaped and inline code safely handles backticks. Success exits zero; invalid options or repository state exit one on stderr without emitting a claim.

## Custom Summary and Raw Output

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --summary 'Linux tests - passed' \
  --command './script/ci' \
  --environment '<actual Linux, CPU, and toolchain versions>'

already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --raw
```

The first command includes a custom disclosure label; the second emits only the hidden canonical comment. Prefer the normal disclosure so reviewers can see what the agent says it ran.

## Verify a Claim

Verify a generated claim or an entire PR description before publishing it. The command reads UTF-8 text from standard input by default, or from `--body-file`:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check 'test-*' > claim.md

already-tested verify \
  --workflow .github/workflows/test.yml \
  --check test-linux < claim.md

already-tested verify \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --body-file claim.md \
  --json
```

Only generate the first claim after actually running every covered test environment. Keep `claim.md` temporary and do not commit it. The verifier accepts both the complete disclosure and a raw canonical comment, including claims embedded in other description text.

`--workflow`, `--check`, and repeated `--files` have the same meanings as the gate. Omitted `--check` defaults to `*`; quote globs. File paths passed to `--body-file` are relative to the current directory, while workflow and selection paths remain repository-relative. Claim-only display options and `--raw` are not accepted by `verify`.

The verifier checks committed HEAD source and workflow, even when the working tree or index has changes. It never changes tracked files or the caller's index, runs tests, fetches Git objects, reads a PR from GitHub, or edits its description. A local match does not cover uncommitted changes or guarantee a match against GitHub's [queued merge content or executing workflow revision](github-actions.md#events-and-checkout).

### Exit Codes

| Code | Meaning | Agent Response |
| --- | --- | --- |
| `0` | A canonical claim covers the entire gate with matching committed content and selection | The claim matches the local preview |
| `1` | No matching claim | Check the gate and selection; retest and regenerate stale claims |
| `2` | Invalid arguments, unreadable input, invalid repository/policy, or verification error | Fix the error; never treat it as a match |

These exit codes apply only to `verify`; claim generation and the Action retain their existing exit behavior. Workflow, scope, file selection, and repository validation still run when no supported claim is present. Input must be valid UTF-8 and no larger than 1 MiB.

### JSON Reports

`--json` writes one JSON object to stdout for a match, no match, or error, without human-readable diagnostics mixed into the stream. Verdicts include the requested workflow/check and the actual matched scope:

```json
{"skip":true,"reason":"matched","message":"A canonical claim covers this gate and committed content.","workflow":".github/workflows/test.yml","check":"test-linux","matched_check":"test-*"}
```

| Reason | Meaning |
| --- | --- |
| `matched` | Verification succeeded; `matched_check` identifies the canonical claim's scope |
| `missing_claim` | No supported claim marker was found in the supplied text |
| `no_matching_claim` | No canonical claim covers the gate with matching committed files, file selection, and workflow |
| `error` | Input or verification failed; `message` contains the error |

`skip` is false and `matched_check` is null for no match or an error. Error reports need not include workflow/check fields because argument parsing may have failed. Without `--json`, verdicts go to stdout and errors go to stderr.

A non-match cannot identify which file changed or distinguish a content change from a changed selection policy. Claims contain a combined digest, not a file manifest, so the verifier does not guess. Display metadata is not evidence that tests ran.

## CI Adapters

`already-tested ci` publishes a skip decision through the selected CI system. It accepts `--ci`, `--workflow`, `--check`, repeated `--files`, and `--body-file`; claim display options and `--json` are not supported. Selection defaults to automatic detection with the same GitHub Actions fallback as the shared API.

For GitHub Actions, the event supplies source, description, and executing workflow; omitted check/file options use the Action inputs. An explicit workflow must match the executing workflow. `already-tested action` remains a fixed GitHub Actions entry point for compatibility with existing binary releases and wrappers.

For Azure Pipelines, `already-tested ci --ci azure-pipelines --workflow azure-pipelines.yml --check test-linux` reads the description from `stdin` or `--body-file` and emits the Azure `skip` output variable. The executing pipeline path must be explicit. See the [Azure Pipelines guide](azure-pipelines.md) for the PowerShell gate, authentication, required build context, and limitations.
