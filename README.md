<img src="assets/logo.svg" alt="Green rubber stamp reading Already Tested with a check mark" width="160">

# already-tested

[![Crates.io](https://img.shields.io/crates/v/already-tested.svg)](https://crates.io/crates/already-tested) [![Test](https://github.com/dblock/already-tested/actions/workflows/test.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/test.yml) [![Coverage](https://github.com/dblock/already-tested/actions/workflows/coverage.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/coverage.yml) [![Lint](https://github.com/dblock/already-tested/actions/workflows/lint.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/lint.yml) [![Package](https://github.com/dblock/already-tested/actions/workflows/package.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/package.yml)

Your AI agent already ran the tests and lints locally. Why make GitHub Actions repeat the same work? `already-tested` lets agents add local test-run claims to pull-request descriptions so CI can skip matching checks against the same committed files and workflow. Add a gate to your workflow and instructions to `AGENTS.md`; missing or stale claims run normal CI.

Of course, anyone who can edit the pull-request description can forge a claim. Don't ever disable CI on `main`.

Read [Use Agent Test Run Claims to Skip CI](https://code.dblock.org/2026/11/12/use-agent-test-run-claims-to-skip-ci.html) for the motivation, setup, and measured CI savings.

## Installation

### GitHub Actions

In `.github/workflows/test.yml`, add the gate immediately after checkout. Replace `./script/ci` with your existing check, and add the `if` condition to every setup, dependency-install, and test step you want to skip:

```yaml
name: Test
on:
  push:
  pull_request:
    types: [opened, synchronize, reopened, edited]
permissions:
  contents: read
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - uses: dblock/already-tested@v0.1.1
        id: tested
        with:
          check: test-linux
      - name: Run Tests
        if: steps.tested.outputs.skip != 'true'
        run: ./script/ci
```

The Action downloads and verifies the matching platform binary automatically.

`fetch-depth: 2` makes the PR-head commit available beneath GitHub's default merge checkout so the Action can hash it; full history (`0`) also works. The `edited` trigger reruns the workflow when an agent adds or updates claims in the PR description, without pushing another commit.

In the agent instructions below, match `--check test-linux` to `check: test-linux` and `--workflow .github/workflows/test.yml` to the workflow file you edited. Use a distinct check ID for each job/OS/runtime.

Matching claims skip conditioned steps while keeping the job's required check. Missing or stale claims run normal CI; pushes always run CI.

Keep independent CI for external services, secrets, or environments you cannot reproduce locally.

### Prerequisite Job

To skip all remaining jobs, add a small prerequisite job and gate the rest on its output. An omitted or empty `check` defaults to the literal `*`: the agent claims it ran everything covered by the gate.

Use an Action and CLI release supporting blanket claims in place of `<version-with-blanket-claims>`. Keep the triggers and permissions from the example above:

```yaml
jobs:
  verify:
    name: Verify Local Claim
    runs-on: ubuntu-latest
    outputs:
      skip: ${{ steps.claims.outputs.skip }}
    steps:
      - uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - uses: dblock/already-tested@<version-with-blanket-claims>
        id: claims

  ci:
    needs: verify
    if: >-
      ${{ !cancelled() &&
          (needs.verify.result != 'success' ||
           needs.verify.outputs.skip != 'true') }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
      - run: ./script/ci
```

Repeat the dependency and condition for every job you want to skip, preserving existing conditions, dependencies, and required-check names. Missing or stale claims, verifier failures, and pushes run normal CI; workflow cancellation is preserved. Named and blanket claims are separate identities, not glob matches. See [Design](DESIGN.md#github-action) for details.

## Usage

### AI Agents

For the prerequisite-job pattern, add this to `AGENTS.md`, replacing the workflow path and `./script/ci` with your complete CI commands:

```markdown
## Local CI Claims

- Install `already-tested` if needed using the [local CLI instructions](https://github.com/dblock/already-tested#local-cli).
- Read `.github/workflows/test.yml` and run every job covered by its gate, including all matrix environments, using `./script/ci`. If anything was not tested, do not claim it; let CI run.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`. The default `*` claims all work covered by this workflow's gate.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
- Create a separate claim for each workflow you fully tested.
```

For per-step skipping, add `--check test-linux` to the claim command to match `check: test-linux` in the workflow, and run only that check's complete commands in its matching environment. Match any `files` selection with repeated `--files` arguments.

## Configuration

By default, all tracked files and the executing workflow are covered; untracked/ignored files are excluded. Claims require a clean tracked working tree. The CLI records your statement; it does not run the command. See [options, filters, installation settings, and limitations](DESIGN.md).

### Local CLI

Agents use the CLI to generate claims. Install on macOS or Linux:

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.1.1/install.sh | sh -s -- --version v0.1.1
export PATH="$HOME/.local/bin:$PATH"
already-tested --version
```

On Windows, run in PowerShell:

```powershell
$installer = "$env:TEMP\already-tested-install.ps1"
Invoke-WebRequest https://github.com/dblock/already-tested/releases/download/v0.1.1/install.ps1 -OutFile $installer
& $installer -Version v0.1.1
$env:PATH = "$HOME\.local\bin;$env:PATH"
already-tested --version
```

The installer selects your OS/CPU and verifies the archive. Supports Linux, macOS, and Windows on x86_64/ARM64; requires Git 2.29+. PATH changes above affect the current shell; add the directory to your shell profile for future sessions.

## Contributing

[Contributing](CONTRIBUTING.md) | [Design](DESIGN.md) | [Changelog](CHANGELOG.md) | [Releasing](RELEASING.md) | [Agent Instructions](AGENTS.md).

## License

[MIT](LICENSE).
