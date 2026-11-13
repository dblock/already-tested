# Configuration

[Documentation](README.md) | [Action Reference](github-actions.md#inputs-and-output)

By default, claims bind all tracked files plus the executing workflow. Check identities describe the tested work; file filters describe its committed inputs. Both must match the workflow's gate.

## Environment Policy

`--environment` records the actual tested environment but is not compared or hashed. There is no automatic version tolerance: a content match is not a decision that different OS versions, CPU architectures, or toolchains are equivalent.

Use [environment-matching instructions](agents.md#environment-matching) to require exact matching by default and document any project-approved exceptions in `AGENTS.md`. Encode required distinctions in check IDs, such as `test-macos-26.6-arm64`, and keep actual versions in the disclosure even when a permitted difference applies.

## Check Globs

Both `--check` and Action `check` accept names or case-sensitive globs. Quote patterns in the shell and YAML.

| Pattern | Meaning |
| --- | --- |
| `test-*` | Every check beginning with `test-` |
| `test-?` | Exactly one character after `test-` |
| `test-[lm]*` | Checks beginning with `test-l` or `test-m` |
| `test-[!w]*` | Checks whose first character after `test-` is not `w` |
| `{test,lint}-*` | Either test or lint checks |
| `*` | All valid check names; the default blanket scope |

Named IDs are 1-128 ASCII letters, numbers, periods, underscores, colons, slashes, or hyphens, beginning with a letter/number. Patterns are 1-128 ASCII characters without whitespace or control characters and must match at least one valid named ID. [globset syntax](https://docs.rs/globset/latest/globset/#syntax) applies, including brace alternatives and supported `**` forms. Backslash escaping is enabled on every platform; ordinary wildcards match `/` too.

### Full-Scope Coverage

One claim must cover every name the gate can match, with matching files and workflow. Overlap is insufficient, and separate partial claims are never combined:

| Agent Claim | Workflow Gate | Skips? |
| --- | --- | --- |
| `*` | `test-linux` | Yes |
| `test-*` | `test-linux` | Yes |
| `test-*` | `test-l*` | Yes |
| `test-linux` | `test-*` | No |
| `test-*` | `*` | No |
| `test-l*` | `test-*x` | No |

After running every test environment, an agent can generate:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check 'test-*' \
  --command '<complete commands actually run for all test jobs>' \
  --environment '<all actual tested OS, CPU, and toolchain versions>'
```

That claim can satisfy a step gate with `check: test-linux` or a prerequisite gate with `check: 'test-*'`. It cannot satisfy a `lint` gate or the default `*` gate. Use the [full prerequisite workflow](skip-jobs.md#complete-workflow) and change only its scope when gating a group of jobs.

Matching compares the scopes' valid check names, not the spelling of one glob against another. Complexity limits deliberately request normal CI rather than attempting an unbounded comparison; see [scope limits](../DESIGN.md#github-action).

### Complete Group Example

This `.github/workflows/test.yml` gates unit and integration test jobs together, while lint always runs independently. Replace `<version>` with your chosen Action tag:

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
    name: Verify Test Claims
    runs-on: ubuntu-latest
    outputs:
      skip: ${{ steps.claims.outputs.skip }}
    steps:
      - uses: actions/checkout@v7.0.1
        with:
          fetch-depth: 2
          persist-credentials: false
      - uses: dblock/already-tested@<version>
        id: claims
        with:
          check: 'test-*'

  test:
    name: ${{ matrix.entry.name }}
    needs: verify
    if: >-
      ${{ !cancelled() &&
          (needs.verify.result != 'success' ||
           needs.verify.outputs.skip != 'true') }}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        entry:
          - { name: Unit Tests, command: ./script/unit }
          - { name: Integration Tests, command: ./script/integration }
    steps:
      - uses: actions/checkout@v7.0.1
      - name: Run Tests
        run: ${{ matrix.entry.command }}

  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
      - run: ./script/lint
```

After running both test commands on the matching Linux environment and committing the source, generate:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check 'test-*' \
  --command './script/unit && ./script/integration' \
  --environment '<actual Linux, CPU, and toolchain versions>'
```

Paste the complete output into the PR description. Both test jobs can skip; lint still runs. If only unit tests passed, use individual named gates as described in [Skip Individual Steps](github-actions.md), or omit the group claim and let CI run. A `test-unit` claim cannot skip this group.

## File Selection

Use identical Git pathspecs in the workflow and claim command. This step-gate fragment includes source, tests, and a dependency lock file:

```yaml
- uses: dblock/already-tested@v0.3.0
  id: tested
  with:
    check: test-linux
    files: |
      src/**
      tests/**
      package-lock.json
```

After running the complete check, commit and generate:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --files 'src/**' \
  --files 'tests/**' \
  --files 'package-lock.json' \
  --command './script/ci' \
  --environment '<actual OS, CPU, and toolchain versions>'
```

Replace the lock file with your project's actual dependency input and include configuration, generated inputs, and other files that affect the result. Add the normal `skip != 'true'` conditions from the [step guide](github-actions.md#complete-workflow). File filters work with named, glob, and blanket scopes; a blanket claim must use the same file selection as its gate.

Selection rules are Git pathspecs, not check globs. Patterns are trimmed using ECMAScript whitespace, duplicate patterns are removed, and the result is sorted by UTF-16 code units. Order and duplicate entries do not matter, but equivalent-looking policy strings remain distinct hash inputs. Attribute-based pathspecs and selections matching no tracked files are errors.

The executing workflow is always included, even if the filters exclude it. Selected symlinks, submodules, and invalid UTF-8 paths are rejected. Committed changes outside the selection need not invalidate a filtered claim, but the CLI still requires a clean tracked working tree before creating it. Never exclude relevant inputs merely to keep a claim valid.
