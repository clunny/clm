//! Fixtures shared by the tests: spec texts, and a chart with its colour removed.

/// A `specd` node with no owners, depending on `deps` (comma-separated), with requirements
/// `reqs`, each `It SHALL.`.
pub fn specd(deps: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// `s` with its ANSI escapes removed.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
