# CLI Installation and Reference

[Documentation](README.md) | [Agent Instructions](agents.md)

The CLI generates claims after you have run tests; it never runs the command you supply. Ordinary use requires Git 2.29+, not Cargo, Rust, or Node/npm.

## Installation

Use the same version for the CLI and workflow Action. When changing the installation version, update both the download URL and installer argument.

### macOS and Linux

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.1.1/install.sh | sh -s -- --version v0.1.1
export PATH="$HOME/.local/bin:$PATH"
already-tested --version
```

### Windows

Run in PowerShell:

```powershell
$installer = "$env:TEMP\already-tested-install.ps1"
Invoke-WebRequest https://github.com/dblock/already-tested/releases/download/v0.1.1/install.ps1 -OutFile $installer
& $installer -Version v0.1.1
$env:PATH = "$HOME\.local\bin;$env:PATH"
already-tested --version
```

The installer selects Linux, macOS, or Windows on x86_64/ARM64 and verifies the archive checksum and executable version before replacing an install. PATH changes above affect the current shell; add the directory to your shell profile for future sessions. Archive checksums detect corruption, not an untrusted release publisher.

### Custom Installation Directory

```sh
curl -fsSL https://github.com/dblock/already-tested/releases/download/v0.1.1/install.sh |
  sh -s -- --version v0.1.1 --bin-dir "$HOME/tools/already-tested"
export PATH="$HOME/tools/already-tested:$PATH"
```

On Windows, pass `-BinDir "$HOME\tools\already-tested"` to the downloaded installer. The default directory is `~/.local/bin`. POSIX installers accept `--print-target`; Windows accepts `-PrintTarget` to inspect native target selection.

For mirrors or archive tests, `ALREADY_TESTED_DOWNLOAD_BASE_URL` accepts an HTTPS directory or `file://` URL containing the archive and checksum. Other schemes are rejected; an empty override uses the default release URL. POSIX also accepts `ALREADY_TESTED_VERSION` and `ALREADY_TESTED_BIN_DIR`. See [binary distribution](../DESIGN.md#binary-distribution) for platform targets.

## Create a Claim

Run your actual complete checks, commit the tested files, then generate a claim:

```sh
./script/ci
git add src tests .github/workflows/test.yml
git commit -m "Update tested source"
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --command './script/ci' \
  --environment '<actual OS, CPU, and toolchain versions>'
```

Adapt the paths to include every intended input and the command/environment to your actual run. The tracked working tree must be clean. The claim hashes committed HEAD source and workflow; untracked and ignored files are excluded.

Copy the entire expandable output into the PR description, not into a comment or a source file. Display metadata is explanatory only: it is not hashed and cannot substitute for actually running the checks. The hidden canonical comment must remain unmodified. See the [claim protocol](../DESIGN.md#cli-and-claim-format) for its exact representation.

## Options

```text
already-tested claim --workflow <path> [--check <name-or-glob>] [options]
```

| Option | Meaning |
| --- | --- |
| `--workflow <path>` | Required tracked `.yml`/`.yaml` file directly under `.github/workflows/`; paths are repository-relative |
| `--check <name-or-glob>` | Named check or glob; defaults to `*` when omitted |
| `--files <pattern>` | Repeat to select tracked files; omitted means all tracked files |
| `--summary <text>` | Expandable label; default `<check> - passed`, or `All workflow checks - passed` for `*` |
| `--command <text>` | Command already validated; recorded, never executed |
| `--environment <text>` | Environment actually validated; recorded, never inferred |
| `--raw` | Canonical comment only; incompatible with summary, command, and environment metadata |
| `--help` | Usage; no repository needed |
| `--version` | Package version; must be used alone; no repository needed |

Quote globs to prevent shell expansion. Explicit CLI `--check ''` is invalid; only omission defaults to `*`. The Action also defaults an empty input to `*`.

Metadata must be nonempty, single-line, and NUL-free. Summary HTML is escaped and inline code safely handles backticks. Success exits zero; invalid options or repository state exit one on stderr without emitting a claim.

## Custom Summary and Raw Output

```sh
already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --summary 'Linux tests - passed' \
  --command './script/ci' \
  --environment '<actual Linux, CPU, and toolchain versions>'

already-tested claim \
  --workflow .github/workflows/test.yml \
  --check test-linux \
  --raw
```

The first command includes a custom disclosure label; the second emits only the hidden canonical comment. Prefer the normal disclosure so reviewers can see what the agent says it ran.
