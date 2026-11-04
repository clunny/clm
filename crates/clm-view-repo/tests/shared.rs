//! The crates' sources hold no copy of the remote's name or of the clock.

use std::path::Path;

/// Every non-comment line of the `.rs` files under `dir`, with its path and number.
fn code_lines(dir: &Path, out: &mut Vec<(String, usize, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            code_lines(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for (i, line) in text.lines().enumerate() {
                if !line.trim_start().starts_with("//") {
                    out.push((path.display().to_string(), i + 1, line.to_string()));
                }
            }
        }
    }
}

#[test]
fn scenario_no_copy_of_the_remotes_name_or_the_clock() {
    // req:view.repo/shared-plumbing@c3db052e
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut lines = Vec::new();
    for krate in ["clm-view-repo", "clm-view"] {
        code_lines(&crates.join(krate).join("src"), &mut lines);
    }
    assert!(
        lines.len() > 100,
        "the scan found only {} lines",
        lines.len()
    );
    // Built so that this file's own lines don't hold the whole patterns.
    let quote = '"';
    let forbidden = [
        format!("{quote}origin{quote}"),
        format!("{quote}origin/"),
        format!("remotes/{}", "origin"),
        format!("System{}", "Time"),
        format!("fn {}(", "now"),
    ];
    for (path, number, line) in &lines {
        for pattern in &forbidden {
            assert!(
                !line.contains(pattern.as_str()),
                "{path}:{number} holds {pattern}: {line}"
            );
        }
    }
}
