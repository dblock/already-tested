use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const COPILOT: &str = ".github/copilot-instructions.md";
const FRONTMATTER: &str = "---\napplyTo: \"**\"\n---\n\n";
const REFERENCE_START: &str = "<!-- already-tested:reference -->";
const REFERENCE_END: &str = "<!-- /already-tested:reference -->";

pub fn bundled() -> String {
    include_str!("../assets/agent-instructions.md")
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}

fn managed(body: &str) -> String {
    let digest: String = Sha256::digest(body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("<!-- already-tested:managed sha256:{digest} -->\n{body}")
}

fn read(root: &Path, relative: &Path) -> super::Result<Option<Vec<u8>>> {
    let destination = root.join(relative);
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if path == destination => {
                if !metadata.file_type().is_file() {
                    return Err("Instruction files must be regular files, not symlinks.".into());
                }
                return Ok(Some(fs::read(path)?));
            }
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err("Instruction directories must not be symlinks or files.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
    }
    Err("An instruction file path is required.".into())
}

fn write(
    root: &Path,
    relative: &Path,
    previous: Option<&[u8]>,
    updated: &str,
) -> super::Result<()> {
    if previous == Some(updated.as_bytes()) {
        return Ok(());
    }
    let mut directory = root.to_path_buf();
    for component in relative.parent().unwrap().components() {
        directory.push(component);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
            return Err("Instruction directories must not be symlinks or files.".into());
        }
    }
    let path = root.join(relative);
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    temporary.write_all(updated.as_bytes())?;
    if previous.is_some() {
        temporary
            .as_file()
            .set_permissions(fs::metadata(&path)?.permissions())?;
    }
    temporary.as_file().sync_all()?;
    if read(root, relative)?.as_deref() != previous {
        return Err(
            "Instructions changed during the update; retry without concurrent edits.".into(),
        );
    }
    if previous.is_some() {
        temporary.persist(&path)?;
    } else {
        temporary.persist_noclobber(&path)?;
    }
    Ok(())
}

fn repository_path(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn reference(previous: &str, path: &Path) -> super::Result<String> {
    let block = format!(
        "{REFERENCE_START}\nRead {} from the repository root before generating or updating local CI claims. Apply this project's own commands and environment policy.\n{REFERENCE_END}",
        super::code(&repository_path(path)),
    );
    match (previous.find(REFERENCE_START), previous.find(REFERENCE_END)) {
        (None, None) => {
            let separator = if previous.is_empty() || previous.ends_with("\n\n") {
                ""
            } else if previous.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            Ok(format!("{previous}{separator}{block}\n"))
        }
        (Some(start), Some(end))
            if start < end
                && previous.matches(REFERENCE_START).count() == 1
                && previous.matches(REFERENCE_END).count() == 1 =>
        {
            Ok(format!(
                "{}{block}{}",
                &previous[..start],
                &previous[end + REFERENCE_END.len()..],
            ))
        }
        _ => Err("The already-tested reference markers are incomplete or duplicated; repair them before updating.".into()),
    }
}

pub fn update(path: Option<&Path>, force: bool) -> super::Result<(PathBuf, PathBuf)> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err("Instruction updates require a Git working tree.".into());
    }
    let root = String::from_utf8(output.stdout)?;
    let root = Path::new(root.trim_end_matches(['\r', '\n']));
    let copilot = read(root, Path::new(COPILOT))?;
    let (policy, policy_bytes) = if let Some(bytes) = copilot {
        (Path::new(COPILOT), Some(bytes))
    } else {
        let policy = Path::new("AGENTS.md");
        (policy, read(root, policy)?)
    };
    let path = path.unwrap_or_else(|| {
        Path::new(if policy == Path::new(COPILOT) {
            ".github/instructions/already-tested.instructions.md"
        } else {
            ".already-tested/agent-instructions.md"
        })
    });
    if path.as_os_str().is_empty()
        || path.to_string_lossy().chars().any(char::is_control)
        || !path.components().all(|component| {
            matches!(component, Component::Normal(name) if !name.to_string_lossy().eq_ignore_ascii_case(".git"))
        })
    {
        return Err("--path must be a repository-relative file outside .git, without parent traversal.".into());
    }
    let path: PathBuf = path.components().collect();
    if ["AGENTS.md", COPILOT]
        .iter()
        .any(|policy| repository_path(&path).eq_ignore_ascii_case(policy))
    {
        return Err(
            "The managed guide must be separate from AGENTS.md and copilot-instructions.md.".into(),
        );
    }
    let previous = read(root, &path)?;
    let mut updated = managed(&bundled());
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with(".instructions.md"))
    {
        updated.insert_str(0, FRONTMATTER);
    }
    if let Some(bytes) = &previous
        && !force
        && !std::str::from_utf8(bytes).ok().is_some_and(|text| {
            let text = text.strip_prefix(FRONTMATTER).unwrap_or(text);
            text.split_once('\n')
                .is_some_and(|(_, body)| text == managed(body))
        })
    {
        return Err(format!("Instructions have local edits or are not managed; move project policy to {}, then use --force to replace them.", policy.display()).into());
    }
    let policy_text = policy_bytes
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()?
        .unwrap_or_default();
    let policy_updated = reference(policy_text, &path)?;
    write(root, &path, previous.as_deref(), &updated)?;
    write(root, policy, policy_bytes.as_deref(), &policy_updated)?;
    Ok((root.join(path), root.join(policy)))
}
