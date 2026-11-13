<!-- already-tested:instructions v{{VERSION}} -->
# Local CI Claims

These consumer instructions ship with already-tested v{{VERSION}}. Read the project's own agent instructions, CI workflows, check mappings, and environment policy first. Project-specific commands and stricter policies take precedence; this document does not grant permission to skip CI.

## Run Complete Checks

Identify each workflow gate, check scope, file selection, and required environment. Run the complete commands for every check you intend to claim. Match the actual OS, CPU, compiler/runtime, dependencies, and SDK unless the project explicitly permits a specific difference for that check. Evaluate eligibility per check; uncertainty about one environment must not suppress another eligible claim.

Never infer success from a content match or invent a permitted environment difference. Never claim an untested platform, focused tests as a complete job, or work whose external inputs cannot be reproduced. A glob or blanket claim requires every covered check and environment; separate partial claims do not combine into blanket coverage.

Measure each complete command or covered sequence's wall-clock time during its final successful run, rounded to the nearest whole second. Use the actual elapsed time, not session time or unrelated work. Do not guess or repeat a successful check solely to time it. If one aggregate command covers multiple checks, reuse its measured total and name that aggregate command rather than inventing individual times.

## Generate and Verify

Commit all intended inputs and leave tracked files and the index clean before generating claims. Ignored and untracked files are not included. Generate one claim per eligible workflow/check, using the project's actual values:

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --summary 'test-linux - passed (<seconds>s)' \
  --command '<complete command actually passed>' \
  --environment '<actual OS, CPU, toolchain, and SDK versions>'
```

Replace placeholders with actual values and measured seconds. Match workflow file selection with repeated `--files` arguments; omit filters when the gate covers all tracked files. Omit `--check` only after running the entire gate's scope, including all matrix environments.

Save the output in a temporary file outside tracked inputs, then verify it with the gate's workflow, scope, and selection:

```sh
already-tested verify \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --body-file '<temporary claim file>' \
  --json
```

Exit 0 means matching committed HEAD; exit 1 means no match; exit 2 means invalid input or verification failure. Never publish a failed verification as a match. Verification does not run tests, compare declared environments, or cover dirty files. CI checks the queued source, normally the PR merge, and actual executing workflow; target-branch changes may require retesting and a fresh claim.

## Publish Eligible Claims

Include verified claim disclosures for every eligible completed check in the PR description. A list of passed commands alone is not a claim. Use existing GitHub tools to read the latest description, preserve human-written text and unrelated claims, and replace only the claim being updated.

Paste complete generated disclosures adjacent without outer bullets or blank separators. Preserve the hidden canonical comments. Customize timing labels with `--summary`, not by editing canonical claim bytes. Always disclose what actually ran, never substitute CI's environment for the local one.

After tracked source or workflow changes, rerun affected complete checks, commit, and regenerate and verify the claims. Remove stale disclosures rather than suggesting untested content passed. Do not put claims in commits or tracked claim files.

## Trust and Limits

Claims are editable trusted statements, not cryptographic proof that tests ran. Command, environment, and elapsed time are descriptive metadata, not verified execution evidence. Missing, stale, malformed, or unverifiable claims mean normal CI. Keep default-branch CI and independent checks such as uploaded coverage; never disable required verification or broaden project policy to force a skip.
