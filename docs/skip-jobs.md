# Skip All Remaining Jobs

[Documentation](README.md) | [Skip Individual Steps](github-actions.md)

A small prerequisite job can verify one blanket claim and skip every covered job before those jobs allocate runners. The agent asserts it ran all covered checks and environments; `already-tested` does not discover your jobs or prove they ran.

An omitted or empty Action `check` defaults to `*`, as does omitting CLI `--check`.

## Complete Workflow

Save this as `.github/workflows/test.yml`, replace `<version>` with your chosen Action tag, and replace the commands with your full checks. Both test and lint are covered by the same blanket claim:

```yaml
name: Test
on:
  push:
  pull_request:
    types: [opened, synchronize, reopened, edited]
permissions:
  contents: read
jobs:
  verify:
    name: Verify Local Claim
    runs-on: ubuntu-latest
    outputs:
      skip: ${{ steps.claims.outputs.skip }}
    steps:
      - name: Check Out Source
        uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - name: Check Local Claim
        uses: dblock/already-tested@<version>
        id: claims

  test:
    name: Test
    needs: verify
    if: >-
      ${{ !cancelled() &&
          (needs.verify.result != 'success' ||
           needs.verify.outputs.skip != 'true') }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
      - run: ./script/test

  lint:
    name: Lint
    needs: verify
    if: >-
      ${{ !cancelled() &&
          (needs.verify.result != 'success' ||
           needs.verify.outputs.skip != 'true') }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
      - run: ./script/lint
```

Add your normal setup and dependency-install steps inside each covered job. They will be skipped with that job. Apply both `needs` and `if` to every job you intend to cover; leaving a job outside the gate keeps it independent. Preserve required-check names and existing dependencies.

The condition explicitly checks the verifier result, so missing claims, stale claims, verifier failures, and skipped verification run normal CI. The `!cancelled()` status check avoids GitHub's implicit success-only dependency guard while preserving workflow cancellation. Pushes always run CI. A matching claim skips dependent jobs, not the enclosing workflow; it does not cancel the run.

The [job summary](github-actions.md#job-summaries) explains the verification decision on the workflow run's Summary page. Dependent jobs still appear as skipped in GitHub; the report explains the gate without changing job status or posting a PR comment.

The verifier only compares committed content, so its Linux runner can verify claims about other environments. That does not let the agent claim other environments without testing them.

## Agent Instructions

Add this complete section to your project's `AGENTS.md`, replacing the workflow and commands:

```markdown
## Local CI Claims

- Install `already-tested` using https://github.com/dblock/already-tested/blob/master/docs/cli.md#installation.
- Read `.github/workflows/test.yml` and identify every job covered by its prerequisite gate, including every matrix environment.
- Run all covered jobs' complete commands: `./script/test` and `./script/lint`, with the same OS, CPU, and toolchain as each job. Measure the full sequence's wall-clock time during its final successful run and round to the nearest whole second. If any covered check or environment was not tested, do not make a blanket claim; let CI run.
- Commit the tested files and leave the tracked working tree clean.
- Run `already-tested claim --workflow .github/workflows/test.yml --summary 'All workflow checks - passed (<seconds>s)' --command './script/test && ./script/lint' --environment '<actual OS, CPU, and toolchain versions>'`, replacing `<seconds>` with the measured elapsed time. Omit `--check` to claim the entire gate's scope.
- Paste the entire output into the PR description, preserving its hidden claim.
- Create a separate claim for each workflow you fully tested. Match any workflow file selection with repeated `--files` arguments.
- After tracked source or workflow changes, rerun the covered checks, commit, and replace the claim.
```

The CLI records your statement; it does not execute or time `--command`. Follow the [timing guidance](agents.md#measured-check-time); never invent elapsed time or rerun successful checks solely for timing. For a multi-platform workflow, record the actual commands and environments you used, not just the environment of the verifier.

## Existing Dependencies

If a job already depends on `build`, keep that dependency and add `verify`. Preserve the original success requirement explicitly:

```yaml
needs: [verify, build]
if: >-
  ${{ !cancelled() &&
      needs.build.result == 'success' &&
      (needs.verify.result != 'success' ||
       needs.verify.outputs.skip != 'true') }}
```

This is a replacement job-level fragment, not another complete workflow. If `build` is itself covered by the blanket gate, give it the same verification condition. When there is no verified claim, build runs and the downstream job runs only after a successful build. When a claim matches, both skip. Do not copy the simple condition over an existing branch restriction or other job-specific condition; combine them without weakening the original requirement.

Keep jobs that need real artifacts, secrets, or external services outside the gate when local checks cannot reproduce their work. Review downstream conditions and branch-protection requirements before relying on skipped jobs.

## Selective Job Groups

To skip just test jobs while leaving lint independent, give the prerequisite Action `check: 'test-*'` and have the agent generate `--check 'test-*'` only after running every covered test environment. See the [complete selective-group workflow](configuration.md#complete-group-example).

The claim must cover the entire gate scope. One `test-linux` claim cannot satisfy a `test-*` gate, and a `test-*` claim cannot satisfy the default `*` gate. Separate partial claims are never combined into a blanket claim.
