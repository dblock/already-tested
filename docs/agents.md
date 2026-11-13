# Agent Instructions

[Documentation](README.md) | [CLI Reference](cli.md)

Agent instructions must name the complete commands, workflow, check identity, and tested environments. Claims are trusted statements; never ask an agent to infer that tests passed or to claim environments it did not run.

## Named Checks

Use this section with the [individual-step workflow](github-actions.md#complete-workflow). Replace the workflow, command, and environment with your project's values:

```markdown
## Local CI Claims

- Install `already-tested` if needed using https://github.com/dblock/already-tested/blob/master/docs/cli.md#installation.
- Run the complete `./script/ci` check in the job's actual OS, CPU, and toolchain environment.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --check test-linux --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
- Never claim checks you did not run or other environments; omit the claim and let CI run instead.
```

For a macOS job, change both the workflow gate and claim command to `test-macos`; do not merely relabel a Linux run. The workflow path is repository-relative even when the agent runs the CLI from a subdirectory.

## Multiple Checks and Workflows

After actually running both commands in their matching environments, an agent can generate separate claims:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --command './script/test' \
  --environment '<actual Linux, CPU, and toolchain versions>'

already-tested claim \
  --workflow .github/workflows/lint.yml \
  --check lint \
  --command './script/lint' \
  --environment '<actual OS, CPU, and lint-tool versions>'
```

Concatenate the complete outputs in the PR description without outer bullets or blank separators. Each claim binds its own workflow and selection policy. A claim for `test.yml` cannot satisfy a gate in `lint.yml`, even with the same check name.

## All Covered Work

For a prerequisite job, replace named-check instructions with the [complete blanket-claim instructions](skip-jobs.md#agent-instructions). Omitting `--check` means the agent claims all work covered by the workflow gate, including every matrix environment. Do not omit it when only one named check ran.

For a [glob claim](configuration.md#check-globs), use a quoted pattern such as `--check 'test-*'` only after running every matching check. A claim's scope must cover the gate's entire scope; overlap or a collection of partial claims is not enough.

## File Selection and Updates

If the workflow uses `files`, add the same [file filters](configuration.md#file-selection) as repeated `--files` arguments. Commit all intended inputs: untracked and ignored files are not included. The tracked working tree must be clean even when the dirty file is outside the selection.

After any tracked source or workflow edit, rerun the affected complete checks, commit, and replace stale claims. Commit identities alone are not hash inputs, but agents should not assume an existing claim remains valid after source changes.

## Verify Before Publishing

Agents can use [local verification](cli.md#verify-a-claim) to check their generated output before editing the PR description. For example, after running every test covered by `test-*` and committing the tested files:

```sh
claim_file=$(mktemp)
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check 'test-*' \
  --command '<complete commands actually run>' \
  --environment '<all actual tested OS, CPU, and toolchain versions>' > "$claim_file" &&
already-tested verify \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --body-file "$claim_file" \
  --json &&
cat "$claim_file"
status=$?
rm "$claim_file"
(exit "$status")
```

Match the verification workflow, gate scope, and repeated `--files` arguments to the Action, not necessarily to the broader claim scope. Exit `0` means the claim matches committed HEAD; exit `1` means no match; exit `2` means an input or verification error. Never publish a claim as verified after either failure status, and never infer that verification ran tests or covered dirty files.

The agent still handles GitHub access with its existing tools. Read the latest PR description, preserve human-written text and unrelated claims, and replace the old claim for the workflow/check being updated. The CLI only generates and verifies text; it does not fetch or edit PRs.
