<img src="assets/logo.svg" alt="Green rubber stamp reading Already Tested with a check mark" width="160">

# already-tested

[![Crates.io](https://img.shields.io/crates/v/already-tested.svg)](https://crates.io/crates/already-tested) [![Test](https://github.com/dblock/already-tested/actions/workflows/test.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/test.yml) [![Coverage](https://github.com/dblock/already-tested/actions/workflows/coverage.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/coverage.yml) [![Lint](https://github.com/dblock/already-tested/actions/workflows/lint.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/lint.yml) [![Package](https://github.com/dblock/already-tested/actions/workflows/package.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/package.yml)

Your AI agent already ran the tests and lints locally. Why make GitHub Actions repeat the same work? `already-tested` lets agents add local test-run claims to pull-request descriptions so CI can skip matching checks against the same committed files and workflow. Add a gate to your workflow and instructions to `AGENTS.md`; missing or stale claims run normal CI.

Of course, anyone who can edit the pull-request description can forge a claim. Don't ever disable CI on `main`.

## Installation

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

## Usage

Add this to your project's `AGENTS.md`, matching the command, workflow, and check to your CI:

```markdown
## Local CI Claims

- Install `already-tested` using https://github.com/dblock/already-tested/blob/master/docs/cli.md#installation.
- Run the complete `./script/ci` check in the same OS, CPU, and toolchain environment as `test-linux`. Never claim checks or environments you did not test.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --check test-linux --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
```

## Documentation

[Detailed Guides and Examples](docs/README.md) | [Blog Post: Use Agent Test Run Claims to Skip CI](https://code.dblock.org/2026/11/12/use-agent-test-run-claims-to-skip-ci.html) | [Contributing](CONTRIBUTING.md) | [Changelog](CHANGELOG.md).

## License

[MIT](LICENSE).
