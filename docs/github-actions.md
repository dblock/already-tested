# Skip Individual Steps

[Documentation](README.md) | [Skip All Remaining Jobs](skip-jobs.md)

Use step gates when each job or matrix environment has its own claim, or when you want required check names to remain unchanged. The job still starts a runner, but matching claims skip the expensive setup and test steps.

## Complete Workflow

Save this as `.github/workflows/test.yml`, replacing `./script/ci` with your project's complete test command:

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
      - name: Check Out Source
        uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - name: Check Local Claim
        uses: dblock/already-tested@v0.2.0
        id: tested
        with:
          check: test-linux
      - name: Set Up Runtime
        if: steps.tested.outputs.skip != 'true'
        run: ./script/setup
      - name: Install Dependencies
        if: steps.tested.outputs.skip != 'true'
        run: ./script/dependencies
      - name: Run Tests
        if: steps.tested.outputs.skip != 'true'
        run: ./script/ci
```

Replace or remove the setup/dependency commands as appropriate. Apply the condition to every replaceable step; an unconditioned step still runs even when a claim matches. Preserve steps with independent responsibilities, such as uploading artifacts from an actual test run, and ensure they do not assume skipped steps produced files.

The Action automatically selects, downloads, and verifies its native binary. The workflow runner does not need a compiler or a separate CLI installation. Pin a reviewed Action commit when you need immutable workflow dependencies.

## Generate the Matching Claim

After running the same complete check locally on Linux with the matching CPU and toolchain, commit the tested source and use the [CLI](cli.md):

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --command './script/ci' \
  --environment '<actual OS, CPU, and toolchain versions>'
```

Paste the entire output into the PR description. See [agent instructions](agents.md) for an `AGENTS.md` section you can copy. A macOS run cannot justify a Linux claim.

## Matrix Jobs

Give each environment a distinct name. This complete workflow allows the agent to claim only the environments it actually tested:

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
    name: Test on ${{ matrix.entry.os }}
    runs-on: ${{ matrix.entry.os }}
    strategy:
      fail-fast: false
      matrix:
        entry:
          - { os: ubuntu-latest, check: test-linux }
          - { os: macos-latest, check: test-macos }
          - { os: windows-latest, check: test-windows }
    steps:
      - uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - uses: dblock/already-tested@v0.2.0
        id: tested
        with:
          check: ${{ matrix.entry.check }}
      - name: Run Tests
        if: steps.tested.outputs.skip != 'true'
        run: ./script/ci
```

Replace the test command with one supported on each runner. For example, after a complete macOS run, generate `--check test-macos`; Linux and Windows continue running without their own matching claims. Include runtime versions in the check IDs when you test more than one version on the same OS.

## Events and Checkout

Only `pull_request` events can skip work. Pushes always run CI, including on the default branch.

`fetch-depth: 2` makes the PR-head commit available beneath GitHub's default merge checkout so the Action can hash it; full history (`0`) also works. The `edited` trigger reruns the workflow when an agent adds or updates claims in the PR description, without pushing another commit.

Verification hashes PR-head source and the actual executing workflow, not arbitrary working-tree files. Custom checkout and reusable-workflow setups must make both required commits available; missing Git objects run normal CI.

## Inputs and Output

| Name | Meaning |
| --- | --- |
| `check` | Named check or glob; omitted or empty defaults to `*` |
| `files` | Newline-separated Git pathspecs; omit to cover all tracked files; match repeated CLI `--files` arguments |
| `version` | Binary release to download; defaults to the Action's version |
| `skip` output | The string `true` for a verified claim, otherwise `false` |

Compare the output string explicitly: `if: steps.tested.outputs.skip != 'true'`. Installation and verification failures request normal CI. A failure writing `GITHUB_OUTPUT` fails the Action instead of silently reporting a result.

## Job Summaries

The Action automatically appends an Already Tested report to the workflow run's Summary page, under the job that ran the gate. Open Checks on a pull request, follow the gate job to its workflow run, and select Summary. No additional workflow steps or permissions are needed.

A matching claim produces a report like this:

### Already Tested

**Matching local claim; gated work may be skipped.**

| Field | Value |
| --- | --- |
| Workflow | `.github/workflows/test.yml` |
| Gate | `test-linux` |
| Source commit | `<PR-head SHA>` |
| Matched claim | `test-*` |

Claims are trusted statements, not proof that tests ran.

The report distinguishes the gate from the matching claim's scope, so a named gate covered by a glob or blanket claim is clear. It describes the gate's decision, not a confirmed list of skipped jobs or steps; your workflow's conditions still control that work.

If no claim matches, the report says **Run CI normally**. Other reports explain that only pull-request events can skip CI, that verification failed, or that the verifier could not be installed. A missing match does not establish whether source, workflow, scope, or file selection changed, so the report does not guess.

Reports do not copy PR-description prose, declared commands, or declared environments. Dynamic values are escaped, and long values are shortened. Each gate appends its own report without replacing earlier summary content. If summary writing fails, the Action logs a warning without changing `skip` or failing the check. Outside GitHub Actions, an absent `GITHUB_STEP_SUMMARY` simply disables reports.
