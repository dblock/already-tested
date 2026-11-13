//! Content-addressed claims over committed Git files.
//!
//! [`create_claim`] requires a clean tracked working tree. [`verify_claim`]
//! compares an exact canonical claim against committed source, independently of
//! the caller's index and checkout. Display metadata is deliberately not an input.

use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod check;
mod ci;

pub use ci::Ci;

pub const PREFIX: &str = "<!-- already-tested:v1 ";
const LIMIT: u64 = 64 * 1024 * 1024;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}
fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(Error(message.into()))
}

/// Snapshot policy. `workflow_ref = None` uses the source `ref`.
#[derive(Clone, Debug)]
pub struct Options {
    pub cwd: PathBuf,
    pub ci: Ci,
    pub workflow: String,
    pub check: String,
    pub files: Vec<String>,
    pub r#ref: String,
    pub workflow_ref: Option<String>,
}

impl Options {
    pub fn new(
        cwd: impl Into<PathBuf>,
        workflow: impl Into<String>,
        check: impl Into<String>,
    ) -> Self {
        Self {
            cwd: cwd.into(),
            ci: Ci::Auto,
            workflow: workflow.into(),
            check: check.into(),
            files: Vec::new(),
            r#ref: "HEAD".into(),
            workflow_ref: None,
        }
    }
}

/// ECMAScript String.prototype.trim whitespace (not Rust's Unicode White_Space).
pub fn trim_js(value: &str) -> &str {
    value.trim_matches(|c| {
        matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
            '\u{205f}' | '\u{3000}' | '\u{feff}')
    })
}

fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn normalize_files(files: &[String]) -> Result<Vec<String>> {
    let attr = Regex::new(r"^:\([^)]*\battr(?:[:\s)])").unwrap();
    let mut patterns = Vec::new();
    for pattern in files {
        let value = trim_js(pattern);
        if value.is_empty() || value.contains('\0') {
            return fail("File patterns must be nonempty strings without NUL characters.");
        }
        if attr.is_match(value) {
            return fail("Attribute-based Git path patterns are not supported.");
        }
        patterns.push(value.to_owned());
    }
    patterns.sort_by(|a, b| utf16_cmp(a, b));
    patterns.dedup();
    Ok(patterns)
}

pub fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn git(cwd: &Path, args: &[&str], index: Option<&Path>, input: &[u8]) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let result = stdin.write_all(input);
            drop(stdin);
            result
        });
        let errors = scope.spawn(move || {
            let mut bytes = Vec::new();
            stderr.by_ref().take(LIMIT + 1).read_to_end(&mut bytes)?;
            // Continue draining so a noisy failure cannot deadlock the child.
            std::io::copy(&mut stderr, &mut std::io::sink())?;
            Ok::<_, std::io::Error>(bytes)
        });
        let mut bytes = Vec::new();
        let read = stdout.by_ref().take(LIMIT + 1).read_to_end(&mut bytes);
        if bytes.len() as u64 > LIMIT || read.is_err() {
            let _ = child.kill();
        }
        let status = child.wait()?;
        let written = writer
            .join()
            .map_err(|_| Error("Git input thread failed".into()))?;
        let errors = errors
            .join()
            .map_err(|_| Error("Git error thread failed".into()))??;
        read?;
        if bytes.len() as u64 > LIMIT || errors.len() as u64 > LIMIT {
            return fail("Git output exceeded the 64 MiB limit.");
        }
        if !status.success() {
            return fail(format!(
                "Git command failed: {}",
                String::from_utf8_lossy(&errors).trim()
            ));
        }
        written?;
        Ok(bytes)
    })
}

fn text(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| Error("File paths must be valid UTF-8.".into()))
}

pub fn head_commit(cwd: &Path) -> Result<String> {
    Ok(
        text(git(cwd, &["rev-parse", "--verify", "HEAD"], None, &[])?)?
            .trim()
            .to_owned(),
    )
}

#[derive(Clone)]
struct Entry {
    path: String,
    mode: String,
    oid: String,
}

fn entries(bytes: Vec<u8>) -> Result<Vec<Entry>> {
    let text = text(bytes)?;
    let mut entries = Vec::new();
    for record in text.split('\0').filter(|s| !s.is_empty()) {
        let (header, path) = record
            .split_once('\t')
            .ok_or_else(|| Error("Invalid Git index entry.".into()))?;
        let parts: Vec<_> = header.split(' ').collect();
        if parts.len() != 3 || parts[0].len() != 6 || parts[2] != "0" || path.is_empty() {
            return fail("Invalid Git index entry.");
        }
        entries.push(Entry {
            path: path.into(),
            mode: parts[0].into(),
            oid: parts[1].into(),
        });
    }
    Ok(entries)
}

#[derive(Serialize)]
struct Claim<'a> {
    workflow: &'a str,
    check: &'a str,
    hash: String,
}

struct Snapshot {
    patterns: Vec<String>,
    entries: Vec<Entry>,
    contents: Vec<u8>,
}

fn validate_identity(options: &Options) -> Result<check::Scope> {
    if !options.workflow.ends_with(".yml") && !options.workflow.ends_with(".yaml")
        || options.workflow.contains(['\\', ':'])
        || options.workflow.chars().any(char::is_control)
        || options
            .workflow
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
    {
        return fail(
            "workflow must be a repository-relative .yml or .yaml path without traversal.",
        );
    }
    if options.ci.resolve()? == Ci::GitHubActions
        && !Regex::new(r"^\.github/workflows/[^/\\\x00\r\n]+\.ya?ml$")
            .unwrap()
            .is_match(&options.workflow)
    {
        return fail(
            "GitHub Actions workflow must be a file directly under .github/workflows/ ending in .yml or .yaml.",
        );
    }
    check::Scope::new(&options.check)
}

fn snapshot(options: &Options) -> Result<Snapshot> {
    let workflow_ref = options.workflow_ref.as_deref().unwrap_or(&options.r#ref);
    for value in [options.r#ref.as_str(), workflow_ref] {
        if value != "HEAD" && !valid_commit(value) {
            return fail("ref and workflowRef must be HEAD or full lowercase Git commit IDs.");
        }
    }
    let patterns = normalize_files(&options.files)?;
    let root = text(git(
        &options.cwd,
        &["rev-parse", "--show-toplevel"],
        None,
        &[],
    )?)?;
    let root = root.strip_suffix('\n').unwrap_or(&root);
    let root = Path::new(root.strip_suffix('\r').unwrap_or(root));
    let resolve = |value: &str| -> Result<String> {
        Ok(text(git(
            root,
            &["rev-parse", "--verify", &format!("{value}^{{commit}}")],
            None,
            &[],
        )?)?
        .trim()
        .into())
    };
    let commit = resolve(&options.r#ref)?;
    // Keep the scratch index off the caller's index and disable read-tree hooks.
    let temporary = tempfile::Builder::new()
        .prefix(".already-tested-index-")
        .tempdir_in(root)?;
    let index = temporary.path().join("index");
    let hooks = format!("core.hooksPath={}", temporary.path().display());
    git(
        root,
        &["-c", &hooks, "read-tree", &commit],
        Some(&index),
        &[],
    )?;
    let mut args = vec!["ls-files", "--stage", "-z", "--"];
    args.extend(patterns.iter().map(String::as_str));
    let selected = entries(git(root, &args, Some(&index), &[])?)?;
    if workflow_ref != options.r#ref {
        git(
            root,
            &["-c", &hooks, "read-tree", &resolve(workflow_ref)?],
            Some(&index),
            &[],
        )?;
    }
    let workflow = if workflow_ref == options.r#ref && patterns.is_empty() {
        selected
            .iter()
            .filter(|entry| entry.path == options.workflow)
            .cloned()
            .collect()
    } else {
        entries(git(
            root,
            &[
                "ls-files",
                "--stage",
                "-z",
                "--",
                &format!(":(top,literal){}", options.workflow),
            ],
            Some(&index),
            &[],
        )?)?
    };
    if workflow.len() != 1 {
        return fail(format!("Workflow is not tracked: {}", options.workflow));
    }
    if selected.is_empty() {
        return fail("File patterns matched no tracked files.");
    }
    let mut combined = BTreeMap::new();
    for entry in selected.into_iter().chain(workflow) {
        combined.insert(entry.path.clone(), entry);
    }
    let mut combined: Vec<_> = combined.into_values().collect();
    combined.sort_by(|a, b| utf16_cmp(&a.path, &b.path));
    for entry in &combined {
        if !matches!(entry.mode.as_str(), "100644" | "100755") {
            return fail(format!(
                "Unsupported file mode {}: {}",
                entry.mode, entry.path
            ));
        }
    }
    let input = combined
        .iter()
        .map(|e| format!("{}\n", e.oid))
        .collect::<String>();
    let contents = git(root, &["cat-file", "--batch"], None, input.as_bytes())?;
    Ok(Snapshot {
        patterns,
        entries: combined,
        contents,
    })
}

impl Snapshot {
    fn claim(&self, workflow: &str, check: &str) -> Result<String> {
        let Self {
            patterns,
            entries,
            contents,
        } = self;
        let mut hash = Sha256::new();
        hash.update(serde_json::to_vec(&(
            "already-tested",
            1,
            workflow,
            check,
            patterns,
        ))?);
        hash.update(b"\n");
        let mut offset = 0;
        for entry in entries {
            let end = contents
                .get(offset..)
                .and_then(|b| b.iter().position(|&b| b == b'\n'))
                .map(|n| n + offset)
                .ok_or_else(|| Error(format!("Cannot read committed file: {}", entry.path)))?;
            let header = std::str::from_utf8(&contents[offset..end]).unwrap_or("");
            let parts: Vec<_> = header.split(' ').collect();
            let size = parts.get(2).and_then(|s| s.parse::<usize>().ok());
            if parts.first() != Some(&entry.oid.as_str())
                || parts.get(1) != Some(&"blob")
                || size.is_none()
            {
                return fail(format!("Cannot read committed file: {}", entry.path));
            }
            let size = size.unwrap();
            let start = end + 1;
            let end = start
                .checked_add(size)
                .filter(|&n| n < contents.len())
                .ok_or_else(|| Error(format!("Cannot read committed file: {}", entry.path)))?;
            if contents[end] != b'\n' {
                return fail(format!("Cannot read committed file: {}", entry.path));
            }
            hash.update(serde_json::to_vec(&(&entry.path, &entry.mode, size))?);
            hash.update(b"\n");
            hash.update(&contents[start..end]);
            offset = end + 1;
        }
        let digest: String = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let claim = Claim {
            workflow,
            check,
            hash: format!("sha256:{digest}"),
        };
        Ok(format!("{PREFIX}{} -->", serde_json::to_string(&claim)?))
    }
}

/// Create a canonical v1 claim. Always uses HEAD for both source and workflow.
pub fn create_claim(options: &Options) -> Result<String> {
    if !git(
        &options.cwd,
        &["status", "--porcelain=v1", "-z", "--untracked-files=no"],
        None,
        &[],
    )?
    .is_empty()
    {
        return fail(
            "Commit tracked changes before generating a claim, then test that committed source.",
        );
    }
    let mut options = options.clone();
    options.r#ref = "HEAD".into();
    options.workflow_ref = None;
    validate_identity(&options)?;
    snapshot(&options)?.claim(&options.workflow, &options.check)
}

/// Missing/unknown claims return false without reading Git. Operational failures
/// return errors; callers deciding whether to skip must treat those as no match.
pub fn verify_claim(body: Option<&str>, options: &Options) -> Result<bool> {
    Ok(matches!(
        verify_claim_details(body, options)?,
        Verification::Matched { .. }
    ))
}

/// Whether a canonical claim covers the requested gate and committed content.
#[derive(Debug, PartialEq, Eq)]
pub enum Verification {
    Matched { check: String },
    NoMatch,
}

/// Verify a claim and retain its matched scope. Errors must request normal CI.
pub fn verify_claim_details(body: Option<&str>, options: &Options) -> Result<Verification> {
    let Some(body) = body.filter(|b| b.contains(PREFIX)) else {
        return Ok(Verification::NoMatch);
    };
    verify_committed_claim(body, options)
}

/// Verify local committed inputs, validating the policy and repository even when
/// the supplied text has no supported claim. Does not require a clean checkout.
pub fn verify_committed_claim(body: &str, options: &Options) -> Result<Verification> {
    let gate = validate_identity(options)?;
    let snapshot = snapshot(options)?;
    if body.contains(&snapshot.claim(&options.workflow, &options.check)?) {
        return Ok(Verification::Matched {
            check: options.check.clone(),
        });
    }
    let mut seen = std::collections::HashSet::from([options.check.clone()]);
    for (offset, _) in body.match_indices(PREFIX) {
        let part = &body[offset + PREFIX.len()..];
        let mut values = serde_json::Deserializer::from_str(part).into_iter::<serde_json::Value>();
        let Some(Ok(value)) = values.next() else {
            continue;
        };
        if !part[values.byte_offset()..].starts_with(" -->")
            || value["workflow"].as_str() != Some(options.workflow.as_str())
        {
            continue;
        }
        let Some(check) = value["check"].as_str() else {
            continue;
        };
        if !seen.insert(check.to_owned()) {
            continue;
        }
        if seen.len() > 32 {
            return fail("Too many distinct claim scopes.");
        }
        let Ok(scope) = check::Scope::new(check) else {
            continue;
        };
        if scope.covers(&gate)? && body.contains(&snapshot.claim(&options.workflow, check)?) {
            return Ok(Verification::Matched {
                check: check.to_owned(),
            });
        }
    }
    Ok(Verification::NoMatch)
}
