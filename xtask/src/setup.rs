use crate::{Result, run};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};
use tempfile::tempdir;

const VERSION: &str = "6.4.0";
const TOOLS: [&str; 2] = ["cargo-dylint", "dylint-link"];

fn install(target: &str, base: &str, bin: &Path) -> Result<()> {
    let temporary = tempdir()?;
    let directory = temporary.path();
    let mut files = Vec::new();
    for tool in TOOLS {
        let name = format!("{tool}-{target}-v{VERSION}");
        let archive = format!("{name}.tar.gz");
        for file in [&archive, &format!("{archive}.sha256")] {
            run(
                "curl",
                &["-fsSL", &format!("{base}/{file}"), "-o", file],
                directory,
            )?;
        }
        let checksum = fs::read_to_string(directory.join(format!("{archive}.sha256")))?;
        let fields: Vec<_> = checksum.split_whitespace().collect();
        let digest: String = Sha256::digest(fs::read(directory.join(&archive))?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if fields != [format!("SHA2-256({archive})="), digest] {
            return Err(format!("Invalid Dylint checksum for {archive}").into());
        }
        let binary = if target.ends_with("windows-msvc") {
            format!("{tool}.exe")
        } else {
            tool.to_owned()
        };
        run(
            "tar",
            &[
                "-xzf",
                &archive,
                "--strip-components=1",
                &format!("{name}/{binary}"),
            ],
            directory,
        )?;
        let source = directory.join(&binary);
        if !fs::symlink_metadata(&source)?.is_file() {
            return Err(format!("Dylint archive did not contain a regular {binary}").into());
        }
        files.push((source, bin.join(binary)));
    }
    fs::create_dir_all(bin)?;
    for (source, destination) in files {
        fs::copy(source, &destination)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(destination, fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

pub fn dylint() -> Result<()> {
    let target = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return Err("No prebuilt Dylint tools for this host; use cargo install".into()),
    };
    let home = env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".cargo")))
        .ok_or("CARGO_HOME or HOME is required")?;
    install(
        target,
        &format!("https://github.com/trailofbits/dylint/releases/download/v{VERSION}"),
        &home.join("bin"),
    )?;
    println!("Installed checksum-verified Dylint {VERSION} tools.");
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "The installer uses only private temporary files and local archive URLs."
        )
    )]
    fn installs_verified_archives_and_rejects_bad_checksums_before_replacement() -> Result<()> {
        let temporary = tempdir()?;
        let root = temporary.path();
        let target = "x86_64-unknown-linux-gnu";
        let bin = root.join("bin");
        for tool in TOOLS {
            let name = format!("{tool}-{target}-v{VERSION}");
            fs::create_dir(root.join(&name))?;
            fs::write(root.join(&name).join(tool), format!("verified {tool}\n"))?;
            let archive = format!("{name}.tar.gz");
            run("tar", &["-czf", &archive, &format!("{name}/{tool}")], root)?;
            let digest: String = Sha256::digest(fs::read(root.join(&archive))?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            fs::write(
                root.join(format!("{archive}.sha256")),
                format!("SHA2-256({archive})= {digest}\n"),
            )?;
        }
        let base = format!("file://{}", root.display());
        install(target, &base, &bin)?;
        for tool in TOOLS {
            assert_eq!(
                fs::read_to_string(bin.join(tool))?,
                format!("verified {tool}\n")
            );
            fs::write(bin.join(tool), "keep existing executable\n")?;
        }
        let archive = format!("dylint-link-{target}-v{VERSION}.tar.gz");
        fs::write(
            root.join(format!("{archive}.sha256")),
            format!("SHA2-256({archive})= {}\n", "0".repeat(64)),
        )?;
        assert!(install(target, &base, &bin).is_err());
        for tool in TOOLS {
            assert_eq!(
                fs::read_to_string(bin.join(tool))?,
                "keep existing executable\n"
            );
        }
        Ok(())
    }
}
