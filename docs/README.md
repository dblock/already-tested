# Documentation

`already-tested` lets agents publish trusted local test-run claims in pull-request descriptions. GitHub Actions skips matching work against the same committed files and executing workflow. Claims are not evidence that tests ran; keep CI enabled on your default branch.

Read [Use Agent Test Run Claims to Skip CI](https://code.dblock.org/2026/11/12/use-agent-test-run-claims-to-skip-ci.html) for the motivation, setup, and measured CI savings.

## Getting Started

Start with the [README quick start](../README.md#installation), then choose how much work to skip:

| Guide | What You Will Build |
| --- | --- |
| [Skip Individual Steps](github-actions.md) | A named gate before setup, dependency installation, and tests, including matrix jobs |
| [Skip All Remaining Jobs](skip-jobs.md) | A small prerequisite job that skips every covered job before allocating runners |
| [Agent Instructions](agents.md) | Instructions for generating and maintaining honest claims |
| [CLI Installation and Reference](cli.md) | Platform installers, claim commands, display metadata, and options |
| [Configuration](configuration.md) | Check globs, shared gates, file filters, and complete matching examples |
| [Troubleshooting](troubleshooting.md) | Why CI ran, checkout requirements, errors, and limitations |

## Internals and Development

[Design and Claim Protocol](../DESIGN.md) | [Contributing and Benchmarks](../CONTRIBUTING.md) | [Releasing](../RELEASING.md) | [Changelog](../CHANGELOG.md) | [Repository Agent Instructions](../AGENTS.md).
