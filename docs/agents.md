# Agent Instructions

[Documentation](README.md) | [CLI Reference](cli.md)

Agent instructions must name the complete commands, workflow, check identity, and tested environments. Claims are trusted statements; never ask an agent to infer that tests passed or hide which environments actually ran.

## Named Checks

Use this section with the [individual-step workflow](github-actions.md#complete-workflow). Replace the workflow, command, and environment with your project's values:

```markdown
## Local CI Claims

- Install `already-tested` if needed using https://github.com/dblock/already-tested/blob/master/docs/cli.md#installation.
- Run the complete `./script/ci` check in the job's actual OS, CPU, and toolchain environment, unless the project's explicit environment policy permits a specific difference.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --check test-linux --command './script/ci' --environment '<actual OS, CPU, and toolchain versions>'`.
- Paste the entire output into the PR description, preserving its hidden claim.
- After tracked source or workflow changes, retest, commit, and replace the claim.
- Never claim checks you did not run or assume an undocumented environment difference is acceptable; omit the claim and let CI run instead.
```

For a macOS job, change both the workflow gate and claim command to `test-macos`; do not merely relabel a Linux run. The workflow path is repository-relative even when the agent runs the CLI from a subdirectory.

## Environment Matching

Match the CI environment exactly by default, including OS version, CPU architecture, compiler/runtime, and any SDK or system dependencies the check uses. If a difference is not explicitly allowed, or the agent cannot establish the required versions, omit the claim and let CI run. A label such as `macos-latest` is not a fixed OS version; inspect the workflow and runner environment rather than assuming it matches a local Mac.

`already-tested` does not inspect or compare environments. `--environment` is descriptive metadata, not a hash input or an environment constraint. The Action and local `verify` can match a claim even when its declared OS or toolchain differs from CI. Neither result grants permission to skip an environment the project's policy does not cover.

### Project-Approved Differences

Maintainers may document narrow exceptions in the project's `AGENTS.md`. Define the affected check, permitted version range, and requirements that must still match. An agent must follow that policy, not invent one because versions look close.

| Difference | Default | Possible Explicit Policy |
| --- | --- | --- |
| macOS 26.6.1 versus 26.6.2 | Run CI | Permit patch differences within 26.6.x for one named check, with matching architecture and pinned tools |
| Different macOS major or minor versions | Run CI | Do not assume compatibility; require a separately approved policy or test the target version |
| Intel versus ARM64 | Run CI | Keep separate checks unless the project explicitly accepts a substitute for that check |
| Different compiler/runtime, Xcode, or SDK versions | Run CI | OS patch permission alone does not permit these differences |

For example, a maintainer may decide OS patch differences are acceptable for a particular check. This is a sample policy to review and adapt, not a built-in default or a claim that macOS patch releases always behave identically:

```markdown
## Local CI Environment Policy

- Require the CI job's exact environment unless a check-specific exception below applies.
- For `test-macos-26.6-arm64` only, accept macOS 26.6.x patch differences, such as a local run on 26.6.2 for CI on 26.6.1.
- This exception still requires ARM64, the same pinned compiler/runtime and dependency versions, and the same Xcode/SDK versions when used by the check.
- Run the complete check. Record the actual local OS, CPU, toolchain, and SDK versions in `--environment`; never substitute CI's versions in the disclosure.
- If any requirement cannot be confirmed, omit the claim and let CI run.
- Do not extend this exception to other checks, OS major/minor versions, architectures, or toolchains.
```

Under that policy, after running the complete check on the permitted local environment and committing the tested files, the agent can generate:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-macos-26.6-arm64 \
  --command './script/ci' \
  --environment 'macOS 26.6.2, ARM64, <actual compiler/runtime and SDK versions>'
```

The check names the CI work the policy allows this run to cover; the environment names what actually ran. It does not say both OS versions were tested. Keep the approved policy visible in `AGENTS.md` so reviewers can understand the substitution.

Use distinct check IDs such as `test-macos-26.6-arm64` and `test-macos-26.6-x86_64` when those environments must remain separate. Apply exceptions individually to every job covered by a glob or blanket claim. Permission for one macOS job does not justify claiming other matrix environments or all workflow checks.

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
