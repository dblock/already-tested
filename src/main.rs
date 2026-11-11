use already_tested::{Options, create_claim, head_commit, trim_js, valid_commit, verify_claim};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const HELP: &str = "Usage: already-tested claim --workflow <path> --check <identifier> [--files <pattern> ...]

Run tests against committed source, then paste the generated expandable section into the PR description.

Options:
  --workflow <path>      Tracked .github/workflows/*.yml or *.yaml file
  --check <identifier>   Distinct identifier for the job or matrix environment
  --files <pattern>      Git path pattern; repeat to select files (default: all tracked files)
  --summary <text>       Disclosure label (default: <check> - passed)
  --command <text>       Command already validated; recorded, never executed
  --environment <text>   Environment actually validated; recorded, never inferred
  --raw                 Print only the canonical hidden claim, without display metadata
  --help                Show this help
  --version             Show the package version
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
    let default = format!("{check} - passed");
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

fn action_verdict() -> Result<bool> {
    if env::var("GITHUB_EVENT_NAME").as_deref() != Ok("pull_request") {
        return Ok(false);
    }
    let event: serde_json::Value = serde_json::from_slice(&fs::read(
        env::var_os("GITHUB_EVENT_PATH").ok_or("GITHUB_EVENT_PATH is missing.")?,
    )?)?;
    let source = event
        .pointer("/pull_request/head/sha")
        .and_then(|v| v.as_str())
        .filter(|s| valid_commit(s))
        .ok_or("The PR head commit ID is missing or invalid.")?;
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
    let mut options = Options::new(
        env::var_os("GITHUB_WORKSPACE")
            .map(std::path::PathBuf::from)
            .unwrap_or(env::current_dir()?),
        workflow.as_str(),
        env::var("INPUT_CHECK").unwrap_or_default(),
    );
    options.r#ref = source.into();
    options.workflow_ref = Some(workflow_ref);
    options.files = env::var("INPUT_FILES")
        .unwrap_or_default()
        .split('\n')
        .map(trim_js)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(verify_claim(
        event.pointer("/pull_request/body").and_then(|b| b.as_str()),
        &options,
    )?)
}

fn action() -> Result<()> {
    let skip = match action_verdict() {
        Ok(skip) => skip,
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
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["action"] {
        return action();
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
                "raw" | "help" | "version" => {
                    if inline.is_some() {
                        return Err(format!("--{name} does not take a value.").into());
                    }
                    values.insert(name.to_owned(), String::new());
                }
                "workflow" | "check" | "files" | "summary" | "command" | "environment" => {
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
    } else {
        if positionals != ["claim"] {
            return Err(HELP.trim_end().into());
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
            values.get("check").cloned().unwrap_or_default(),
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
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("already-tested: {error}");
        std::process::exit(1);
    }
}
