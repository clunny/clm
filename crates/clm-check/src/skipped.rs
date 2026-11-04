//! `skipped-tests`: no skipped test in a file that cites requirements (see
//! specs/check/SPEC.md).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clm_model::Violation;

use crate::Marker;

/// Each line that skips a test, in each file holding one of `markers`.
pub fn violations(root: &Path, markers: &[Marker]) -> Vec<Violation> {
    let files: BTreeSet<&Path> = markers.iter().map(|m| m.path.as_path()).collect();
    let mut out = Vec::new();
    for file in files {
        let Some(skips) = rule_for(file) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(root.join(file)) else {
            continue;
        };
        out.extend(
            text.lines()
                .enumerate()
                .filter(|(_, line)| skips(line))
                .map(|(i, _)| Violation {
                    path: PathBuf::from(format!("{}:{}", file.display(), i + 1)),
                    rule: "check/skipped-tests",
                    message: "a skipped test in a file that cites requirements; run it, or move it to a file that cites none".into(),
                }),
        );
    }
    out
}

/// How a file's lines skip a test, by its extension; none for a language it doesn't read.
fn rule_for(file: &Path) -> Option<fn(&str) -> bool> {
    match file.extension()?.to_str()? {
        "rs" => Some(|line| line.trim_start().starts_with("#[ignore")),
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => Some(script_skips),
        "py" => Some(|line| {
            let t = line.trim_start();
            t.starts_with("@pytest.mark.skip")
                || t.starts_with("@unittest.skip")
                || line.contains("pytest.skip(")
        }),
        _ => None,
    }
}

/// `(it|test|describe)\.(skip|skipIf|todo)\b` or `\bx(it|test|describe)\(`, not preceded by
/// a letter, digit, `_` or `.`.
fn script_skips(line: &str) -> bool {
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let bytes = line.as_bytes();
    let ends_word = |r: &[u8]| r.first().is_none_or(|&c| !word(c));
    (0..bytes.len()).any(|i| {
        if i > 0 && (word(bytes[i - 1]) || bytes[i - 1] == b'.') {
            return false;
        }
        let rest = &bytes[i..];
        [&b"it"[..], b"test", b"describe"].iter().any(|f| {
            let method = rest
                .strip_prefix(*f)
                .and_then(|r| r.strip_prefix(b"."))
                .is_some_and(|r| {
                    [&b"skipIf"[..], b"skip", b"todo"]
                        .iter()
                        .any(|m| r.strip_prefix(*m).is_some_and(ends_word))
                });
            let x = rest
                .strip_prefix(b"x")
                .and_then(|r| r.strip_prefix(*f))
                .is_some_and(|r| r.starts_with(b"("));
            method || x
        })
    })
}
