# Agent Instructions

[Documentation](README.md) | [CLI Reference](cli.md)

The bundled guide tells agents how to inspect CI workflows, run complete checks, and publish honest claims. Keep only project-specific policy that CI does not describe in your own `AGENTS.md`.

## Release-Matched Instructions

`already-tested agent-instructions show` prints standalone consumer instructions bundled with the installed CLI version, offline and without a Git repository. The output identifies its version and covers honest claims, measured timing labels, verification, and publishing. It is not this project's contributor `AGENTS.md`.

Every native archive also contains the same `agent-instructions.md`. Each release publishes that file and `agent-instructions.md.sha256` as separate assets for users without the CLI. Select an explicit release tag rather than a moving latest URL. These checksums detect corruption, not an untrusted release publisher.

Bundled guidance does not broaden project policy or establish environment equivalence. Installing or upgrading the CLI does not modify project instructions; review instruction changes alongside the matching CLI and CI-verifier upgrade.

### Adopt in a Project

After installing a release with this command, run this once from anywhere inside the project's Git working tree:

```sh
already-tested agent-instructions update
```

The command uses the installed binary's bundled instructions and chooses the entry point at the Git working tree root:

| Existing Instructions | Managed Guide | Reference Updated |
| --- | --- | --- |
| `.github/copilot-instructions.md` | `.github/instructions/already-tested.instructions.md` | `.github/copilot-instructions.md` |
| Otherwise | `.already-tested/agent-instructions.md` | `AGENTS.md`, created if absent |

An existing Copilot entry point takes precedence even when `AGENTS.md` also exists; `AGENTS.md` is then left untouched. The generated Copilot guide includes `applyTo: "**"` front matter. The command adds or refreshes only its marked reference in the selected entry point, preserving handwritten policy. It does not fetch files, run tests, stage, or commit changes. Track both files so agents can read them offline.

Override the guide destination with a repository-relative file path, even when running from a subdirectory:

```sh
already-tested agent-instructions update --path docs/local-ci.md
```

The override does not change which entry point receives the reference. Keep project-specific commands and environment policy outside the managed guide. Do not copy this repository's contributor instructions into another project.

### Upgrade Instructions

Upgrade to the chosen CLI release, align the Action and any separately pinned CI verifier, then explicitly run `already-tested agent-instructions update` again, repeating `--path` if you chose a custom destination. Review and commit both file diffs alongside those upgrades. Installing the CLI alone never rewrites either file. Changing destinations leaves old copies and references in other entry points for you to review and remove.

An unchanged managed guide from an older release can be replaced offline. A body checksum detects local edits; it is not an authenticity guarantee. Updates refuse customized or unrecognized guides unless you deliberately pass `already-tested agent-instructions update --force`. Preserve local policy in the selected entry point before overriding. Symbolic links and targets that are not regular files are rejected even with `--force`, which never overrides handwritten policy or malformed reference markers. Each file is replaced atomically.

## Project Setup

Copy the [two-line agent setup](../README.md#usage) into your project's `AGENTS.md` or `.github/copilot-instructions.md`. The agent installs the CLI and reads its release-matched instructions; CI workflows remain the source of truth for commands, check names, and environments.

The same setup works for [individual steps](github-actions.md) and [blanket gates](skip-jobs.md); separate claim-generation checklists are unnecessary.

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

Use distinct check IDs such as `test-macos-26.6-arm64` and `test-macos-26.6-x86_64` when those environments must remain separate. Apply exceptions individually to every job covered by a glob or blanket claim. Permission for one macOS job does not justify claiming other matrix environments or all workflow checks.
