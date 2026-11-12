<img src="assets/logo.svg" alt="Green rubber stamp reading Already Tested with a check mark" width="160">

# already-tested

[![Crates.io](https://img.shields.io/crates/v/already-tested.svg)](https://crates.io/crates/already-tested) [![Test](https://github.com/dblock/already-tested/actions/workflows/test.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/test.yml) [![Coverage](https://github.com/dblock/already-tested/actions/workflows/coverage.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/coverage.yml) [![Lint](https://github.com/dblock/already-tested/actions/workflows/lint.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/lint.yml) [![Package](https://github.com/dblock/already-tested/actions/workflows/package.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/package.yml)

Run your existing CI checks locally, then let GitHub Actions skip matching work.

**For trusted contributors only:** claims bind committed files and the workflow, but do not prove tests ran. Anyone who can edit the PR description can forge a claim.

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
      - uses: dblock/already-tested@v0.1.0
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

## Usage

### AI Agents

Add this to your project's `AGENTS.md`, replacing the command, workflow path, and check ID with your job's values:

```markdown
## Local CI Claims

- Install `already-tested` if needed using the [local CLI instructions](https://github.com/dblock/already-tested#local-cli).
- Run the complete `./script/ci` check in the job's OS, CPU, and toolchain environment.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --check test-linux --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
- Never claim checks you did not run or other environments; omit the claim and let CI run instead.
```

## Configuration

By default, all tracked files and the executing workflow are covered; untracked/ignored files are excluded. Claims require a clean tracked working tree. The CLI records your statement; it does not run the command. See [options, filters, installation settings, and limitations](DESIGN.md).

### Local CLI

Agents use the CLI to generate claims. Install on macOS or Linux:

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.1.0/install.sh | sh -s -- --version v0.1.0
export PATH="$HOME/.local/bin:$PATH"
already-tested --version
```

On Windows, run in PowerShell:

```powershell
$installer = "$env:TEMP\already-tested-install.ps1"
Invoke-WebRequest https://github.com/dblock/already-tested/releases/download/v0.1.0/install.ps1 -OutFile $installer
& $installer -Version v0.1.0
$env:PATH = "$HOME\.local\bin;$env:PATH"
already-tested --version
```

The installer selects your OS/CPU and verifies the archive. Supports Linux, macOS, and Windows on x86_64/ARM64; requires Git 2.29+. PATH changes above affect the current shell; add the directory to your shell profile for future sessions.

## Contributing

[Contributing](CONTRIBUTING.md) | [Design](DESIGN.md) | [Changelog](CHANGELOG.md) | [Releasing](RELEASING.md) | [Agent Instructions](AGENTS.md). The original JavaScript implementation is [already-tested-js](https://github.com/dblock/already-tested-js).

## License

[MIT](LICENSE).
