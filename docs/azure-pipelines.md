# Azure Pipelines

[Documentation](README.md) | [CLI Reference](cli.md)

Use trusted local-test claims in Azure Repos PR descriptions to skip matching steps in an Azure Pipelines YAML build. Claims are statements, not evidence that tests ran; anyone who can edit the description can forge them. Keep default-branch CI and independent integration checks.

This integration is unreleased; the published v0.2.0 binary does not include it. For evaluation, build this revision with `cargo build --locked --release --bin already-tested` and provision the resulting native executable on the matching agent platform. Consumers of a release containing this feature can use the normal [native installers](cli.md#installation); agents do not need Rust or Node/npm at runtime.

## Pipeline Setup

Start with an Azure DevOps Services repository and a single-file YAML pipeline in that same repository. Configure a [build validation branch policy](https://learn.microsoft.com/en-us/azure/devops/repos/git/branch-policies#build-validation) for PRs; Azure Repos does not use a YAML `pr` trigger.

Copy the reviewed [PowerShell gate](../scripts/azure-pipelines.ps1) into your repository as `.ci/already-tested.ps1`. Install a supporting native `already-tested` binary on PATH before the gate, or pass its path through `-Binary`. Installation must not prevent normal CI if it fails; the gate treats a missing binary as `skip=false`.

In the actual executing `azure-pipelines.yml`:

```yaml
trigger:
  branches:
    include:
      - main

pool:
  vmImage: ubuntu-latest

steps:
  - checkout: self
    fetchDepth: 0
    persistCredentials: false
  - pwsh: ./.ci/already-tested.ps1 -Workflow azure-pipelines.yml -Check test-linux
    name: tested
    displayName: Verify Local CI Claim
    env:
      SYSTEM_ACCESSTOKEN: $(System.AccessToken)
  - script: ./script/ci
    displayName: Run Tests
    condition: and(succeeded(), ne(variables['tested.skip'], 'true'))
```

Replace `main`, `./script/ci`, and `test-linux` with your branch, complete check, and environment-specific identity. Apply the condition to each replaceable setup, dependency-install, and test step while preserving existing conditions. The named step's output defaults to false before API or binary work. The pipeline still runs; this example skips steps, not the enclosing job or build validation record.

The build service identity needs repository read access. Explicitly map [`System.AccessToken`](https://learn.microsoft.com/en-us/azure/devops/pipelines/build/variables#systemaccesstoken) into the gate; no PAT or write permission is needed. The wrapper only makes its API request for Azure Repos PR builds. It never edits the PR or logs the description or token.

Full checkout history makes both queued source and merge commits available without hidden fetches. The native adapter uses `System.PullRequest.SourceCommitId` for source and `Build.SourceVersion` for the executing YAML, not checkout HEAD or the latest PR source from the API. This preserves a decision tied to the queued build even if the PR advances while it runs. The pipeline path must identify the actual executing YAML; it is not discovered automatically.

## Agent Claims

Add instructions to `AGENTS.md` naming the complete check and required environment. After actually running that check in the matching environment, commit every intended input and generate:

```sh
already-tested claim \
  --ci azure-pipelines \
  --workflow azure-pipelines.yml \
  --check test-linux \
  --command './script/ci' \
  --environment '<actual Linux, CPU, and toolchain versions>'
```

Paste the entire output into the Azure Repos PR description, preserving its canonical hidden comment. If Azure's editor changes the expandable disclosure, retain the exact canonical comment and a visible explanation of the command/environment actually tested. Neither verification nor environment metadata proves that tests ran. Retest, commit, and replace the claim after tracked changes.

The gate defaults an omitted check to `*`; only use a blanket or glob claim after running all covered checks and environments. For file filtering, pass an array through the script's `-Files` parameter and repeat the same patterns with CLI `--files`. The executing YAML is always hashed regardless of filters. Local claim generation and `verify` should use `--ci azure-pipelines`; automatic detection falls back to GitHub Actions outside CI.

## Limits

Only Azure DevOps Services, Azure Repos Git, and single-repository, single-file YAML pipelines are covered initially. GitHub-hosted PRs built by Azure, classic pipelines, Azure DevOps Server, multiple checkouts, and template-based pipelines are not supported by this guide. Templates, variable groups, secrets, service configuration, and external dependencies are not resolved or hashed; do not use claims to replace checks whose required inputs are outside this model.

Description edits do not provide a GitHub-style `edited` trigger in this integration. Queue PR validation again after adding a claim if needed; an already-running gate does not reconsider its decision. Non-PR builds, missing/stale/malformed claims, unavailable commits, token/API failures, and missing or incompatible binaries run normal CI. The wrapper requires PowerShell, available through `pwsh` on Microsoft-hosted agents.

The native command can also be called directly from another wrapper:

```sh
already-tested ci \
  --ci azure-pipelines \
  --workflow azure-pipelines.yml \
  --check test-linux \
  --body-file /path/to/pr-description.md
```

`--ci` can be omitted in Azure Pipelines when `TF_BUILD=True` identifies the system unambiguously. The PowerShell wrapper selects Azure explicitly because its integration is already known. The native command reads Azure's build environment and emits `##vso[task.setvariable variable=skip;isOutput=true]true` or `false`. Operational verification errors return success with a false decision; invalid CLI syntax, conflicting detection signals, or output-write failures are errors. A custom wrapper must default to normal CI if the binary cannot start or deliver a decision.
