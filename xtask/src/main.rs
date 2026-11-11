mod checks;
mod package;

use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run(program: &str, args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new(program).args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        return Err(format!(
            "{program} {}: {}\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn documents(root: &Path) -> Result<()> {
    checks::documents(root)?;
    for entry in fs::read_dir(root.join(".github/workflows"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "yml") {
            checks::yaml_names(&fs::read_to_string(&path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    checks::yaml_names(&fs::read_to_string(root.join("action.yml"))?)?;
    if !cfg!(windows) {
        run("sh", &["-n", "scripts/install.sh"], root)?;
        run("sh", &["-n", "scripts/action.sh"], root)?;
    }
    Ok(())
}

fn execute() -> Result<()> {
    #[cfg_attr(
        dylint_lib = "general",
        allow(
            abs_home_path,
            reason = "This private maintenance command intentionally runs from its source checkout."
        )
    )]
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing root")?;
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("lint") if args.len() == 1 => {
            run("cargo", &["fmt", "--all", "--", "--check"], root)?;
            run(
                "cargo",
                &[
                    "clippy",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ],
                root,
            )?;
            documents(root)?;
            let dylint = Command::new("cargo")
                .args([
                    "dylint",
                    "--all",
                    "--fail-on-no-libraries",
                    "--",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                ])
                .env(
                    "DYLINT_RUSTFLAGS",
                    format!(
                        "{} -Dwarnings",
                        env::var("DYLINT_RUSTFLAGS").unwrap_or_default()
                    ),
                )
                .current_dir(root)
                .status()?;
            if !dylint.success() {
                return Err("Dylint checks failed".into());
            }
            println!("Rust, Markdown, spelling, YAML display names, and shell syntax passed.");
        }
        Some("docs") if args.len() == 1 => {
            documents(root)?;
            println!("Markdown, YAML display names, and shell syntax passed.");
        }
        Some("package") => {
            let target = match args.as_slice() {
                [_] => None,
                [_, flag, target] if flag == "--target" => Some(target.as_str()),
                _ => return Err("Usage: cargo xtask package [--target TARGET]".into()),
            };
            package::build(root, target)?;
        }
        _ => return Err("Usage: cargo xtask lint | docs | package [--target TARGET]".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}
