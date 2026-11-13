use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

pub fn title_case(value: &str) -> bool {
    let mut plain = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("${{") {
        plain.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("}}") else {
            return false;
        };
        plain.push('X');
        rest = &rest[start + end + 2..];
    }
    plain.push_str(rest);
    let words: Vec<_> = plain.split_whitespace().collect();
    !words.is_empty()
        && words.iter().enumerate().all(|(index, word)| {
            let word = word.trim_matches([':', '(', ')', '[', ']', '"']);
            if word.starts_with('`')
                || matches!(word, "already-tested" | "already-tested-js" | "macOS")
            {
                return true;
            }
            let minor = matches!(
                word.to_ascii_lowercase().as_str(),
                "a" | "an"
                    | "and"
                    | "as"
                    | "at"
                    | "but"
                    | "by"
                    | "for"
                    | "from"
                    | "in"
                    | "into"
                    | "nor"
                    | "of"
                    | "on"
                    | "or"
                    | "per"
                    | "the"
                    | "to"
                    | "via"
                    | "with"
            );
            if minor && index > 0 && index < words.len() - 1 {
                word.chars().all(|c| !c.is_uppercase())
            } else {
                word.chars()
                    .next()
                    .is_some_and(|c| !c.is_alphabetic() || c.is_uppercase())
            }
        })
}

pub fn markdown(text: &str) -> Result<(), String> {
    let mut fence = false;
    let mut paragraph = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fence = !fence;
            paragraph = false;
            continue;
        }
        if fence {
            continue;
        }
        if line.ends_with([' ', '\t']) {
            return Err(format!("line {}: trailing whitespace", index + 1));
        }
        if let Some(heading) = line.strip_prefix('#') {
            let heading = heading.trim_start_matches('#').trim();
            if !title_case(heading) || heading.ends_with(['.', ':', '?', '!']) {
                return Err(format!(
                    "line {}: use title-case headings without trailing punctuation",
                    index + 1
                ));
            }
        }
        let reference = line.starts_with('[') && line.contains("]:");
        let prose = !reference
            && !line.is_empty()
            && !line.starts_with(['#', '-', '*', '>', '|', '<', ' '])
            && !line.chars().next().is_some_and(|c| c.is_ascii_digit());
        if prose && paragraph {
            return Err(format!(
                "line {}: keep prose paragraphs on one source line",
                index + 1
            ));
        }
        paragraph = prose;
    }
    if fence {
        return Err("unclosed code fence".into());
    }
    Ok(())
}

pub fn yaml_names(text: &str) -> Result<(), String> {
    let documents = YamlLoader::load_from_str(text).map_err(|error| error.to_string())?;
    for document in documents {
        check_name(&document)?;
        if let Some(jobs) = document["jobs"].as_hash() {
            for (_, job) in jobs {
                check_name(job)?;
                if let Some(steps) = job["steps"].as_vec() {
                    for step in steps {
                        check_name(step)?;
                    }
                }
            }
        }
        if let Some(steps) = document["runs"]["steps"].as_vec() {
            for step in steps {
                check_name(step)?;
            }
        }
    }
    Ok(())
}

fn check_name(mapping: &Yaml) -> Result<(), String> {
    if mapping["name"].is_badvalue() {
        return Ok(());
    }
    match mapping["name"].as_str() {
        Some(name) if title_case(name) => Ok(()),
        _ => {
            Err("workflow, job, and step display names must be nonempty title-case strings".into())
        }
    }
}

pub fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut directories = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_dir()
            {
                if directory != root || path == root.join("docs") {
                    directories.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn headings(text: &str) -> HashSet<String> {
    text.lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .to_lowercase()
                .replace(' ', "-")
        })
        .collect()
}

pub fn documents(root: &Path) -> Result<(), String> {
    for path in markdown_files(root)? {
        let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        markdown(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut rest = text.as_str();
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find(')') else { break };
            let target = &rest[..end];
            if !target.contains("://") {
                let (file, anchor) = target.split_once('#').unwrap_or((target, ""));
                let linked = if file.is_empty() {
                    path.clone()
                } else {
                    path.parent().unwrap().join(file)
                };
                if !linked.exists() {
                    return Err(format!("{}: missing link {target}", path.display()));
                }
                if !anchor.is_empty() {
                    let linked_text =
                        fs::read_to_string(&linked).map_err(|error| error.to_string())?;
                    if !headings(&linked_text).contains(anchor) {
                        return Err(format!("{}: missing anchor {target}", path.display()));
                    }
                }
            }
            rest = &rest[end + 1..];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_headings_paragraphs_and_code() {
        assert!(markdown("# Working on already-tested\n\nOne paragraph.\n\n## CLI and Claim Format\n\n```sh\nanything\nwrapped\n```\n").is_ok());
        assert!(markdown("## [Unreleased]\n\n[Unreleased]: https://example.com/compare/v0.1.0...master\n[0.1.0]: https://example.com/releases/v0.1.0\n").is_ok());
        assert!(markdown("# Bad heading\n").is_err());
        assert!(markdown("# Bad Heading:\n").is_err());
        assert!(markdown("Wrapped\nparagraph.\n").is_err());
        assert!(markdown("```sh\nunclosed\n").is_err());
    }

    #[test]
    fn checks_only_workflow_display_names() {
        assert!(yaml_names("name: Test\njobs:\n  test:\n    name: Test on ${{ matrix.entry.os }}\n    steps:\n      - name: Run Tests\n        run: cargo test\n      - with: {name: not a display name}\n").is_ok());
        assert!(yaml_names("name: bad name\n").is_err());
        assert!(yaml_names("jobs:\n  x:\n    steps:\n      - name: 7\n").is_err());
        assert!(yaml_names("jobs: [broken\n").is_err());
        assert!(yaml_names("name: Test\nname: Lint\n").is_err());
    }

    #[test]
    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes are confined to this test's unique temporary directory."
        )
    )]
    fn checks_nested_docs_and_relative_file_anchors_without_scanning_builds() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("docs/nested")).unwrap();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target/bad.md"), "# bad heading\n").unwrap();
        fs::write(
            root.join("README.md"),
            "# Project\n\n[Examples](docs/nested/guide.md#examples)\n",
        )
        .unwrap();
        fs::write(
            root.join("docs/README.md"),
            "# Documentation\n\n[Project](../README.md#project)\n",
        )
        .unwrap();
        let guide = root.join("docs/nested/guide.md");
        fs::write(
            &guide,
            "# Guide\n\n## Examples\n\n[Documentation](../README.md)\n",
        )
        .unwrap();
        assert!(documents(root).is_ok());
        fs::write(root.join("README.md"), "# Project\n").unwrap();
        fs::write(&guide, "# Guide\n\n[Missing](../missing.md)\n").unwrap();
        assert!(documents(root).unwrap_err().contains("missing link"));
        fs::write(&guide, "# bad heading\n").unwrap();
        assert!(documents(root).unwrap_err().contains("title-case"));
        fs::write(&guide, "# Guide\n\n[Missing](../../README.md#absent)\n").unwrap();
        assert!(documents(root).unwrap_err().contains("missing anchor"));
    }
}
