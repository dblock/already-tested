use crate::{Result, checks, package::TARGETS, run};
use regex::{Captures, Regex};
use semver::Version;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::{env, fs, path::Path};
use tempfile::tempdir;

fn replace_version(text: &str, old: &str, new: &str) -> Result<String> {
    if text.matches(old).count() == 1 {
        Ok(text.replacen(old, new, 1))
    } else if text.matches(new).count() == 1 {
        Ok(text.to_owned())
    } else {
        Err(format!("Expected exactly one occurrence of {old:?} or {new:?}").into())
    }
}

pub fn update(root: &Path, released: &str, next: &str) -> Result<()> {
    let version = Version::parse(released)?;
    let next_version = Version::parse(next)?;
    if !version.pre.is_empty()
        || !version.build.is_empty()
        || !next_version.pre.is_empty()
        || !next_version.build.is_empty()
        || next_version <= version
    {
        return Err("Release and next versions must be stable and increasing".into());
    }
    if !fs::read_to_string(root.join("Cargo.toml"))?.contains(&format!("\nversion = \"{next}\"\n"))
    {
        return Err("rcr must update Cargo.toml before the metadata hook".into());
    }
    let published = Regex::new(
        r"(dblock/already-tested@v|already-tested/releases/download/v|--version v|-Version v)([0-9]+\.[0-9]+\.[0-9]+)",
    )?;
    let mut edits = Vec::new();
    let mut files: Vec<_> = [
        "action.yml",
        "scripts/action.sh",
        "README.md",
        ".github/ci-version",
    ]
    .into_iter()
    .map(|file| root.join(file))
    .collect();
    files.extend(
        checks::markdown_files(root)?
            .into_iter()
            .filter(|path| path.starts_with(root.join("docs"))),
    );
    for path in files {
        let text = fs::read_to_string(&path)?;
        let updated = if path == root.join("action.yml") {
            replace_version(
                &text,
                &format!("default: v{released}\n"),
                &format!("default: v{next}\n"),
            )?
        } else if path == root.join("scripts/action.sh") {
            replace_version(
                &text,
                &format!("INPUT_VERSION:-v{released}"),
                &format!("INPUT_VERSION:-v{next}"),
            )?
        } else if path == root.join(".github/ci-version") {
            format!("v{released}\n")
        } else {
            published
                .replace_all(&text, |captures: &Captures<'_>| {
                    format!("{}{released}", &captures[1])
                })
                .into_owned()
        };
        edits.push((path, updated));
    }
    for (path, text) in edits {
        fs::write(path, text)?;
    }
    Ok(())
}

pub fn execute() -> Result<()> {
    update(
        &env::current_dir()?,
        &env::var("RCR_RELEASE_VERSION")?,
        &env::var("RCR_NEXT_VERSION")?,
    )
}

pub fn publish_native_assets(root: &Path) -> Result<()> {
    let version = env::var("RELEASED_VERSION")?;
    let parsed = Version::parse(&version)?;
    if !parsed.pre.is_empty() || !parsed.build.is_empty() {
        return Err("Native assets require a stable release version".into());
    }
    let tag = format!("v{version}");
    let repository = env::var("GH_REPO")?;
    let actual = run(
        "gh",
        &[
            "api",
            &format!("repos/{repository}/commits/refs%2Ftags%2F{tag}"),
            "--jq",
            ".sha",
        ],
        root,
    )?;
    if actual.trim() != run("git", &["rev-parse", &format!("{tag}^{{commit}}")], root)?.trim() {
        return Err("Local and published release tags differ".into());
    }
    // Release preparation dates the changelog; native inputs must otherwise be identical.
    run(
        "git",
        &[
            "diff",
            "--exit-code",
            &env::var("GITHUB_SHA")?,
            &tag,
            "--",
            ".",
            ":!CHANGELOG.md",
        ],
        root,
    )?;
    let published = run(
        "gh",
        &[
            "release",
            "view",
            &tag,
            "--json",
            "isDraft,isPrerelease",
            "--jq",
            "[.isDraft,.isPrerelease] | all(. == false)",
        ],
        root,
    )?;
    if published.trim() != "true" {
        return Err("Native assets require a published stable GitHub Release".into());
    }
    let archives = std::path::PathBuf::from(
        env::var_os("NATIVE_ARCHIVE_DIRECTORY").ok_or("NATIVE_ARCHIVE_DIRECTORY is required")?,
    );
    if fs::read_dir(&archives)?.count() != TARGETS.len() * 2 {
        return Err("Expected exactly six native archives and their checksums".into());
    }
    let mut files = Vec::new();
    for target in TARGETS {
        let archive = format!("already-tested-{target}.tar.gz");
        let path = archives.join(&archive);
        let checksum = archives.join(format!("{archive}.sha256"));
        let digest: String = Sha256::digest(fs::read(&path)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if fs::read_to_string(&checksum)?.trim() != format!("{digest}  {archive}") {
            return Err(format!("Invalid native archive checksum for {archive}").into());
        }
        files.extend([path, checksum]);
    }
    let temporary = tempdir()?;
    for installer in ["install.sh", "install.ps1"] {
        let path = temporary.path().join(installer);
        fs::write(
            &path,
            run(
                "git",
                &["show", &format!("{tag}:scripts/{installer}")],
                root,
            )?,
        )?;
        files.push(path);
    }
    let instructions = run(
        "git",
        &["show", &format!("{tag}:assets/agent-instructions.md")],
        root,
    )?
    .replace("{{VERSION}}", &version);
    let name = "agent-instructions.md";
    let path = temporary.path().join(name);
    fs::write(&path, &instructions)?;
    let digest: String = Sha256::digest(instructions.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let checksum = temporary.path().join(format!("{name}.sha256"));
    fs::write(&checksum, format!("{digest}  {name}\n"))?;
    files.extend([path, checksum]);
    let assets = run(
        "gh",
        &[
            "release",
            "view",
            &tag,
            "--json",
            "assets",
            "--jq",
            ".assets[].name",
        ],
        root,
    )?;
    let download = temporary.path().join("download");
    fs::create_dir(&download)?;
    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid asset name")?;
        if !assets.lines().any(|asset| asset == name) {
            run(
                "gh",
                &[
                    "release",
                    "upload",
                    &tag,
                    file.to_str().ok_or("Invalid asset path")?,
                ],
                root,
            )?;
        }
        run(
            "gh",
            &[
                "release",
                "download",
                &tag,
                "--pattern",
                name,
                "--dir",
                download.to_str().ok_or("Invalid download path")?,
            ],
            root,
        )?;
        let downloaded = download.join(name);
        if fs::read(&file)? != fs::read(&downloaded)? {
            return Err(format!("Published native asset differs: {name}").into());
        }
        fs::remove_file(downloaded)?;
    }
    let message = format!(
        "Uploaded or verified native assets for `{tag}`, including installers and agent instructions."
    );
    if let Some(summary) = env::var_os("GITHUB_STEP_SUMMARY") {
        writeln!(
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(summary)?,
            "{message}"
        )?;
    }
    println!("{message}");
    Ok(())
}
