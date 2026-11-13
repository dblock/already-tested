#![cfg(unix)]

use rust_crate_release::{
    changelog::{Changelog, KeepAChangelog},
    engine::CommandValidator,
    package::{CargoPackage, PackageProject},
    testing::{CopySelection, GitProject, ReleaseFixture},
};
use semver::Version;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Arc,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn checked(output: Output) -> Result<String> {
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg_attr(
    dylint_lib = "general",
    allow(
        abs_home_path,
        reason = "The upstream fixture copies reviewed source files from this checkout."
    )
)]
fn project() -> Result<GitProject> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing root")?;
    let files = checked(
        Command::new("git")
            .args(["ls-files", "-c", "-o", "--exclude-standard", "-z"])
            .current_dir(root)
            .output()?,
    )?;
    let files = files
        .split('\0')
        .filter(|file| !file.is_empty())
        .map(PathBuf::from)
        .filter(|path| root.join(path).is_file())
        .collect();
    Ok(GitProject::from_project(
        root,
        &CopySelection::Paths(files),
    )?)
}

fn git(project: &GitProject, args: &[&str]) -> Result<String> {
    checked(
        Command::new("git")
            .args(args)
            .current_dir(project.root())
            .output()?,
    )
}

fn next_version(version: &Version) -> Version {
    Version::new(version.major, version.minor, version.patch + 1)
}

fn prepare_fixture_notes(project: &GitProject) -> Result<()> {
    let changelog = project.root().join("CHANGELOG.md");
    let original = fs::read_to_string(&changelog)?;
    let updated = original.replacen(
        "## [Unreleased]\n",
        "## [Unreleased]\n\n- Exercise the release lifecycle.\n",
        1,
    );
    if updated != original {
        fs::write(changelog, updated)?;
        git(project, &["add", "CHANGELOG.md"])?;
        git(project, &["commit", "-m", "Prepare fixture release notes"])?;
        git(project, &["push", "origin", "master"])?;
    }
    Ok(())
}

fn native_assets(project: &GitProject, version: &str) -> Result<PathBuf> {
    let mock = project.temporary_directory().join("mock");
    fs::create_dir_all(mock.join("assets"))?;
    fs::create_dir(mock.join("bin"))?;
    fs::create_dir(mock.join("native"))?;
    fs::write(mock.join("bin/gh"), include_str!("fixtures/gh"))?;
    fs::set_permissions(mock.join("bin/gh"), fs::Permissions::from_mode(0o755))?;
    for target in [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
    ] {
        let archive = format!("already-tested-{target}.tar.gz");
        let bytes = format!("validated artifact {target}\n");
        fs::write(mock.join("native").join(&archive), &bytes)?;
        let digest: String = Sha256::digest(bytes.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        fs::write(
            mock.join("native").join(format!("{archive}.sha256")),
            format!("{digest}  {archive}\n"),
        )?;
    }
    fs::write(
        mock.join("tag-sha"),
        git(project, &["rev-parse", &format!("v{version}")])?,
    )?;
    fs::write(mock.join("released"), "")?;
    Ok(mock)
}

fn upload(project: &GitProject, mock: &Path, source: &str, version: &str) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("publish-native-assets")
        .current_dir(project.root())
        .env(
            "PATH",
            format!("{}:{}", mock.join("bin").display(), std::env::var("PATH")?),
        )
        .env("MOCK", mock)
        .env("GH_REPO", "dblock/already-tested")
        .env("GITHUB_SHA", source.trim())
        .env("NATIVE_ARCHIVE_DIRECTORY", mock.join("native"))
        .env("RELEASED_VERSION", version)
        .env("GITHUB_STEP_SUMMARY", mock.join("summary"))
        .output()?)
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "The upstream fixture isolates Git, registry, hosting, and child commands."
    )
)]
fn release_fixture_updates_metadata_and_native_uploads_recover_without_overwriting() -> Result<()> {
    let project = project()?;
    let released = CargoPackage::new(project.root())?.metadata()?.version;
    let next = next_version(&released);
    let version = released.to_string();
    prepare_fixture_notes(&project)?;
    let mut fixture = ReleaseFixture::new(project)?;
    fixture.configure_workflow(|workflow| {
        workflow.components.validator = Arc::new(CommandValidator {
            commands: vec![
                vec!["cargo".into(), "check".into(), "--locked".into()],
                vec!["cargo".into(), "xtask".into(), "docs".into()],
            ],
        });
        Ok(())
    });
    let source = git(&fixture.project, &["rev-parse", "HEAD"])?;
    assert_eq!(fixture.release(None)?["verification"]["status"], "verified");
    assert_eq!(fixture.registry.publication_count(), 1);
    assert_eq!(fixture.host.publication_count(), 1);
    for (file, expected) in [
        ("Cargo.toml", format!("version = \"{next}\"")),
        (
            "Cargo.lock",
            format!("\nname = \"already-tested\"\nversion = \"{next}\"\n"),
        ),
        ("action.yml", format!("default: v{next}")),
        ("scripts/action.sh", format!("INPUT_VERSION:-v{next}")),
        ("CHANGELOG.md", "## [Unreleased]".into()),
        (".github/ci-version", format!("v{released}\n")),
        ("README.md", format!("already-tested@v{released}")),
        (
            "docs/cli.md",
            format!("releases/download/v{released}/install.sh"),
        ),
        (
            "docs/cli.md",
            format!("releases/download/v{released}/install.ps1"),
        ),
        ("docs/cli.md", format!("--version v{released}")),
        ("docs/cli.md", format!("-Version v{released}")),
        (
            "docs/github-actions.md",
            format!("already-tested@v{released}"),
        ),
        (
            "docs/configuration.md",
            format!("already-tested@v{released}"),
        ),
    ] {
        let contents = String::from_utf8(fixture.remote_file("master", Path::new(file))?)?;
        assert!(contents.contains(&expected), "{file}: {contents}");
    }
    let released_changelog = String::from_utf8(
        fixture.remote_file(&format!("v{released}"), Path::new("CHANGELOG.md"))?,
    )?;
    assert!(released_changelog.contains(&format!("## [{released}] - ")));
    assert!(!released_changelog.contains("## [Unreleased]"));
    let development_changelog =
        String::from_utf8(fixture.remote_file("master", Path::new("CHANGELOG.md"))?)?;
    assert!(development_changelog.contains(&format!(
        "[Unreleased]: https://github.com/dblock/already-tested/compare/v{released}...master"
    )));
    let project = &fixture.project;
    checked(
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("release-metadata")
            .current_dir(project.root())
            .env("RCR_RELEASE_VERSION", &version)
            .env("RCR_NEXT_VERSION", next.to_string())
            .output()?,
    )?;
    git(project, &["diff", "--exit-code", "HEAD"])?;
    let mock = native_assets(project, &version)?;
    fs::write(mock.join("fail-upload"), "")?;
    assert!(!upload(project, &mock, &source, &version)?.status.success());
    assert_eq!(fs::read_dir(mock.join("assets"))?.count(), 1);
    fs::write(
        project.root().join("scripts/install.sh"),
        "development installer\n",
    )?;
    fs::write(
        project.root().join("assets/agent-instructions.md"),
        "development instructions\n",
    )?;
    checked(upload(project, &mock, &source, &version)?)?;
    checked(upload(project, &mock, &source, &version)?)?;
    assert_eq!(fs::read_dir(mock.join("assets"))?.count(), 16);
    let instructions = String::from_utf8(fixture.remote_file(
        &format!("v{released}"),
        Path::new("assets/agent-instructions.md"),
    )?)?
    .replace("{{VERSION}}", &version);
    assert_eq!(
        fs::read_to_string(mock.join("assets/agent-instructions.md"))?,
        instructions
    );
    let digest: String = Sha256::digest(instructions.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        fs::read_to_string(mock.join("assets/agent-instructions.md.sha256"))?,
        format!("{digest}  agent-instructions.md\n")
    );
    assert_eq!(
        fs::read(mock.join("assets/install.sh"))?,
        fixture.remote_file(&format!("v{released}"), Path::new("scripts/install.sh"))?
    );
    fs::write(mock.join("assets/install.sh"), "changed remotely\n")?;
    assert!(!upload(project, &mock, &source, &version)?.status.success());
    assert_eq!(
        fs::read_to_string(mock.join("assets/install.sh"))?,
        "changed remotely\n"
    );
    fs::write(
        mock.join("assets/install.sh"),
        fixture.remote_file(&format!("v{released}"), Path::new("scripts/install.sh"))?,
    )?;
    let archive = mock.join("native/already-tested-x86_64-apple-darwin.tar.gz");
    let bytes = fs::read(&archive)?;
    fs::write(&archive, "corrupt\n")?;
    assert!(!upload(project, &mock, &source, &version)?.status.success());
    fs::write(archive, bytes)?;
    fs::remove_file(mock.join("released"))?;
    assert!(!upload(project, &mock, &source, &version)?.status.success());
    fs::write(mock.join("released"), "")?;
    assert!(!upload(project, &mock, "HEAD", &version)?.status.success());
    Ok(())
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "The upstream disposable Git fixture reproduces release-prepared source."
    )
)]
fn fixture_notes_accept_an_already_dated_release() -> Result<()> {
    let project = project()?;
    prepare_fixture_notes(&project)?;
    let version = CargoPackage::new(project.root())?.metadata()?.version;
    let changelog = project.root().join("CHANGELOG.md");
    KeepAChangelog::default().release_file(&changelog, &version, "2026/01/01", None)?;
    git(&project, &["add", "CHANGELOG.md"])?;
    git(
        &project,
        &["commit", "--allow-empty", "-m", "Prepare release snapshot"],
    )?;
    let prepared = fs::read(&changelog)?;
    let head = git(&project, &["rev-parse", "HEAD"])?;
    prepare_fixture_notes(&project)?;
    assert_eq!(fs::read(changelog)?, prepared);
    assert_eq!(git(&project, &["rev-parse", "HEAD"])?, head);
    Ok(())
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "All writes and child processes use the upstream disposable Git fixture."
    )
)]
fn invalid_metadata_hook_inputs_leave_action_defaults_unchanged() -> Result<()> {
    let project = project()?;
    let package = CargoPackage::new(project.root())?;
    let released = package.metadata()?.version;
    let next = next_version(&released);
    let original = fs::read(project.root().join("action.yml"))?;
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("release-metadata")
            .current_dir(project.root())
            .env("RCR_RELEASE_VERSION", released.to_string())
            .env("RCR_NEXT_VERSION", next.to_string())
            .output()
    };
    assert!(!run()?.status.success());
    package.set_version(&next)?;
    fs::remove_file(project.root().join("README.md"))?;
    assert!(!run()?.status.success());
    assert_eq!(fs::read(project.root().join("action.yml"))?, original);
    Ok(())
}
