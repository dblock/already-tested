use already_tested::{
    Options, Verification, create_claim, head_commit, trim_js, valid_commit, verify_claim_details,
    verify_committed_claim,
};
use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

const HELP: &str = "Usage: already-tested claim --workflow <path> [--check <identifier>] [--files <pattern> ...]
       already-tested verify --workflow <path> [--check <identifier>] [--files <pattern> ...] [--body-file <path>] [--json]

Run tests against committed source, then paste the generated expandable section into the PR description.
Verify reads a claim or PR description from stdin or --body-file and checks committed HEAD.

Options:
  --workflow <path>      Tracked .github/workflows/*.yml or *.yaml file
  --check <identifier>   Check name or glob (default: * claims all work covered by the workflow gate)
  --files <pattern>      Git path pattern; repeat to select files (default: all tracked files)
  --summary <text>       Disclosure label (default: <check> - passed)
  --command <text>       Command already validated; recorded, never executed
  --environment <text>   Environment actually validated; recorded, never inferred
  --raw                 Print only the canonical hidden claim, without display metadata
  --body-file <path>     Read UTF-8 claim text from a file instead of stdin (verify only)
  --json                Print a machine-readable verdict or error (verify only)
  --help                Show this help
  --version             Show the package version

Verify exit codes: 0 = match, 1 = no match, 2 = invalid input or verification error.
";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn code(value: &str) -> String {
    let longest = value.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    format!("{fence} {value} {fence}")
}

fn format_claim(claim: &str, values: &HashMap<String, String>, cwd: &Path) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(
        claim
            .strip_prefix(already_tested::PREFIX)
            .unwrap()
            .strip_suffix(" -->")
            .unwrap(),
    )?;
    let workflow = value["workflow"].as_str().unwrap();
    let check = value["check"].as_str().unwrap();
    let hash = value["hash"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let default = if check == "*" {
        "All workflow checks - passed".to_owned()
    } else {
        format!("{check} - passed")
    };
    let summary = values
        .get("summary")
        .unwrap_or(&default)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut lines = Vec::new();
    if let Some(command) = values.get("command") {
        lines.push(format!("- **Ran:** {}", code(command)));
    }
    if let Some(environment) = values.get("environment") {
        lines.push(format!("- **Environment:** {}", code(environment)));
    }
    lines.extend([
        format!("- **Workflow:** {}", code(workflow)),
        format!("- **Check:** {}", code(check)),
        format!("- **Commit:** {}", code(&head_commit(cwd)?)),
        format!("- **Content SHA-256:** {} {claim}", code(hash)),
    ]);
    Ok(format!(
        "<details>\n<summary>{summary}</summary>\n\n{}\n\n</details>",
        lines.join("\n")
    ))
}

struct ActionReport {
    options: Options,
    verification: Verification,
}

fn action_verdict() -> Result<Option<ActionReport>> {
    if env::var("GITHUB_EVENT_NAME").as_deref() != Ok("pull_request") {
        return Ok(None);
    }
    let event: serde_json::Value = serde_json::from_slice(&fs::read(
        env::var_os("GITHUB_EVENT_PATH").ok_or("GITHUB_EVENT_PATH is missing.")?,
    )?)?;
    let source = env::var("GITHUB_SHA")
        .ok()
        .filter(|s| valid_commit(s))
        .ok_or("The queued source commit ID is missing or invalid.")?;
    let workflow_ref = env::var("GITHUB_WORKFLOW_SHA")
        .ok()
        .filter(|s| valid_commit(s))
        .ok_or("The executing workflow commit ID is missing or invalid.")?;
    let workflow = env::var("GITHUB_WORKFLOW_REF").unwrap_or_default();
    let captures = regex::Regex::new(r"^[^/]+/[^/]+/(.+?)@refs/").unwrap();
    let workflow = captures
        .captures(&workflow)
        .and_then(|c| c.get(1))
        .ok_or("workflow must be a file under .github/workflows/ ending in .yml or .yaml.")?;
    let check = env::var("INPUT_CHECK").unwrap_or_default();
    let mut options = Options::new(
        env::var_os("GITHUB_WORKSPACE")
            .map(std::path::PathBuf::from)
            .unwrap_or(env::current_dir()?),
        workflow.as_str(),
        if check.is_empty() { "*" } else { &check },
    );
    options.r#ref = source;
    options.workflow_ref = Some(workflow_ref);
    options.files = env::var("INPUT_FILES")
        .unwrap_or_default()
        .split('\n')
        .map(trim_js)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let verification = verify_claim_details(
        event.pointer("/pull_request/body").and_then(|b| b.as_str()),
        &options,
    )?;
    Ok(Some(ActionReport {
        options,
        verification,
    }))
}

fn summary_code(value: &str) -> String {
    let mut characters = value.chars();
    let mut output: String = characters
        .by_ref()
        .take(512)
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    if characters.next().is_some() {
        output.push_str("...");
    }
    code(&output)
}

fn action_summary(verdict: &Result<Option<ActionReport>>) -> String {
    let mut summary = String::from("\n### Already Tested\n\n");
    match verdict {
        Ok(Some(report)) => {
            match &report.verification {
                Verification::Matched { .. } => summary.push_str(
                    "**Matching local claim; gated work may be skipped.**\n\n",
                ),
                Verification::NoMatch => summary.push_str(
                    "**Run CI normally.**\n\nNo matching claim was found for this gate and committed content.\n\n",
                ),
            }
            summary.push_str("| Field | Value |\n| --- | --- |\n");
            for (name, value) in [
                ("Workflow", report.options.workflow.as_str()),
                ("Gate", report.options.check.as_str()),
                ("Source commit", report.options.r#ref.as_str()),
            ] {
                writeln!(
                    summary,
                    "| {name} | {} |",
                    summary_code(value).replace('|', "\\|")
                )
                .unwrap();
            }
            if let Verification::Matched { check } = &report.verification {
                writeln!(
                    summary,
                    "| Matched claim | {} |",
                    summary_code(check).replace('|', "\\|")
                )
                .unwrap();
            }
        }
        Ok(None) => {
            summary.push_str("**Run CI normally.**\n\nOnly pull-request events can skip CI.\n")
        }
        Err(error) => {
            writeln!(
                summary,
                "**Run CI normally.**\n\nVerification failed: {}\n",
                summary_code(&error.to_string())
            )
            .unwrap();
        }
    }
    summary.push_str("\nClaims are trusted statements, not proof that tests ran.\n");
    summary
}

fn action() -> Result<()> {
    let verdict = action_verdict();
    let skip = match &verdict {
        Ok(Some(report)) => matches!(report.verification, Verification::Matched { .. }),
        Ok(None) => false,
        Err(error) => {
            println!(
                "already-tested: {} Running CI normally.",
                error.to_string().replace(['\r', '\n'], " ")
            );
            false
        }
    };
    let output = env::var_os("GITHUB_OUTPUT").ok_or("GITHUB_OUTPUT is missing.")?;
    writeln!(
        OpenOptions::new().create(true).append(true).open(output)?,
        "skip={skip}"
    )?;
    println!(
        "already-tested: {}",
        if skip {
            "Matching claim; duplicate tests may be skipped."
        } else {
            "No verified claim; run CI normally."
        }
    );
    if let Some(path) = env::var_os("GITHUB_STEP_SUMMARY")
        && let Err(error) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(action_summary(&verdict).as_bytes()))
    {
        eprintln!(
            "already-tested: Could not write job summary: {}",
            error.to_string().replace(['\r', '\n'], " ")
        );
    }
    Ok(())
}

fn read_claim_text(input: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    input.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err("Claim text exceeds the 1 MiB limit.".into());
    }
    Ok(String::from_utf8(bytes)?)
}

fn verify(values: &HashMap<String, String>, files: Vec<String>) -> Result<u8> {
    for name in ["raw", "summary", "command", "environment"] {
        if values.contains_key(name) {
            return Err(format!("--{name} is only valid for claim.").into());
        }
    }
    let workflow = values
        .get("workflow")
        .filter(|value| !value.is_empty())
        .ok_or("--workflow is required.")?;
    let mut options = Options::new(
        env::current_dir()?,
        workflow,
        values.get("check").map_or("*", String::as_str),
    );
    options.files = files;
    let body = if let Some(path) = values.get("body-file") {
        read_claim_text(fs::File::open(path)?)?
    } else {
        read_claim_text(std::io::stdin().lock())?
    };
    let verification = verify_committed_claim(&body, &options)?;
    let (matched_check, reason, message) = match &verification {
        Verification::Matched { check } => (
            Some(check),
            "matched",
            "A canonical claim covers this gate and committed content.",
        ),
        Verification::NoMatch if !body.contains(already_tested::PREFIX) => (
            None,
            "missing_claim",
            "No supported claim was found in the supplied text. Run CI normally.",
        ),
        Verification::NoMatch => (
            None,
            "no_matching_claim",
            "No canonical claim covers this gate with matching committed files, file selection, and workflow. Run CI normally.",
        ),
    };
    let skip = matched_check.is_some();
    if values.contains_key("json") {
        println!(
            "{}",
            serde_json::json!({
                "skip": skip,
                "reason": reason,
                "message": message,
                "workflow": options.workflow,
                "check": options.check,
                "matched_check": matched_check,
            })
        );
    } else {
        println!("already-tested: {message}");
        if let Some(check) = matched_check {
            println!("Matched scope: {check}");
        }
    }
    Ok(u8::from(!skip))
}

fn run(args: &[String]) -> Result<u8> {
    if args == ["action"] {
        action()?;
        return Ok(0);
    }
    let mut values = HashMap::new();
    let mut files = Vec::new();
    let mut positionals = Vec::new();
    let mut index = 0;
    let mut positional_only = false;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" && !positional_only {
            positional_only = true;
        } else if argument.starts_with('-') && argument != "-" && !positional_only {
            let option = argument.strip_prefix("--").ok_or("Unknown option.")?;
            let (name, inline) = option
                .split_once('=')
                .map_or((option, None), |(n, v)| (n, Some(v)));
            match name {
                "raw" | "help" | "version" | "json" => {
                    if inline.is_some() {
                        return Err(format!("--{name} does not take a value.").into());
                    }
                    values.insert(name.to_owned(), String::new());
                }
                "workflow" | "check" | "files" | "summary" | "command" | "environment"
                | "body-file" => {
                    let value = if let Some(value) = inline {
                        value.to_owned()
                    } else {
                        index += 1;
                        let value = args
                            .get(index)
                            .filter(|v| !v.starts_with('-'))
                            .ok_or_else(|| format!("--{name} requires a value."))?;
                        value.clone()
                    };
                    if name == "files" {
                        files.push(value);
                    } else {
                        values.insert(name.to_owned(), value);
                    }
                }
                _ => return Err(format!("Unknown option: {argument}").into()),
            }
        } else {
            positionals.push(argument.as_str());
        }
        index += 1;
    }
    if values.contains_key("version") {
        if args.len() != 1 {
            return Err("--version cannot be combined with other arguments.".into());
        }
        println!("{}", env!("CARGO_PKG_VERSION"));
    } else if args.is_empty() || values.contains_key("help") {
        print!("{HELP}");
    } else if positionals == ["verify"] && args.first().is_some_and(|arg| arg == "verify") {
        return verify(&values, files);
    } else {
        if positionals != ["claim"] {
            return Err(HELP.trim_end().into());
        }
        for name in ["body-file", "json"] {
            if values.contains_key(name) {
                return Err(format!("--{name} is only valid for verify.").into());
            }
        }
        for name in ["summary", "command", "environment"] {
            if let Some(value) = values.get(name) {
                if values.contains_key("raw") {
                    return Err("--raw cannot be combined with display metadata.".into());
                }
                if trim_js(value).is_empty() || value.contains(['\0', '\r', '\n']) {
                    return Err(format!(
                        "--{name} must be a nonempty single-line value without NUL characters."
                    )
                    .into());
                }
            }
        }
        let cwd = env::current_dir()?;
        let mut options = Options::new(
            &cwd,
            values.get("workflow").cloned().unwrap_or_default(),
            values.get("check").cloned().unwrap_or_else(|| "*".into()),
        );
        options.files = files;
        let claim = create_claim(&options)?;
        let output = if values.contains_key("raw") {
            claim
        } else {
            format_claim(&claim, &values, &cwd)?
        };
        println!("{output}");
    }
    Ok(0)
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let verifying = args.first().is_some_and(|arg| arg == "verify");
    let json = verifying
        && args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--json");
    let status = match run(&args) {
        Ok(status) => status,
        Err(error) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "skip": false,
                        "reason": "error",
                        "message": error.to_string(),
                        "matched_check": null,
                    })
                );
            } else {
                eprintln!("already-tested: {error}");
            }
            if verifying { 2 } else { 1 }
        }
    };
    std::process::exit(i32::from(status));
}
