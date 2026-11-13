<img src="assets/logo.svg" alt="Green rubber stamp reading Already Tested with a check mark" width="160">

# already-tested

[![Crates.io](https://img.shields.io/crates/v/already-tested.svg)](https://crates.io/crates/already-tested) [![Test](https://github.com/dblock/already-tested/actions/workflows/test.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/test.yml) [![Coverage](https://github.com/dblock/already-tested/actions/workflows/coverage.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/coverage.yml) [![Lint](https://github.com/dblock/already-tested/actions/workflows/lint.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/lint.yml) [![Package](https://github.com/dblock/already-tested/actions/workflows/package.yml/badge.svg)](https://github.com/dblock/already-tested/actions/workflows/package.yml)

Your AI agent already ran the tests and lints locally. Why make GitHub Actions repeat the same work? `already-tested` lets agents add local test-run claims to pull-request descriptions so CI can skip matching checks against the same committed files and workflow. Add a workflow gate and agent instructions; missing or stale claims run normal CI.

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
      - uses: dblock/already-tested@v0.3.0
        id: tested
        with:
          check: test-linux
      - name: Run Tests
        if: steps.tested.outputs.skip != 'true'
        run: ./script/ci
```

## Usage

Add this to your project's `AGENTS.md` or `.github/copilot-instructions.md`:

```markdown
## Local CI Claims

- Install `already-tested` using https://github.com/dblock/already-tested/blob/master/docs/cli.md#installation.
- Run `already-tested agent-instructions show` and follow its guidance before generating or updating claims.
```

The agent installs the CLI, reads its release-matched guidance, and inspects your CI workflows. Keep any project-specific environment exceptions in your own instructions. See [agent setup](docs/agents.md) for details.

## Documentation

[Detailed Guides and Examples](docs/README.md) | [Blog Post: Use Agent Test Run Claims to Skip CI](https://code.dblock.org/2026/11/12/use-agent-test-run-claims-to-skip-ci.html) | [Contributing](CONTRIBUTING.md) | [Changelog](CHANGELOG.md).

For Azure Repos and Azure Pipelines, see the [minimal Azure gate](docs/azure-pipelines.md).

## License

[MIT](LICENSE).
