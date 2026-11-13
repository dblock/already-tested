# Troubleshooting

[Documentation](README.md) | [Design and Trust Model](../DESIGN.md#trust-and-interfaces)

Normal CI is the fallback, not an error: a claim can skip work only when verification succeeds.

Start with the gate's [job summary](github-actions.md#job-summaries) on the workflow run's Summary page. It shows a matched scope or a normal-CI fallback reason without requiring you to search the job logs.

For a local preview, save the claim or PR description and run [verify](cli.md#verify-a-claim) with the workflow's gate and file selection. Its JSON report distinguishes a missing supported claim, no matching claim, and an operational error. It checks local committed HEAD, not uncommitted changes or a different executing workflow revision.

## CI Ran Despite a Claim

Check these in order:

1. The event is `pull_request`, not `push`. Pushes intentionally run CI, including default-branch pushes.
2. The full claim is in the PR description, with its hidden canonical comment intact. PR comments are not read.
3. The workflow includes the `edited` activity type, so changing the description triggers another run.
4. The workflow path and check identity match. For globs, the claim must cover the gate's entire scope, not merely overlap.
5. The CLI and Action use matching versions.
6. File filters match on both sides. A default all-files claim does not satisfy a filtered gate.
7. The tested files and executing workflow are unchanged. Changes brought in from the base branch can require a new claim.
8. The required PR-head and executing-workflow commits exist in the runner's checkout. The normal PR checkout uses `fetch-depth: 2`; full history also works.
9. Installation and Git operations succeeded. Missing objects, unsupported platforms, invalid inputs, or verification resource limits request normal CI.

Compare the Action's `skip` output with the step/job conditions. Even a matching claim cannot skip a step you left unconditioned or a job you left outside a prerequisite gate.

## Cannot Generate a Claim

Commit all tracked changes first, including staged changes and dirty files outside any selection. Ignored and untracked files are not included; commit intended test inputs before claiming them.

Use a tracked `.yml` or `.yaml` workflow directly under `.github/workflows/`. Paths are repository-relative, not relative to the current subdirectory. File filters must select tracked files and cannot use attribute-based pathspecs. Selected symlinks, submodules, invalid UTF-8 paths, and Git output above 64 MiB are rejected.

Use real single-line command/environment metadata, quote shell patterns, and do not combine `--raw` with display metadata. `already-tested --help` and `already-tested --version` work without a repository.

## Failed or Skipped Prerequisite

Use the full job condition in [Skip All Remaining Jobs](skip-jobs.md#complete-workflow), including `!cancelled()` and the verifier result check. A plain `if: needs.verify.outputs.skip != 'true'` can inherit GitHub's success-only dependency behavior and skip work after a prerequisite failure.

Preserve existing dependencies and success requirements when adding the gate. Jobs that require build artifacts or external services may need to stay independent even when local unit tests passed.

## Operational Limits

Binary installation and verification errors request normal CI. Failure to write `GITHUB_OUTPUT` instead fails the Action, because the verifier cannot safely communicate its decision.

Scope compilation is limited to 1 MiB each for the DFA and determinization workspace; comparisons visit at most 16,384 state pairs over check names up to 128 characters. Verification examines at most 32 distinct relevant scopes including the gate itself. Simplify a complex pattern or remove obsolete claims if these limits are reached.

## What Claims Do Not Prove

Anyone who can edit a PR description can forge a claim. Hashes bind committed input bytes, check identity, file selection, and the executing workflow; they do not prove execution, environment equivalence, external state, or dependencies outside the selection. Keep CI on the default branch and retain independent checks for work that was not actually reproduced locally.
