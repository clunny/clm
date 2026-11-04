//! A request for changes is decided through `clm-ledger`, with no copy of its rules
//! (land.approval/changes-requested-holds).

#[test]
fn decided_through_ledger_with_no_copy() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let krate = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = krate.join("src");
    // Built from parts, so this file never holds the spellings it looks for: a function
    // named for either rule, or the person rule written inline.
    let copies = [
        concat!("fn ", "person("),
        concat!("fn ", "cleared("),
        concat!("split(", "'/')"),
    ];
    let mut files = rust_files(&src);
    files.extend(rust_files(&krate.join("../clm-land/src")));
    assert!(files.len() > 1, "no source files read");
    let mut found = Vec::new();
    for path in files {
        for (n, line) in code_lines(&path) {
            if copies.iter().any(|c| line.contains(c)) {
                found.push(format!("{}:{}: {}", path.display(), n, line));
            }
        }
    }
    assert!(found.is_empty(), "a copy of ledger's rule: {found:#?}");
    let hold: Vec<String> = code_lines(&src.join("hold.rs"))
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    assert!(
        hold.iter().any(|l| l.contains(".cleared(")),
        "hold.rs doesn't ask ledger's cleared"
    );
    assert!(
        !hold.iter().any(|l| l.contains(".approvals")),
        "hold.rs reads the approvals itself"
    );
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

/// A file's lines that aren't comments, trimmed, with their line numbers.
fn code_lines(path: &std::path::Path) -> Vec<(usize, String)> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .enumerate()
        .map(|(n, l)| (n + 1, l.trim_start().to_string()))
        .filter(|(_, l)| !l.starts_with("//"))
        .collect()
}
