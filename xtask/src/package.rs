use crate::{Result, run};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

pub const TARGETS: [&str; 6] = [
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];

fn file_url(path: &Path) -> Result<String> {
    let path = path.canonicalize()?.to_string_lossy().replace('\\', "/");
    let path = path.trim_start_matches("//?/");
    let path = path
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23");
    Ok(if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    })
}

pub fn build(root: &Path, target: Option<&str>) -> Result<()> {
    let metadata: serde_json::Value = serde_json::from_str(&run(
        "cargo",
        &["metadata", "--locked", "--no-deps", "--format-version", "1"],
        root,
    )?)?;
    let version = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages.iter().find_map(|package| {
                (package["name"] == "already-tested")
                    .then(|| package["version"].as_str())
                    .flatten()
            })
        })
        .ok_or("Missing package version")?;
    let selected = if cfg!(windows) {
        run(
            "pwsh",
            &["-NoProfile", "-File", "scripts/install.ps1", "-PrintTarget"],
            root,
        )?
    } else {
        run("sh", &["scripts/install.sh", "--print-target"], root)?
    };
    let selected = selected.trim();
    let target = target.unwrap_or(selected);
    if !TARGETS.contains(&target) {
        return Err(format!(
            "Unsupported distribution target: {target}; use --target with a supported target"
        )
        .into());
    }
    if target != selected {
        return Err("Run package validation on the target OS/architecture; use cargo build for cross-compilation".into());
    }
    run(
        "cargo",
        &[
            "build",
            "--locked",
            "--release",
            "--bin",
            "already-tested",
            "--target",
            target,
        ],
        root,
    )?;
    let binary = if target.ends_with("windows-msvc") {
        "already-tested.exe"
    } else {
        "already-tested"
    };
    let source = Path::new(
        metadata["target_directory"]
            .as_str()
            .ok_or("Missing Cargo target directory")?,
    )
    .join(target)
    .join("release")
    .join(binary);
    let temporary = tempdir()?;
    fs::copy(&source, temporary.path().join(binary))?;
    fs::copy(root.join("LICENSE"), temporary.path().join("LICENSE"))?;
    let directory = root.join("pkg");
    fs::create_dir_all(&directory)?;
    let archive_name = format!("already-tested-{target}.tar.gz");
    let archive = directory.join(&archive_name);
    run(
        "tar",
        &[
            "-czf",
            archive.to_str().ok_or("Invalid archive path")?,
            "-C",
            temporary.path().to_str().ok_or("Invalid temporary path")?,
            binary,
            "LICENSE",
        ],
        root,
    )?;
    let listing = run(
        "tar",
        &["-tzf", archive.to_str().ok_or("Invalid archive path")?],
        root,
    )?;
    let mut entries: Vec<_> = listing.lines().collect();
    entries.sort_unstable();
    if entries != ["LICENSE", binary] {
        return Err("Archive must contain exactly the executable and LICENSE".into());
    }
    let digest: String = Sha256::digest(fs::read(&archive)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    fs::write(
        directory.join(format!("{archive_name}.sha256")),
        format!("{digest}  {archive_name}\n"),
    )?;
    let install = tempdir()?;
    let mut command = if cfg!(windows) {
        let mut command = Command::new("pwsh");
        command.args([
            "-NoProfile",
            "-File",
            "scripts/install.ps1",
            "-Version",
            version,
            "-BinDir",
        ]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["scripts/install.sh", "--version", version, "--bin-dir"]);
        command
    };
    let download_base = file_url(&directory)?;
    let result = command
        .arg(install.path())
        .current_dir(root)
        .env("ALREADY_TESTED_DOWNLOAD_BASE_URL", &download_base)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "Installer failed: {}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    let executable = install.path().join(binary);
    installed_consumer(&executable, version, root, &download_base)?;
    let previous = fs::read(&executable)?;
    let replaced = command.output()?;
    if !replaced.status.success() || fs::read(&executable)? != previous {
        return Err(format!(
            "Existing executable replacement failed: {}",
            String::from_utf8_lossy(&replaced.stderr)
        )
        .into());
    }
    fs::write(
        directory.join(format!("{archive_name}.sha256")),
        format!("{}  {archive_name}\n", "0".repeat(64)),
    )?;
    let failed = command.output()?;
    fs::write(
        directory.join(format!("{archive_name}.sha256")),
        format!("{digest}  {archive_name}\n"),
    )?;
    if failed.status.success() || fs::read(&executable)? != previous {
        return Err(
            "A checksum mismatch must fail without replacing the existing executable".into(),
        );
    }
    println!(
        "Built {} and verified real archive installation, CLI/Action round trip, and checksum failure.",
        archive.display()
    );
    Ok(())
}

fn invoke(binary: &Path, args: &[&str], cwd: &Path) -> Result<String> {
    run(binary.to_str().ok_or("Invalid executable path")?, args, cwd)
}

fn installed_consumer(
    binary: &Path,
    expected_version: &str,
    source: &Path,
    download_base: &str,
) -> Result<()> {
    let temporary = tempdir()?;
    let root = temporary.path();
    fs::create_dir_all(root.join(".github/workflows"))?;
    fs::write(
        root.join(".github/workflows/test.yml"),
        "name: Test\non: push\n",
    )?;
    fs::write(root.join("example.txt"), "already tested\n")?;
    run("git", &["init", "-q", "-b", "master"], root)?;
    run("git", &["config", "user.name", "Package Fixture"], root)?;
    run(
        "git",
        &["config", "user.email", "package@example.invalid"],
        root,
    )?;
    run("git", &["add", "."], root)?;
    run("git", &["commit", "-qm", "Tested fixture"], root)?;
    let version = invoke(binary, &["--version"], root)?;
    if version.trim() != expected_version {
        return Err("Unexpected installed version".into());
    }
    let claim = invoke(
        binary,
        &[
            "claim",
            "--workflow",
            ".github/workflows/test.yml",
            "--check",
            "unit",
        ],
        root,
    )?;
    if !claim.starts_with("<details>\n") || !claim.contains("<!-- already-tested:v1 ") {
        return Err("Installed CLI did not emit a complete claim".into());
    }
    let claim_file = root.join(".git/claim.md");
    for (body, expected_code, expected_reason) in [
        (Some(claim.as_str()), 0, "matched"),
        (Some("No claim."), 1, "missing_claim"),
        (None, 2, "error"),
    ] {
        if let Some(body) = body {
            fs::write(&claim_file, body)?;
        } else {
            fs::remove_file(&claim_file)?;
        }
        let result = Command::new(binary)
            .args([
                "verify",
                "--workflow",
                ".github/workflows/test.yml",
                "--check",
                "unit",
                "--json",
                "--body-file",
            ])
            .arg(&claim_file)
            .current_dir(root)
            .output()?;
        let report: serde_json::Value = serde_json::from_slice(&result.stdout)?;
        if result.status.code() != Some(expected_code)
            || report["reason"] != expected_reason
            || report["skip"] != (expected_code == 0)
            || !result.stderr.is_empty()
        {
            return Err(
                "Installed verify command returned an incorrect verdict or exit code".into(),
            );
        }
    }
    let blanket_claim = invoke(
        binary,
        &["claim", "--workflow", ".github/workflows/test.yml"],
        root,
    )?;
    let scoped_claim = invoke(
        binary,
        &[
            "claim",
            "--workflow",
            ".github/workflows/test.yml",
            "--check",
            "test-*",
        ],
        root,
    )?;
    let head = run("git", &["rev-parse", "HEAD"], root)?;
    let output = root.join("output");
    let summary = root.join("summary");
    let event = root.join("event.json");
    for wrapper in [false, true] {
        let mut action = if wrapper {
            let mut action = Command::new("sh");
            action.arg(
                source
                    .join("scripts/action.sh")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
            action
        } else {
            let mut action = Command::new(binary);
            action.arg("action");
            action
        };
        action
            .current_dir(root)
            .env("INPUT_VERSION", expected_version)
            .env("GITHUB_EVENT_NAME", "pull_request")
            .env("GITHUB_EVENT_PATH", &event)
            .env("GITHUB_WORKSPACE", root)
            .env(
                "GITHUB_ACTION_PATH",
                source.to_string_lossy().replace('\\', "/"),
            )
            .env("RUNNER_TEMP", root)
            .env("ALREADY_TESTED_DOWNLOAD_BASE_URL", download_base)
            .env_remove("ALREADY_TESTED_BINARY")
            .env(
                "GITHUB_WORKFLOW_REF",
                "dblock/already-tested/.github/workflows/test.yml@refs/heads/master",
            )
            .env("GITHUB_WORKFLOW_SHA", head.trim())
            .env("GITHUB_STEP_SUMMARY", &summary)
            .env("GITHUB_OUTPUT", &output);
        for (check, body, expected) in [
            (Some("unit"), claim.as_str(), "skip=true\n"),
            (Some("unit"), "Claim removed.", "skip=false\n"),
            (None, blanket_claim.as_str(), "skip=true\n"),
            (Some(""), blanket_claim.as_str(), "skip=true\n"),
            (Some("*"), blanket_claim.as_str(), "skip=true\n"),
            (None, claim.as_str(), "skip=false\n"),
            (Some("unit"), blanket_claim.as_str(), "skip=true\n"),
            (Some("test-linux"), scoped_claim.as_str(), "skip=true\n"),
            (Some("test-l*"), scoped_claim.as_str(), "skip=true\n"),
            (Some("lint-*"), scoped_claim.as_str(), "skip=false\n"),
            (None, scoped_claim.as_str(), "skip=false\n"),
        ] {
            if let Some(check) = check {
                action.env("INPUT_CHECK", check);
            } else {
                action.env_remove("INPUT_CHECK");
            }
            fs::write(
                &event,
                serde_json::to_vec(
                    &serde_json::json!({"pull_request":{"head":{"sha":head.trim()},"body":body}}),
                )?,
            )?;
            fs::write(&output, "")?;
            fs::write(&summary, "Earlier step.\n")?;
            let result = action.output()?;
            if !result.status.success() || fs::read_to_string(&output)? != expected {
                return Err(format!(
                    "Installed Action adapter/wrapper failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                )
                .into());
            }
            let report = fs::read_to_string(&summary)?;
            let expected = if expected == "skip=true\n" {
                "Matching local claim; gated work may be skipped."
            } else {
                "Run CI normally."
            };
            if !report.starts_with("Earlier step.\n\n### Already Tested\n")
                || !report.contains(expected)
            {
                return Err(
                    "Installed Action adapter/wrapper did not append its job summary".into(),
                );
            }
        }
        if wrapper {
            action.env(
                "ALREADY_TESTED_DOWNLOAD_BASE_URL",
                format!("{download_base}/missing"),
            );
            fs::write(&output, "")?;
            fs::write(&summary, "")?;
            if !action.output()?.status.success() || fs::read_to_string(&output)? != "skip=false\n"
            {
                return Err("Missing release must fall back to normal CI".into());
            }
            if !fs::read_to_string(&summary)?.contains("Could not install verifier.") {
                return Err("Installation failure must appear in the job summary".into());
            }
            action.env("GITHUB_STEP_SUMMARY", root);
            fs::write(&output, "")?;
            let failed_summary = action.output()?;
            if !failed_summary.status.success()
                || fs::read_to_string(&output)? != "skip=false\n"
                || !String::from_utf8_lossy(&failed_summary.stderr)
                    .contains("Could not write job summary")
            {
                return Err("Summary write failures must not change installation fallback".into());
            }
            action.env("GITHUB_STEP_SUMMARY", &summary);
            let previous = fs::read(&summary)?;
            action.env("GITHUB_OUTPUT", root);
            if action.output()?.status.success() {
                return Err("Action output write failures must not be masked".into());
            }
            if fs::read(&summary)? != previous {
                return Err("Failed Action output writes must not append a summary".into());
            }
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{env, path::PathBuf};

    #[test]
    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All file mutations are confined to a fresh per-test temporary directory."
        )
    )]
    fn platform_selection_rejects_unsupported_systems_and_covers_all_targets() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Missing root")?;
        let shim = tempdir()?;
        let cases = [
            ("Linux", "x86_64", "0", "", "", TARGETS[0]),
            ("Linux", "aarch64", "0", "", "", TARGETS[1]),
            ("Darwin", "x86_64", "0", "", "", TARGETS[2]),
            ("Darwin", "arm64", "1", "", "", TARGETS[3]),
            ("Darwin", "x86_64", "1", "", "", TARGETS[3]),
            ("MINGW64_NT-10.0", "x86_64", "0", "", "", TARGETS[4]),
            ("MINGW64_NT-10.0", "aarch64", "0", "", "", TARGETS[5]),
            ("MINGW64_NT-10.0", "x86_64", "0", "ARM64", "", TARGETS[5]),
            (
                "MINGW64_NT-10.0",
                "x86_64",
                "0",
                "AMD64",
                "Arm64",
                TARGETS[5],
            ),
            ("MINGW64_NT-10.0", "x86_64", "0", "", "X64", TARGETS[4]),
            ("MINGW64_NT-10.0", "x86_64", "0", "", "X86", ""),
            ("Linux", "unsupported", "0", "", "", ""),
            ("unsupported", "x86_64", "0", "", "", ""),
        ];
        for (os, arch, apple_arm, windows_native, powershell_arch, expected) in cases {
            fs::write(
                shim.path().join("pwsh"),
                format!(
                    "#!/bin/sh\n[ -n '{powershell_arch}' ] || exit 1\nprintf '%s\\r\\n' '{powershell_arch}'\n"
                ),
            )?;
            fs::set_permissions(shim.path().join("pwsh"), fs::Permissions::from_mode(0o755))?;
            fs::write(
                shim.path().join("sysctl"),
                format!("#!/bin/sh\nprintf '{apple_arm}\\n'\n"),
            )?;
            fs::set_permissions(
                shim.path().join("sysctl"),
                fs::Permissions::from_mode(0o755),
            )?;
            fs::write(
                shim.path().join("uname"),
                format!(
                    "#!/bin/sh\ncase \"$1\" in -s) printf '%s\\n' '{os}' ;; -m) printf '%s\\n' '{arch}' ;; esac\n"
                ),
            )?;
            fs::set_permissions(shim.path().join("uname"), fs::Permissions::from_mode(0o755))?;
            let path = env::join_paths(
                std::iter::once(PathBuf::from(shim.path()))
                    .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
            )?;
            let result = Command::new("sh")
                .args(["scripts/install.sh", "--print-target"])
                .env("PATH", &path)
                .env_remove("PROCESSOR_ARCHITECTURE")
                .env("PROCESSOR_ARCHITEW6432", windows_native)
                .current_dir(root)
                .output()?;
            if expected.is_empty() {
                assert!(!result.status.success());
                continue;
            }
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(String::from_utf8(result.stdout)?.trim(), expected);
        }
        Ok(())
    }
}
