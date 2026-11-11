<img src="assets/logo.svg" alt="Green rubber stamp reading Already Tested with a check mark" width="160">

# already-tested

[![Test](https://github.com/dblock/already-tested/actions/workflows/test.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/test.yml) [![Coverage](https://github.com/dblock/already-tested/actions/workflows/coverage.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/coverage.yml) [![Lint](https://github.com/dblock/already-tested/actions/workflows/lint.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/lint.yml) [![Package](https://github.com/dblock/already-tested/actions/workflows/package.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/package.yml)

Run your existing CI checks locally, then let GitHub Actions skip matching work.

**For trusted contributors only:** claims bind committed files and the workflow, but do not prove tests ran. Anyone who can edit the PR description can forge a claim.

## Installation

Install locally on macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/dblock/already-tested/master/scripts/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
already-tested --version
```

On Windows, run in PowerShell:

```powershell
$installer = "$env:TEMP\already-tested-install.ps1"
Invoke-WebRequest https://raw.githubusercontent.com/dblock/already-tested/master/scripts/install.ps1 -OutFile $installer
& $installer
$env:PATH = "$HOME\.local\bin;$env:PATH"
already-tested --version
```

The installer selects your OS/CPU and verifies the download. Supports Linux, macOS, and Windows on x86_64/ARM64; requires Git 2.29+, not Node or Rust. PATH changes above affect the current shell; add the directory to your shell profile for future sessions.

## Usage

With the [workflow configured](#github-actions), run your complete CI check locally and commit the tested files. Then:

```sh
already-tested claim --workflow .github/workflows/test.yml --check test-linux \
  --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'
```

Paste the **entire output into the PR description**, not a comment. After tracked source or workflow changes, retest, commit, and replace the claim. The CLI records your statement; it does not run the command.

## Configuration

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

The Action installs its verifier automatically. Keep `fetch-depth: 2` and the `edited` trigger. Use a distinct `check` ID for each job/OS/runtime and the same ID/workflow path in the CLI. Pin a reviewed full Action commit SHA instead of the release tag for immutable dependencies.

Matching claims skip conditioned steps while keeping the job's required check. Missing or stale claims run normal CI; pushes always run CI. By default, all tracked files and the executing workflow are covered; untracked/ignored files are excluded. Claims require a clean tracked working tree.

Keep independent CI for external services, secrets, or environments you cannot reproduce locally. See [options, filters, installation settings, and limitations](DESIGN.md).

### AI Agents

Add this to your project's `AGENTS.md`, replacing the command, workflow path, and check ID with your job's values:

```markdown
## Local CI Claims

- Run the complete `./script/ci` check in the job's OS, CPU, and toolchain environment.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --check test-linux --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
- Never claim checks you did not run or other environments; omit the claim and let CI run instead.
```

## Contributing

[Contributing](CONTRIBUTING.md) | [Design](DESIGN.md) | [Changelog](CHANGELOG.md) | [Releasing](RELEASING.md) | [Agent Instructions](AGENTS.md). The original JavaScript implementation is [already-tested-js](https://github.com/dblock/already-tested-js).

## License

[MIT](LICENSE).
