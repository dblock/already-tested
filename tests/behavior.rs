use already_tested::{Options, Verification, create_claim, verify_claim, verify_claim_details};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const WORKFLOW: &str = ".github/workflows/test.yml";

struct Repo {
    directory: tempfile::TempDir,
}

impl Repo {
    fn empty(format: &str) -> Self {
        let repo = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        repo.git(&[
            "init",
            "-q",
            "-b",
            "master",
            &format!("--object-format={format}"),
        ]);
        for (key, value) in [
            ("user.name", "Test"),
            ("user.email", "test@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("core.filemode", "false"),
        ] {
            repo.git(&["config", key, value]);
        }
        repo.git(&[
            "config",
            "core.hooksPath",
            repo.path().join(".git/no-hooks").to_str().unwrap(),
        ]);
        repo
    }

    fn new() -> Self {
        let repo = Self::empty("sha1");
        repo.write(WORKFLOW, "name: Test\non: push\njobs: {}\n");
        repo.write(".github/workflows/other.yml", "name: Other\n");
        repo.write("src/add.rs", "pub fn add(a: u8, b: u8) -> u8 { a + b }\n");
        repo.write("README.md", "# Fixture\n");
        repo.commit();
        repo
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn git_bytes(&self, args: &[&str], input: &[u8]) -> Vec<u8> {
        let mut child = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                stdin.write_all(input).unwrap();
            });
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        })
    }

    fn git(&self, args: &[&str]) -> String {
        String::from_utf8(self.git_bytes(args, &[]))
            .unwrap()
            .trim_end()
            .into()
    }

    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        let path = self.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn commit(&self) -> String {
        self.git(&["add", "--all"]);
        self.git(&["commit", "-qm", "Fixture"]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn options(&self) -> Options {
        Options::new(self.path(), WORKFLOW, "unit")
    }

    fn claim(&self) -> String {
        create_claim(&self.options()).unwrap()
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_already-tested"))
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap()
    }

    fn cli_input(&self, args: &[&str], input: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_already-tested"))
            .args(args)
            .current_dir(self.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                let _ = stdin.write_all(input);
            });
            child.wait_with_output().unwrap()
        })
    }

    fn index_file(&self, path: &str, mode: &str, bytes: &[u8]) {
        let oid =
            String::from_utf8(self.git_bytes(&["hash-object", "-w", "--stdin"], bytes)).unwrap();
        self.git_bytes(
            &["update-index", "-z", "--index-info"],
            format!("{mode} {}\t{path}\0", oid.trim()).as_bytes(),
        );
    }

    fn commit_index(&self) -> String {
        let tree = self.git(&["write-tree"]);
        let commit =
            String::from_utf8(self.git_bytes(&["commit-tree", &tree, "-m", "Fixture"], &[]))
                .unwrap();
        self.git(&["update-ref", "HEAD", commit.trim()]);
        commit.trim().into()
    }

    fn action(
        &self,
        body: serde_json::Value,
        source: &str,
        extra: &[(&str, Option<&str>)],
    ) -> (Output, String) {
        self.write(
            ".git/event.json",
            serde_json::to_vec(&serde_json::json!({
                "pull_request": { "body": body, "head": { "sha": source } }
            }))
            .unwrap(),
        );
        self.write(".git/output", "");
        let mut command = Command::new(env!("CARGO_BIN_EXE_already-tested"));
        command
            .arg("action")
            .current_dir(self.path())
            .env("GITHUB_EVENT_NAME", "pull_request")
            .env("GITHUB_EVENT_PATH", self.path().join(".git/event.json"))
            .env("GITHUB_OUTPUT", self.path().join(".git/output"))
            .env_remove("GITHUB_STEP_SUMMARY")
            .env("GITHUB_WORKSPACE", self.path())
            .env(
                "GITHUB_WORKFLOW_REF",
                format!("owner/project/{WORKFLOW}@refs/pull/1/merge"),
            )
            .env("GITHUB_WORKFLOW_SHA", self.git(&["rev-parse", "HEAD"]))
            .env("INPUT_CHECK", "unit")
            .env("INPUT_FILES", "");
        for (name, value) in extra {
            if let Some(value) = value {
                command.env(name, value);
            } else {
                command.env_remove(name);
            }
        }
        let result = command.output().unwrap();
        (
            result,
            fs::read_to_string(self.path().join(".git/output")).unwrap(),
        )
    }
}

#[derive(Deserialize)]
struct GoldenFile {
    path: String,
    mode: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct GoldenCase {
    name: String,
    patterns: Vec<String>,
    replacement: Option<GoldenFile>,
    claim: String,
}

#[derive(Deserialize)]
struct Golden {
    workflow: String,
    check: String,
    files: Vec<GoldenFile>,
    cases: Vec<GoldenCase>,
}

#[test]
fn legacy_v1_golden_bytes_unicode_order_binary_escapes_policy_and_workflow_override() {
    check_golden(include_str!("fixtures/v1.json"));
    #[cfg(unix)]
    check_golden(include_str!("fixtures/v1-unix.json"));
}

fn check_golden(json: &str) {
    let golden: Golden = serde_json::from_str(json).unwrap();
    for format in ["sha1", "sha256"] {
        for case in &golden.cases {
            let repo = Repo::empty(format);
            for file in &golden.files {
                repo.index_file(&file.path, &file.mode, &file.bytes);
            }
            let source = repo.commit_index();
            let mut options = Options::new(repo.path(), &golden.workflow, &golden.check);
            options.files = case.patterns.clone();
            options.r#ref = source;
            if let Some(file) = &case.replacement {
                repo.index_file(&file.path, &file.mode, &file.bytes);
                options.workflow_ref = Some(repo.commit_index());
            }
            assert!(
                verify_claim(Some(&case.claim), &options).unwrap(),
                "{format}: {}",
                case.name
            );
            let stale = case.claim.replace("sha256:", "sha256:0");
            assert!(!verify_claim(Some(&stale), &options).unwrap());
        }
    }
}

#[test]
fn deterministic_claims_ignore_commit_identity_but_bind_content_and_identity() {
    let repo = Repo::new();
    let claim = repo.claim();
    assert_eq!(claim, repo.claim());
    repo.git(&["commit", "--allow-empty", "-qm", "Same source, new commit"]);
    assert_eq!(claim, repo.claim());
    let mut options = repo.options();
    assert!(verify_claim(Some(&format!("Description\n{claim}\n{claim}")), &options).unwrap());
    options.check = "lint".into();
    assert!(!verify_claim(Some(&claim), &options).unwrap());
    options.workflow = ".github/workflows/other.yml".into();
    assert!(!verify_claim(Some(&claim), &options).unwrap());
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "File mutations affect only this test's unique temporary repository."
    )
)]
fn committed_content_paths_additions_deletions_workflow_and_mode_are_hashed() {
    for change in [
        "contents", "addition", "deletion", "rename", "workflow", "mode", "framing",
    ] {
        let repo = Repo::new();
        repo.write("src/a", "bc");
        repo.write("src/b", "d");
        repo.commit();
        let claim = repo.claim();
        match change {
            "contents" => repo.write("src/add.rs", "changed"),
            "addition" => repo.write("src/new", "new"),
            "deletion" => fs::remove_file(repo.path().join("src/add.rs")).unwrap(),
            "rename" => {
                repo.git(&["mv", "src/add.rs", "src/renamed.rs"]);
            }
            "workflow" => repo.write(WORKFLOW, "name: Changed\n"),
            "mode" => {
                repo.git(&["update-index", "--chmod=+x", "src/add.rs"]);
            }
            "framing" => {
                repo.write("src/a", "b");
                repo.write("src/b", "cd");
            }
            _ => unreachable!(),
        }
        if change == "mode" {
            repo.git(&["commit", "-qm", "Executable"]);
        } else {
            repo.commit();
        }
        assert!(
            !verify_claim(Some(&claim), &repo.options()).unwrap(),
            "{change}"
        );
    }
}

#[test]
fn filters_bind_policy_always_include_workflow_and_ignore_unselected_committed_changes() {
    let repo = Repo::new();
    let mut options = repo.options();
    options.files = vec!["src/**".into(), " README.md ".into(), "src/**".into()];
    let wide = create_claim(&options).unwrap();
    options.files = vec!["README.md".into(), "src/**".into()];
    assert_eq!(wide, create_claim(&options).unwrap());
    options.files = vec!["src/**".into()];
    let narrow = create_claim(&options).unwrap();
    assert!(!verify_claim(Some(&wide), &options).unwrap());
    repo.write("README.md", "unselected\n");
    repo.commit();
    assert!(verify_claim(Some(&narrow), &options).unwrap());
    repo.write(WORKFLOW, "name: Changed\n");
    repo.commit();
    assert!(!verify_claim(Some(&narrow), &options).unwrap());
}

#[test]
fn local_claims_reject_all_tracked_dirtiness_but_verification_never_mutates_index() {
    let repo = Repo::new();
    let mut options = repo.options();
    options.files = vec!["src/**".into()];
    let claim = create_claim(&options).unwrap();
    let source = repo.git(&["rev-parse", "HEAD"]);
    repo.write("README.md", "dirty outside selection");
    assert!(
        create_claim(&options)
            .unwrap_err()
            .to_string()
            .contains("Commit tracked changes")
    );
    repo.git(&["add", "README.md"]);
    assert!(create_claim(&options).is_err());
    let index = fs::read(repo.path().join(".git/index")).unwrap();
    options.r#ref = source;
    assert!(verify_claim(Some(&claim), &options).unwrap());
    assert_eq!(index, fs::read(repo.path().join(".git/index")).unwrap());
    repo.write("src/untracked", "ignored");
    assert!(verify_claim(Some(&claim), &options).unwrap());
    repo.git(&["reset", "--hard", "-q", "HEAD"]);
    assert_eq!(claim, create_claim(&options).unwrap());
    repo.git(&["config", "core.autocrlf", "true"]);
    assert!(verify_claim(Some(&claim), &options).unwrap());
}

#[test]
fn missing_malformed_unknown_version_and_noncanonical_claims_never_match() {
    let repo = Repo::new();
    let claim = repo.claim();
    for body in [
        None,
        Some(""),
        Some("Tests passed"),
        Some("<!-- already-tested:v1 nope -->"),
        Some(&claim.replace(":v1 ", ":v2 ")),
        Some(&claim.replace("sha256:", "sha512:")),
        Some(&claim.replace("\"workflow\":", "\"workflow\": ")),
    ] {
        assert!(!verify_claim(body, &repo.options()).unwrap());
    }
    let options = Options::new("nonexistent", "", "");
    assert!(!verify_claim(None, &options).unwrap());
}

#[test]
fn invalid_policy_and_refs_fail_before_generating_a_claim() {
    let repo = Repo::new();
    for workflow in [
        "",
        "../test.yml",
        "/test.yml",
        ".github/workflows/a/b.yml",
        ".github/workflows/a\\b.yml",
    ] {
        let mut options = repo.options();
        options.workflow = workflow.into();
        assert!(
            create_claim(&options)
                .unwrap_err()
                .to_string()
                .contains("workflow must")
        );
    }
    for check in [
        "",
        "two words",
        "$(touch bad)",
        &"a".repeat(129),
        "test-[",
        "test-{a,b",
        "test-\\",
        "[*]",
    ] {
        let mut options = repo.options();
        options.check = check.into();
        assert!(
            create_claim(&options)
                .unwrap_err()
                .to_string()
                .contains("check must")
        );
    }
    for pattern in [
        "",
        "\u{feff} \u{feff}",
        "a\0b",
        " :(attr:generated)**",
        ":(attr)src/**",
        "no-such-path",
        ":(nonsense)**",
    ] {
        let mut options = repo.options();
        options.files = vec![pattern.into()];
        assert!(create_claim(&options).is_err(), "{pattern:?}");
    }
    let mut options = repo.options();
    options.workflow = ".github/workflows/missing.yml".into();
    assert!(
        create_claim(&options)
            .unwrap_err()
            .to_string()
            .contains("not tracked")
    );
    let claim = repo.claim();
    for value in ["--help", "main", &"A".repeat(40), &"0".repeat(40)] {
        options = repo.options();
        options.r#ref = value.into();
        assert!(verify_claim(Some(&claim), &options).is_err());
        options = repo.options();
        options.workflow_ref = Some(value.into());
        assert!(verify_claim(Some(&claim), &options).is_err());
    }
}

#[test]
fn unsupported_index_modes_are_rejected_unless_excluded() {
    for mode in ["120000", "160000"] {
        let repo = Repo::new();
        let mut options = repo.options();
        options.files = vec!["src/**".into()];
        let claim = create_claim(&options).unwrap();
        let oid = if mode == "160000" {
            repo.git(&["rev-parse", "HEAD"])
        } else {
            String::from_utf8(repo.git_bytes(&["hash-object", "-w", "--stdin"], b"README.md"))
                .unwrap()
                .trim()
                .into()
        };
        repo.git(&[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("{mode},{oid},unsupported"),
        ]);
        repo.git(&["commit", "-qm", "Unsupported mode"]);
        assert!(verify_claim(Some(&claim), &options).unwrap());
        options.files.clear();
        assert!(
            verify_claim(Some(&claim), &options)
                .unwrap_err()
                .to_string()
                .contains("Unsupported file mode")
        );
    }
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "Object deletion affects only this test's unique temporary repository."
    )
)]
fn missing_blobs_and_replace_objects_cannot_forge_matches() {
    let repo = Repo::new();
    let claim = repo.claim();
    let original = repo.git(&["rev-parse", "HEAD:src/add.rs"]);
    let replacement =
        String::from_utf8(repo.git_bytes(&["hash-object", "-w", "--stdin"], b"replacement"))
            .unwrap();
    repo.git(&["replace", &original, replacement.trim()]);
    assert!(verify_claim(Some(&claim), &repo.options()).unwrap());
    let path = repo
        .path()
        .join(".git/objects")
        .join(&original[..2])
        .join(&original[2..]);
    #[cfg(windows)]
    {
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
    }
    fs::remove_file(path).unwrap();
    assert!(
        verify_claim(Some(&claim), &repo.options())
            .unwrap_err()
            .to_string()
            .contains("Cannot read committed file")
    );
}

#[test]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "Hook permissions affect only this test's unique temporary repository."
    )
)]
fn verification_uses_raw_committed_bytes_without_filters_or_index_hooks() {
    let repo = Repo::new();
    repo.write(".gitattributes", "src/** filter=trap\n");
    repo.commit();
    let claim = repo.claim();
    repo.git(&["config", "filter.trap.clean", "git --version"]);
    repo.git(&["config", "filter.trap.smudge", "git --version"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        repo.write(
            ".git/hooked/post-index-change",
            "#!/bin/sh\nprintf triggered > hook-marker\n",
        );
        fs::set_permissions(
            repo.path().join(".git/hooked/post-index-change"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        repo.git(&[
            "config",
            "core.hooksPath",
            repo.path().join(".git/hooked").to_str().unwrap(),
        ]);
    }
    assert!(verify_claim(Some(&claim), &repo.options()).unwrap());
    assert!(!repo.path().join("hook-marker").exists());
}

#[cfg(unix)]
#[test]
fn invalid_utf8_paths_and_real_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let repo = Repo::new();
    let claim = repo.claim();
    let oid = repo.git(&["rev-parse", "HEAD:src/add.rs"]);
    let mut entry = format!("100644 {oid}\t").into_bytes();
    entry.extend([255, 0]);
    repo.git_bytes(&["update-index", "-z", "--index-info"], &entry);
    repo.commit_index();
    assert!(
        verify_claim(Some(&claim), &repo.options())
            .unwrap_err()
            .to_string()
            .contains("valid UTF-8")
    );
    let repo = Repo::new();
    symlink("README.md", repo.path().join("link")).unwrap();
    repo.commit();
    assert!(
        create_claim(&repo.options())
            .unwrap_err()
            .to_string()
            .contains("Unsupported file mode")
    );
}

#[test]
fn cli_help_version_errors_and_nested_directory_behavior() {
    let repo = Repo::new();
    for args in [
        vec![],
        vec!["--help"],
        vec!["claim", "--help"],
        vec!["verify", "--help"],
    ] {
        let output = repo.cli(&args);
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .starts_with("Usage:")
        );
        assert!(output.stderr.is_empty());
    }
    let version = repo.cli(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("{}\n", env!("CARGO_PKG_VERSION"))
    );
    for args in [
        vec!["claim"],
        vec!["unknown"],
        vec!["--typo"],
        vec!["----help"],
        vec!["--workflow"],
        vec!["--version", "--help"],
        vec!["claim", "--version"],
    ] {
        let output = repo.cli(&args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with("already-tested:")
        );
    }
    let args = [
        "claim",
        "--workflow",
        WORKFLOW,
        "--check",
        "unit",
        "--files",
        "src/**",
        "--files",
        "README.md",
    ];
    let output = repo.cli(&args);
    assert!(output.status.success());
    let nested = Command::new(env!("CARGO_BIN_EXE_already-tested"))
        .args(args)
        .current_dir(repo.path().join("src"))
        .output()
        .unwrap();
    assert_eq!(output.stdout, nested.stdout);
    repo.write("README.md", "dirty");
    let dirty = repo.cli(&args);
    assert_eq!(dirty.status.code(), Some(1));
    assert!(dirty.stdout.is_empty());
}

#[test]
fn verify_cli_reads_claim_text_and_reports_matches_with_distinct_exit_codes() {
    let repo = Repo::new();
    let exact = repo.claim();
    let glob = create_claim(&Options::new(repo.path(), WORKFLOW, "u*")).unwrap();
    let blanket = create_claim(&Options::new(repo.path(), WORKFLOW, "*")).unwrap();
    let stale = exact.replace("sha256:", "sha256:0");
    for (body, scope, reason) in [
        (exact.as_str(), Some("unit"), "matched"),
        (glob.as_str(), Some("u*"), "matched"),
        (blanket.as_str(), Some("*"), "matched"),
        (stale.as_str(), None, "no_matching_claim"),
        ("<!-- already-tested:v1 nope -->", None, "no_matching_claim"),
        ("No claim.", None, "missing_claim"),
    ] {
        let args = [
            "verify",
            "--workflow",
            WORKFLOW,
            "--check",
            "unit",
            "--json",
        ];
        let result = repo.cli_input(&args, body.as_bytes());
        assert_eq!(result.status.code(), Some(i32::from(scope.is_none())));
        assert!(result.stderr.is_empty());
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["skip"], scope.is_some());
        assert_eq!(report["reason"], reason);
        assert_eq!(report["workflow"], WORKFLOW);
        assert_eq!(report["check"], "unit");
        assert_eq!(report["matched_check"], serde_json::json!(scope));
        repo.write(".git/claim.md", body);
        let mut from_file = args.to_vec();
        from_file.extend(["--body-file", ".git/claim.md"]);
        let result = repo.cli(&from_file);
        assert_eq!(result.status.code(), Some(i32::from(scope.is_none())));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
            report
        );
    }
    let args = ["verify", "--workflow", WORKFLOW, "--check", "unit"];
    let result = repo.cli_input(&args, exact.as_bytes());
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("Matched scope: unit")
    );
    let result = repo.cli_input(&args, stale.as_bytes());
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("Run CI normally.")
    );
    let result = repo.cli_input(
        &["verify", "--workflow", WORKFLOW, "--json"],
        blanket.as_bytes(),
    );
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["check"], "*");
    assert_eq!(report["matched_check"], "*");
}

#[test]
fn verify_cli_errors_are_distinct_bounded_and_machine_readable() {
    let repo = Repo::new();
    let claim = repo.claim();
    for extra in [
        vec![],
        vec!["--workflow", "bad.yml"],
        vec!["--workflow", ".github/workflows/missing.yml"],
        vec!["--workflow", WORKFLOW, "--check", ""],
        vec!["--workflow", WORKFLOW, "--check", "bad-["],
        vec!["--workflow", WORKFLOW, "--files", "missing"],
        vec!["--workflow", WORKFLOW, "--body-file", "missing"],
        vec!["--workflow", WORKFLOW, "--raw"],
        vec!["--workflow", WORKFLOW, "--command", "echo bad"],
        vec!["--workflow", WORKFLOW, "--summary", "passed"],
        vec!["--workflow", WORKFLOW, "--environment", "untested"],
        vec!["--workflow"],
        vec!["--typo"],
    ] {
        let mut args = vec!["verify", "--json"];
        args.extend(extra);
        let result = repo.cli_input(&args, claim.as_bytes());
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        assert!(result.stderr.is_empty(), "{args:?}");
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["skip"], false);
        assert_eq!(report["reason"], "error");
        assert!(report["matched_check"].is_null());
        assert!(!report["message"].as_str().unwrap().is_empty());
    }
    let args = ["verify", "--workflow", WORKFLOW, "--json"];
    for input in [vec![255], vec![b'x'; 1024 * 1024 + 1]] {
        let result = repo.cli_input(&args, &input);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stderr.is_empty());
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["reason"], "error");
    }
    let result = repo.cli(&["verify", "--workflow", WORKFLOW, "--body-file", "missing"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .starts_with("already-tested:")
    );
    for flag in ["--json", "--body-file=.git/claim.md"] {
        let result = repo.cli(&["claim", "--workflow", WORKFLOW, flag]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
    }
    let result = repo.cli_input(
        &["verify", "--workflow", "bad.yml", "--json"],
        b"No supported claim.",
    );
    assert_eq!(result.status.code(), Some(2));
    let empty = Repo::empty("sha1");
    let result = empty.cli_input(&args, b"No supported claim.");
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["reason"],
        "error"
    );
}

#[test]
fn verify_cli_checks_committed_head_and_filters_without_mutating_the_checkout() {
    let repo = Repo::new();
    let mut options = repo.options();
    options.files = vec!["src/**".into()];
    let claim = create_claim(&options).unwrap();
    repo.write(".git/claim.md", &claim);
    repo.write("src/add.rs", "dirty selected input");
    repo.git(&["add", "src/add.rs"]);
    repo.write("README.md", "dirty unselected input");
    let index = fs::read(repo.path().join(".git/index")).unwrap();
    let status = repo.git(&["status", "--porcelain"]);
    let body_path = repo.path().join(".git/claim.md");
    let args = [
        "verify",
        "--workflow",
        WORKFLOW,
        "--check",
        "unit",
        "--files",
        "src/**",
        "--body-file",
        body_path.to_str().unwrap(),
        "--json",
    ];
    let result = Command::new(env!("CARGO_BIN_EXE_already-tested"))
        .args(args)
        .current_dir(repo.path().join("src"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(fs::read(repo.path().join(".git/index")).unwrap(), index);
    assert_eq!(repo.git(&["status", "--porcelain"]), status);
    assert_eq!(
        fs::read_to_string(repo.path().join("src/add.rs")).unwrap(),
        "dirty selected input"
    );
    let unfiltered = repo.cli_input(
        &[
            "verify",
            "--workflow",
            WORKFLOW,
            "--check",
            "unit",
            "--json",
        ],
        claim.as_bytes(),
    );
    assert_eq!(unfiltered.status.code(), Some(1));
    repo.commit();
    let stale = repo.cli(&args);
    assert_eq!(stale.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stale.stdout).unwrap()["reason"],
        "no_matching_claim"
    );
}

#[test]
fn cli_disclosure_is_exact_and_display_metadata_is_inert() {
    let repo = Repo::new();
    let claim = repo.claim();
    let commit = repo.git(&["rev-parse", "HEAD"]);
    let parsed: serde_json::Value = serde_json::from_str(
        claim
            .strip_prefix(already_tested::PREFIX)
            .unwrap()
            .strip_suffix(" -->")
            .unwrap(),
    )
    .unwrap();
    let hash = parsed["hash"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let args = ["claim", "--workflow", WORKFLOW, "--check", "unit"];
    let output = repo.cli(&args);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "<details>\n<summary>unit - passed</summary>\n\n- **Workflow:** ` {WORKFLOW} `\n- **Check:** ` unit `\n- **Commit:** ` {commit} `\n- **Content SHA-256:** ` {hash} ` {claim}\n\n</details>\n"
        )
    );
    let command = "echo bad > executed";
    let mut with_metadata = args.to_vec();
    with_metadata.extend([
        "--summary",
        "<passed> & done",
        "--command",
        command,
        "--environment",
        "<host> & `tools` ``",
    ]);
    let output = repo.cli(&with_metadata);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("<summary>&lt;passed&gt; &amp; done</summary>\n\n- **Ran:** ` echo bad > executed `\n- **Environment:** ``` <host> & `tools` `` ```\n"));
    assert!(text.contains(&claim));
    assert!(!repo.path().join("executed").exists());
    let mut raw = args.to_vec();
    raw.push("--raw");
    assert_eq!(repo.cli(&raw).stdout, format!("{claim}\n").as_bytes());
    for option in ["--summary", "--command", "--environment"] {
        for value in ["", "  ", "\u{feff}", "first\nsecond", "first\rsecond"] {
            let mut invalid = args.to_vec();
            invalid.extend([option, value]);
            let output = repo.cli(&invalid);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("nonempty single-line")
            );
        }
        let mut invalid = raw.clone();
        invalid.extend([option, "metadata"]);
        let output = repo.cli(&invalid);
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("--raw cannot be combined")
        );
    }
}

#[test]
fn library_cli_and_action_complete_pr_head_and_description_edit_round_trip() {
    let repo = Repo::new();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let output = repo.cli(&["claim", "--workflow", WORKFLOW, "--check", "unit"]);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).unwrap();
    assert!(verify_claim(Some(&body), &repo.options()).unwrap());
    let (result, output) = repo.action(body.clone().into(), &source, &[]);
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
    let (result, output) = repo.action("Tests passed".into(), &source, &[]);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
    repo.write("src/add.rs", "changed committed source");
    let changed = repo.commit();
    let (result, output) = repo.action(body.into(), &changed, &[]);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
    let fresh = repo.claim();
    let (result, output) = repo.action(fresh.into(), &changed, &[]);
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
}

#[test]
fn blanket_claim_defaults_round_trip_without_inferring_blanket_from_named_claims() {
    let repo = Repo::new();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let default = repo.cli(&["claim", "--workflow", WORKFLOW]);
    assert!(default.status.success());
    let explicit = repo.cli(&["claim", "--workflow", WORKFLOW, "--check", "*"]);
    assert!(explicit.status.success());
    assert_eq!(default.stdout, explicit.stdout);
    let body = String::from_utf8(default.stdout).unwrap();
    assert!(verify_claim(Some(&body), &Options::new(repo.path(), WORKFLOW, "*")).unwrap());
    let unit = repo.claim();
    for (claim, check, expected) in [
        (body.as_str(), None, true),
        (body.as_str(), Some("*"), true),
        (body.as_str(), Some("unit"), true),
        (unit.as_str(), None, false),
        (unit.as_str(), Some("unit"), true),
        ("No claim.", None, false),
        (body.as_str(), Some("unit*"), true),
        (body.as_str(), Some(""), true),
    ] {
        let (result, output) = repo.action(claim.into(), &source, &[("INPUT_CHECK", check)]);
        assert!(result.status.success());
        assert_eq!(output, format!("skip={expected}\n"));
    }
    repo.write("src/add.rs", "changed source");
    let changed = repo.commit();
    let (result, output) = repo.action(body.into(), &changed, &[("INPUT_CHECK", None)]);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
}

#[test]
fn check_globs_require_full_scope_coverage_in_both_directions() {
    let repo = Repo::new();
    for (claim, gate, expected) in [
        ("*", "test-linux", true),
        ("test-*", "test-linux", true),
        ("test-*", "test-l*", true),
        ("test-linux", "test-*", false),
        ("test-l*", "test-*", false),
        ("test-*", "*", false),
        ("test-*", "lint-*", false),
        ("test-?", "test-[lm]", true),
        ("test-[lm]", "test-?", false),
        ("test-[lm]*", "test-linux", true),
        ("test-[lm]*", "test-[lw]*", false),
        ("test-[!w]*", "test-[lm]*", true),
        ("test-[!w]*", "test-*", false),
        ("{test,lint}-*", "test-*", true),
        ("test-*", "{test,lint}-*", false),
        ("test-*x", "test-x*", false),
        ("**", "*", true),
        ("test/*", "test/linux/*", true),
        ("test/*", "lint/*", false),
        ("?", "[a-z]", true),
        ("[a-z]", "?", false),
    ] {
        let claim = create_claim(&Options::new(repo.path(), WORKFLOW, claim)).unwrap();
        assert_eq!(
            verify_claim(Some(&claim), &Options::new(repo.path(), WORKFLOW, gate)).unwrap(),
            expected,
            "{claim} -> {gate}"
        );
    }
}

#[test]
fn glob_claims_keep_canonical_hash_binding_and_never_union_partial_claims() {
    let repo = Repo::new();
    let claim = create_claim(&Options::new(repo.path(), WORKFLOW, "test-*")).unwrap();
    let gate = Options::new(repo.path(), WORKFLOW, "test-linux");
    for invalid in [
        claim.replace("\"check\":", "\"check\": "),
        claim.replace("\"check\":\"test-*\"", "\"check\":\"*\""),
        claim.replace("sha256:", "sha256:0"),
        claim.replace(" -->", "oops -->"),
        claim.replace(":v1 ", ":v2 "),
    ] {
        assert!(!verify_claim(Some(&invalid), &gate).unwrap(), "{invalid}");
    }
    let partial = ["test-linux", "test-windows"]
        .map(|check| create_claim(&Options::new(repo.path(), WORKFLOW, check)).unwrap())
        .join("\n");
    assert!(
        !verify_claim(
            Some(&partial),
            &Options::new(repo.path(), WORKFLOW, "test-*")
        )
        .unwrap()
    );
    let source = repo.git(&["rev-parse", "HEAD"]);
    let (result, output) = repo.action(claim.into(), &source, &[("INPUT_CHECK", Some("test-l*"))]);
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
}

#[test]
fn verification_checks_later_scopes_and_refreshes_content_between_calls() {
    let repo = Repo::new();
    let gate = Options::new(repo.path(), WORKFLOW, "test-linux");
    let stale = ["test-{linux,a}", "test-{linux,b}"]
        .map(|check| create_claim(&Options::new(repo.path(), WORKFLOW, check)).unwrap())
        .join("\n");
    repo.write("src/add.rs", "changed source");
    repo.commit();
    let fresh = create_claim(&Options::new(repo.path(), WORKFLOW, "test-*")).unwrap();
    let body = format!("{stale}\n{fresh}");
    assert_eq!(
        verify_claim_details(Some(&body), &gate).unwrap(),
        Verification::Matched {
            check: "test-*".into()
        }
    );
    repo.write("src/add.rs", "changed again");
    repo.commit();
    assert_eq!(
        verify_claim_details(Some(&body), &gate).unwrap(),
        Verification::NoMatch
    );
    let fresh = create_claim(&Options::new(repo.path(), WORKFLOW, "test-*")).unwrap();
    assert!(verify_claim(Some(&format!("{body}\n{fresh}")), &gate).unwrap());
}

#[test]
fn check_glob_resource_limits_request_normal_ci() {
    let repo = Repo::new();
    let expensive = format!("*a{}", "?".repeat(32));
    assert!(
        create_claim(&Options::new(repo.path(), WORKFLOW, expensive))
            .unwrap_err()
            .to_string()
            .contains("Cannot compile check scope")
    );
    let body = (0..33).map(|id| {
        format!("<!-- already-tested:v1 {{\"workflow\":\"{WORKFLOW}\",\"check\":\"check-{id}\",\"hash\":\"sha256:bad\"}} -->")
    }).collect::<Vec<_>>().join("\n");
    let source = repo.git(&["rev-parse", "HEAD"]);
    let (result, output) = repo.action(body.into(), &source, &[("INPUT_CHECK", None)]);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("Too many distinct claim scopes")
    );
}

#[test]
fn blanket_claim_binds_workflow_and_filters_and_never_skips_pushes() {
    let repo = Repo::new();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let mut options = Options::new(repo.path(), WORKFLOW, "test-*");
    options.files = vec!["src/**".into()];
    let claim = create_claim(&options).unwrap();
    let extra = [
        ("INPUT_CHECK", Some("test-linux")),
        ("INPUT_FILES", Some("src/**")),
    ];
    let (result, output) = repo.action(claim.clone().into(), &source, &extra);
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
    for overrides in [
        vec![("INPUT_FILES", Some(""))],
        vec![("GITHUB_EVENT_NAME", Some("push"))],
        vec![(
            "GITHUB_WORKFLOW_REF",
            Some("owner/project/.github/workflows/other.yml@refs/heads/master"),
        )],
    ] {
        let mut inputs = extra.to_vec();
        inputs.extend(overrides);
        let (result, output) = repo.action(claim.clone().into(), &source, &inputs);
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
    }
    repo.write(WORKFLOW, "name: Changed workflow\n");
    repo.commit();
    let (result, output) = repo.action(claim.into(), &source, &extra);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
}

#[test]
fn action_binds_filters_identity_and_actual_executing_workflow_not_checkout() {
    let repo = Repo::new();
    let mut options = repo.options();
    options.files = vec!["src/**".into()];
    let claim = create_claim(&options).unwrap();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let (result, output) = repo.action(
        claim.clone().into(),
        &source,
        &[("INPUT_FILES", Some("\r\n\u{feff} src/**\u{feff}\r\n\n"))],
    );
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
    for extra in [
        vec![],
        vec![
            ("INPUT_CHECK", Some("other")),
            ("INPUT_FILES", Some("src/**")),
        ],
    ] {
        let (result, output) = repo.action(claim.clone().into(), &source, &extra);
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
    }
    repo.write(
        "README.md",
        "new base content irrelevant to selected source",
    );
    let base = repo.commit();
    let (result, output) = repo.action(
        claim.clone().into(),
        &source,
        &[
            ("INPUT_FILES", Some("src/**")),
            ("GITHUB_WORKFLOW_SHA", Some(&base)),
        ],
    );
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
    repo.write(WORKFLOW, "name: Workflow changed on base\n");
    repo.commit();
    let (result, output) = repo.action(claim.into(), &source, &[("INPUT_FILES", Some("src/**"))]);
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
}

#[test]
fn action_summaries_report_exact_glob_and_blanket_matches_without_copying_metadata() {
    let repo = Repo::new();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let summary = repo.path().join(".git/summary");
    let path = summary.to_str().unwrap();
    for (scope, gate, displayed_scope) in [
        ("unit", Some("unit"), "` unit `"),
        ("u*", Some("unit"), "` u* `"),
        ("*", None, "` * `"),
        ("*", Some(""), "` * `"),
        (
            "{unit,<b>|`[x](https://bad.invalid)}",
            Some("unit"),
            "`` {unit,<b>\\|`[x](https://bad.invalid)} ``",
        ),
    ] {
        let claim = create_claim(&Options::new(repo.path(), WORKFLOW, scope)).unwrap();
        let body = format!("<details>DO_NOT_COPY_METADATA</details>\n{claim}");
        repo.write(".git/summary", "Earlier step.\n");
        let (result, output) = repo.action(
            body.into(),
            &source,
            &[("GITHUB_STEP_SUMMARY", Some(path)), ("INPUT_CHECK", gate)],
        );
        assert!(result.status.success());
        assert_eq!(output, "skip=true\n");
        let report = fs::read_to_string(&summary).unwrap();
        assert!(report.starts_with("Earlier step.\n\n### Already Tested\n"));
        assert!(report.contains("Matching local claim; gated work may be skipped."));
        assert!(report.contains(&format!("| Workflow | ` {WORKFLOW} ` |")));
        let displayed_gate = if gate.is_none_or(str::is_empty) {
            "` * `"
        } else {
            "` unit `"
        };
        assert!(report.contains(&format!("| Gate | {displayed_gate} |")));
        assert!(report.contains(&format!("| Source commit | ` {source} ` |")));
        assert!(report.contains(&format!("| Matched claim | {displayed_scope} |")));
        assert!(report.contains("Claims are trusted statements, not proof that tests ran."));
        assert!(!report.contains("DO_NOT_COPY_METADATA"));
    }
}

#[test]
fn action_summaries_explain_fallbacks_without_guessing_why_a_claim_is_stale() {
    let repo = Repo::new();
    let claim = repo.claim();
    repo.write("src/add.rs", "changed source");
    let source = repo.commit();
    let summary = repo.path().join(".git/summary");
    let path = summary.to_str().unwrap();
    for body in ["No claim.", claim.as_str()] {
        repo.write(".git/summary", "");
        let (result, output) =
            repo.action(body.into(), &source, &[("GITHUB_STEP_SUMMARY", Some(path))]);
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
        let report = fs::read_to_string(&summary).unwrap();
        assert!(report.contains("Run CI normally."));
        assert!(
            report.contains("No matching claim was found for this gate and committed content.")
        );
        assert!(!report.contains("Matched claim"));
        assert!(!report.contains("source changed"));
    }
    for (extra, expected) in [
        (
            ("GITHUB_EVENT_NAME", Some("push")),
            "Only pull-request events can skip CI.",
        ),
        (
            ("GITHUB_WORKFLOW_SHA", None),
            "The executing workflow commit ID is missing or invalid.",
        ),
        (("INPUT_CHECK", Some("[<b>|`")), "[<b>|`"),
    ] {
        repo.write(".git/summary", "");
        let (result, output) = repo.action(
            claim.clone().into(),
            &source,
            &[("GITHUB_STEP_SUMMARY", Some(path)), extra],
        );
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
        let report = fs::read_to_string(&summary).unwrap();
        assert!(report.contains("Run CI normally."));
        assert!(report.contains(expected), "{report}");
    }
    let long_error = format!("<b>\n{}DO_NOT_COPY_TAIL", "x".repeat(1024));
    repo.write(".git/summary", "");
    let (result, output) = repo.action(
        claim.into(),
        &source,
        &[
            ("GITHUB_STEP_SUMMARY", Some(path)),
            ("GIT_CONFIG_COUNT", Some("1")),
            ("GIT_CONFIG_KEY_0", Some("core.abbrev")),
            ("GIT_CONFIG_VALUE_0", Some(&long_error)),
        ],
    );
    assert!(result.status.success());
    assert_eq!(output, "skip=false\n");
    let report = fs::read_to_string(&summary).unwrap();
    assert!(report.contains("Verification failed:"));
    assert!(report.contains("... `"));
    assert!(report.len() < 2048);
    assert!(!report.contains("DO_NOT_COPY_TAIL"));
    assert!(!report.contains("<b>\n"));
}

#[test]
fn summary_writes_append_and_never_change_the_action_decision_or_mask_output_errors() {
    let repo = Repo::new();
    let claim = repo.claim();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let summary = repo.path().join(".git/summary");
    let path = summary.to_str().unwrap();
    repo.write(".git/summary", "Earlier step.\n");
    for body in [claim.as_str(), "No claim."] {
        let (result, _) = repo.action(body.into(), &source, &[("GITHUB_STEP_SUMMARY", Some(path))]);
        assert!(result.status.success());
    }
    let previous = fs::read_to_string(&summary).unwrap();
    assert!(previous.starts_with("Earlier step.\n"));
    assert_eq!(previous.matches("### Already Tested").count(), 2);
    for invalid in [
        repo.path().to_path_buf(),
        repo.path().join("missing/summary"),
    ] {
        for (body, expected) in [
            (claim.as_str(), "skip=true\n"),
            ("No claim.", "skip=false\n"),
        ] {
            let (result, output) = repo.action(
                body.into(),
                &source,
                &[("GITHUB_STEP_SUMMARY", Some(invalid.to_str().unwrap()))],
            );
            assert!(result.status.success());
            assert_eq!(output, expected);
            assert!(
                String::from_utf8(result.stderr)
                    .unwrap()
                    .contains("Could not write job summary")
            );
        }
    }
    let (result, output) = repo.action(claim.clone().into(), &source, &[]);
    assert!(result.status.success());
    assert_eq!(output, "skip=true\n");
    assert!(result.stderr.is_empty());
    assert_eq!(fs::read_to_string(&summary).unwrap(), previous);
    let (result, _) = repo.action(
        claim.into(),
        &source,
        &[
            ("GITHUB_STEP_SUMMARY", Some(path)),
            ("GITHUB_OUTPUT", Some(repo.path().to_str().unwrap())),
        ],
    );
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(&summary).unwrap(), previous);
}

#[test]
fn action_fails_safe_for_untrusted_events_and_operational_failures_but_output_errors_fail() {
    let repo = Repo::new();
    let claim = repo.claim();
    let source = repo.git(&["rev-parse", "HEAD"]);
    let missing = repo.path().join("missing");
    let missing = missing.to_str().unwrap();
    let invalid_event = repo.path().join("README.md");
    let invalid_event = invalid_event.to_str().unwrap();
    for (name, value) in [
        ("GITHUB_EVENT_PATH", Some(missing)),
        ("GITHUB_EVENT_PATH", Some(invalid_event)),
        ("GITHUB_WORKFLOW_REF", Some("")),
        ("GITHUB_WORKFLOW_REF", None),
        ("GITHUB_WORKFLOW_SHA", Some("")),
        ("GITHUB_WORKSPACE", Some(missing)),
        ("INPUT_FILES", Some("missing-path")),
    ] {
        let (result, output) = repo.action(claim.clone().into(), &source, &[(name, value)]);
        assert!(
            result.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(output, "skip=false\n");
        let log = String::from_utf8(result.stdout).unwrap();
        assert!(log.contains("Running CI normally."), "{name}: {log}");
        assert_eq!(log.lines().count(), 2);
    }
    for source in ["--help", &"0".repeat(40)] {
        let (result, output) = repo.action(claim.clone().into(), source, &[]);
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains("Running CI normally.")
        );
    }
    for event in ["push", "workflow_dispatch", "pull_request_target"] {
        let (result, output) = repo.action(
            claim.clone().into(),
            &source,
            &[
                ("GITHUB_EVENT_NAME", Some(event)),
                ("GITHUB_EVENT_PATH", Some(missing)),
            ],
        );
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
        assert!(
            !String::from_utf8(result.stdout)
                .unwrap()
                .contains("Running CI normally.")
        );
    }
    for value in [
        serde_json::Value::Null,
        serde_json::json!(42),
        serde_json::json!("<!-- already-tested:v2 nope -->"),
    ] {
        let (result, output) = repo.action(value, &source, &[]);
        assert!(result.status.success());
        assert_eq!(output, "skip=false\n");
    }
    for output_path in [None, Some(repo.path().to_str().unwrap()), Some(missing)] {
        // A missing file itself is appendable; a missing parent is not.
        let nested = format!("{missing}/output");
        let output_path = output_path.map(|path| {
            if path == missing {
                nested.as_str()
            } else {
                path
            }
        });
        let (result, _) = repo.action(
            claim.clone().into(),
            &source,
            &[("GITHUB_OUTPUT", output_path)],
        );
        assert_eq!(result.status.code(), Some(1));
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .starts_with("already-tested:")
        );
    }
    repo.write("append-output", "existing\n");
    let path = repo.path().join("append-output");
    let (result, _) = repo.action(
        claim.into(),
        &source,
        &[("GITHUB_OUTPUT", Some(path.to_str().unwrap()))],
    );
    assert!(result.status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), "existing\nskip=true\n");
}

#[test]
fn committed_blob_output_is_capped_at_64_mib() {
    let repo = Repo::new();
    let claim = repo.claim();
    repo.index_file("large.bin", "100644", &vec![0; 64 * 1024 * 1024]);
    repo.commit_index();
    assert!(
        verify_claim(Some(&claim), &repo.options())
            .unwrap_err()
            .to_string()
            .contains("64 MiB")
    );
}
