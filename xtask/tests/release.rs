#![cfg(unix)]

use rust_crate_release::{
    engine::CommandValidator,
    testing::{CopySelection, GitProject, ReleaseFixture},
};
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

fn native_assets(project: &GitProject) -> Result<PathBuf> {
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
        git(project, &["rev-parse", "v0.1.0"])?,
    )?;
    fs::write(mock.join("released"), "")?;
    Ok(mock)
}

fn upload(project: &GitProject, mock: &Path, source: &str) -> Result<Output> {
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
        .env("RELEASED_VERSION", "0.1.0")
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
    let mut fixture = ReleaseFixture::new(project()?)?;
    fixture.configure_workflow(|workflow| {
        workflow.components.validator = Arc::new(CommandValidator {
            commands: vec![vec!["cargo".into(), "check".into(), "--locked".into()]],
        });
        Ok(())
    });
    let source = git(&fixture.project, &["rev-parse", "HEAD"])?;
    assert_eq!(fixture.release(None)?["verification"]["status"], "verified");
    assert_eq!(fixture.registry.publication_count(), 1);
    assert_eq!(fixture.host.publication_count(), 1);
    for (file, expected) in [
        ("Cargo.toml", "version = \"0.2.0\""),
        ("action.yml", "default: v0.2.0"),
        ("scripts/action.sh", "INPUT_VERSION:-v0.2.0"),
        ("rcr.toml", "next-version = \"0.3.0\""),
        ("CHANGELOG.md", "## 0.2.0 (Next)"),
        (".github/ci-version", "v0.1.0\n"),
        ("README.md", "already-tested@v0.1.0"),
        ("DESIGN.md", "currently `v0.2.0`"),
    ] {
        let contents = String::from_utf8(fixture.remote_file("master", Path::new(file))?)?;
        assert!(contents.contains(expected), "{file}: {contents}");
    }
    let project = &fixture.project;
    let mock = native_assets(project)?;
    fs::write(mock.join("fail-upload"), "")?;
    assert!(!upload(project, &mock, &source)?.status.success());
    assert_eq!(fs::read_dir(mock.join("assets"))?.count(), 1);
    fs::write(
        project.root().join("scripts/install.sh"),
        "development installer\n",
    )?;
    checked(upload(project, &mock, &source)?)?;
    checked(upload(project, &mock, &source)?)?;
    assert_eq!(fs::read_dir(mock.join("assets"))?.count(), 14);
    assert_eq!(
        fs::read(mock.join("assets/install.sh"))?,
        fixture.remote_file("v0.1.0", Path::new("scripts/install.sh"))?
    );
    fs::write(mock.join("assets/install.sh"), "changed remotely\n")?;
    assert!(!upload(project, &mock, &source)?.status.success());
    assert_eq!(
        fs::read_to_string(mock.join("assets/install.sh"))?,
        "changed remotely\n"
    );
    fs::write(
        mock.join("assets/install.sh"),
        fixture.remote_file("v0.1.0", Path::new("scripts/install.sh"))?,
    )?;
    let archive = mock.join("native/already-tested-x86_64-apple-darwin.tar.gz");
    let bytes = fs::read(&archive)?;
    fs::write(&archive, "corrupt\n")?;
    assert!(!upload(project, &mock, &source)?.status.success());
    fs::write(archive, bytes)?;
    fs::remove_file(mock.join("released"))?;
    assert!(!upload(project, &mock, &source)?.status.success());
    fs::write(mock.join("released"), "")?;
    assert!(!upload(project, &mock, "HEAD")?.status.success());
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
    let original = fs::read(project.root().join("action.yml"))?;
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("release-metadata")
            .current_dir(project.root())
            .env("RCR_RELEASE_VERSION", "0.1.0")
            .env("RCR_NEXT_VERSION", "0.2.0")
            .output()
    };
    assert!(!run()?.status.success());
    fs::write(
        project.root().join("Cargo.toml"),
        "[package]\nversion = \"0.2.0\"\n",
    )?;
    fs::write(project.root().join("rcr.toml"), "missing next version\n")?;
    assert!(!run()?.status.success());
    assert_eq!(fs::read(project.root().join("action.yml"))?, original);
    Ok(())
}
