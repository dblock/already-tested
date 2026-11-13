use already_tested::{Options, create_claim, verify_claim};
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const WORKFLOW: &str = ".github/workflows/test.yml";

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git is required for benchmarks");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture(files: usize) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join(WORKFLOW), "name: Test\non: push\n").unwrap();
    for index in 0..files {
        fs::write(
            root.join(format!("src/file-{index}.txt")),
            format!("{index:04}{}", "x".repeat(4092)),
        )
        .unwrap();
    }
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Benchmark"]);
    git(root, &["config", "user.email", "benchmark@example.invalid"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "core.hooksPath=",
            "commit",
            "-qm",
            "Benchmark fixture",
        ],
    );
    directory
}

fn measure(name: &str, samples: usize, mut operation: impl FnMut()) {
    operation();
    let mut times = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        operation();
        times.push(start.elapsed());
    }
    times.sort_unstable();
    let milliseconds = |duration: Duration| duration.as_secs_f64() * 1000.0;
    println!(
        "{name:<38} {:>10.3} ms  [{:.3}, {:.3}]",
        milliseconds(times[samples / 2]),
        milliseconds(times[0]),
        milliseconds(times[samples - 1]),
    );
}

fn main() {
    let smoke = !std::env::args().any(|arg| arg == "--bench");
    let samples = if smoke { 1 } else { 9 };
    println!("Claims benchmark: {samples} samples, median [min, max], one warm-up.");
    for files in if smoke { &[10][..] } else { &[10, 200][..] } {
        let directory = fixture(*files);
        let root = directory.path();
        let named = Options::new(root, WORKFLOW, "test-linux");
        let exact = create_claim(&named).unwrap();
        let mut stale = String::new();
        for index in 0..31 {
            stale.push_str(
                &create_claim(&Options::new(
                    root,
                    WORKFLOW,
                    format!("test-{{linux,{index}}}"),
                ))
                .unwrap(),
            );
            stale.push('\n');
        }
        fs::write(root.join("src/file-0.txt"), "changed source\n").unwrap();
        git(root, &["add", "."]);
        git(
            root,
            &["-c", "core.hooksPath=", "commit", "-qm", "Stale claims"],
        );
        for (case, body, expected) in [
            ("missing", "No claims.", false),
            ("stale-exact", exact.as_str(), false),
            ("stale-31-scopes", stale.as_str(), false),
        ] {
            measure(&format!("{files}-files/{case}"), samples, || {
                assert_eq!(
                    black_box(verify_claim(black_box(Some(body)), black_box(&named)).unwrap()),
                    expected
                );
            });
        }
        // Keep the remaining fixtures on the same committed content as their claims.
        let exact = create_claim(&named).unwrap();
        let glob = create_claim(&Options::new(root, WORKFLOW, "test-*")).unwrap();
        measure(&format!("{files}-files/create"), samples, || {
            black_box(create_claim(black_box(&named)).unwrap());
        });
        for (case, body) in [("exact", exact.as_str()), ("glob", glob.as_str())] {
            measure(&format!("{files}-files/{case}"), samples, || {
                assert!(black_box(
                    verify_claim(black_box(Some(body)), black_box(&named)).unwrap()
                ));
            });
        }
    }
}
