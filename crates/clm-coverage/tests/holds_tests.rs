use std::path::Path;

use clm_coverage::{holds_tests, markers_outside_tests, tests_in};

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";

#[test]
fn scenario_which_paths_can_hold_a_test() {
    // req:check.coverage/test-holding-paths@17606603
    let holds = ["a.rs", "a.test.ts", "x.mjs", "e.spec.tsx", "test_a.py"];
    let holds_none = ["a_test.go", "README.md", "Cargo.lock", "rs", "lib.RS"];
    for path in holds {
        assert!(holds_tests(Path::new(path)), "{path} can hold a test");
    }
    for path in holds_none {
        assert!(!holds_tests(Path::new(path)), "{path} holds no test");
    }
}

#[test]
fn a_path_that_holds_no_test_yields_nothing_whatever_its_text() {
    // req:check.coverage/test-holding-paths@17606603
    let text = format!(
        "#[test]\nfn proves() {{\n    // {REQ}:a/x@11111111\n}}\ndef test_x():\n    # {REQ}:a/y\n"
    );
    for path in ["a_test.go", "README.md", "Cargo.lock", "rs", "lib.RS"] {
        let path = Path::new(path);
        assert!(!holds_tests(path));
        assert!(tests_in(path, &text).is_empty());
        assert!(markers_outside_tests(path, &text).is_empty());
    }
    // The same text in a path that holds tests yields a test and a marker outside it.
    let rust = Path::new("a.rs");
    assert!(holds_tests(rust));
    assert!(!tests_in(rust, &text).is_empty());
}
