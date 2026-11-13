# Documentation

`already-tested` lets agents publish trusted local test-run claims in pull-request descriptions. GitHub Actions skips matching work against the same committed files and executing workflow. Claims are not evidence that tests ran; keep CI enabled on your default branch.

Read [Use Agent Test Run Claims to Skip CI](https://code.dblock.org/2026/11/12/use-agent-test-run-claims-to-skip-ci.html) for the motivation, setup, and measured CI savings.

## Getting Started

Start with the [README quick start](../README.md#installation) to add a workflow gate, then copy the [agent setup](../README.md#usage) into your project's `AGENTS.md` or `.github/copilot-instructions.md`. The agent installs the CLI, reads its bundled guidance, and inspects your CI workflows; you do not need to run those commands yourself.

Choose how much work to skip and adapt the complete examples:

| Guide | What You Will Build |
| --- | --- |
| [Skip Individual Steps](github-actions.md) | A named gate before setup, dependency installation, and tests, including matrix jobs |
| [Skip All Remaining Jobs](skip-jobs.md) | A small prerequisite job that skips every covered job before allocating runners |
| [Agent Instructions](agents.md) | Release-matched guidance, check mappings, and environment policies |
| [CLI Installation and Reference](cli.md) | Platform installers, bundled instructions, claim commands, and options |
| [Configuration](configuration.md) | Check globs, shared gates, file filters, and complete matching examples |
| [Troubleshooting](troubleshooting.md) | Why CI ran, checkout requirements, errors, and limitations |

## Internals and Development

[Design and Claim Protocol](../DESIGN.md) | [Contributing and Benchmarks](../CONTRIBUTING.md) | [Releasing](../RELEASING.md) | [Changelog](../CHANGELOG.md) | [Repository Agent Instructions](../AGENTS.md).
