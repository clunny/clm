//! `test-spans`: where the tests of a file are, and the markers outside them.

use std::path::Path;

use clm_coverage::{TestSpan, markers_outside_tests, tests_in};

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";

fn m(id: &str) -> String {
    format!("{REQ}:{id}")
}

fn spans(path: &str, text: &str) -> Vec<(String, bool, usize, usize)> {
    tests_in(Path::new(path), text)
        .into_iter()
        .map(|t: TestSpan| (t.name, t.group, t.first, t.last))
        .collect()
}

fn outside(path: &str, text: &str) -> Vec<(String, usize)> {
    markers_outside_tests(Path::new(path), text)
        .into_iter()
        .map(|m| (m.id, m.line))
        .collect()
}

#[test]
fn scenario_a_rust_test_and_a_helper() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        format!("// {}", m("a/x")),
        "#[test]".to_string(),
        "fn proves_x() {".to_string(),
        format!("    // {}", m("a/y")),
        "}".to_string(),
        String::new(),
        "fn helper() {".to_string(),
        format!("    // {}", m("a/z")),
        "}".to_string(),
    ]
    .join("\n");
    assert_eq!(
        spans("t.rs", &text),
        [("proves_x".to_string(), false, 1, 5)]
    );
    assert_eq!(outside("t.rs", &text), [("a/z".to_string(), 8)]);
}

#[test]
fn rust_attributes_and_nesting() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
#[cfg(test)]
mod tests {
    /// docs
    #[tokio::test(flavor = \"x\")]
    async fn one() {
        if true {
            let _ = 1; // }
        }
    }

    #[should_panic]
    #[wasm_bindgen_test]
    fn two() {}

    #[rstest]
    // between
    fn three() {
    }

    #[test_case(1)]
    fn four() {}

    #[derive(Debug)]
    fn not_a_test() {}

    #[test]
    fn five_test() {}

    #[my_test]
    fn six() {}
}
";
    let found: Vec<_> = spans("t.rs", text)
        .into_iter()
        .map(|(n, _, a, b)| (n, a, b))
        .collect();
    assert_eq!(
        found,
        [
            ("one".to_string(), 3, 9),
            ("two".to_string(), 11, 13),
            ("three".to_string(), 15, 18),
            ("four".to_string(), 20, 21),
            ("five_test".to_string(), 26, 27),
            ("six".to_string(), 29, 30),
        ]
    );
}

#[test]
fn rust_attribute_without_a_function_is_no_test() {
    // req:check.coverage/test-spans@056ed909
    let text = "#[test]\nstruct S;\n";
    assert!(tests_in(Path::new("t.rs"), text).is_empty());
}

#[test]
fn scenario_a_group_above_its_tests() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        format!("// {}", m("a/h")),
        String::new(),
        format!("// {}", m("a/g")),
        "describe('group', () => {".to_string(),
        "  it('works', () => {".to_string(),
        format!("    // {}", m("a/x")),
        "  });".to_string(),
        "});".to_string(),
    ]
    .join("\n");
    assert_eq!(
        spans("a.test.ts", &text),
        [
            ("group".to_string(), true, 3, 8),
            ("works".to_string(), false, 5, 7),
        ]
    );
    assert_eq!(outside("a.test.ts", &text), [("a/h".to_string(), 1)]);
}

#[test]
fn typescript_modifiers_and_extensions() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
test.describe.serial('phone', () => {
  test.only('a', async () => {
    expect(1).toBe(1);
  });
  it.skip(`b`, () => {});
  it.each([1, 2])('c %s', (n) => {
    foo(n);
  });
  test(name, () => {});
  test.describe.configure({ mode: 'serial' });
  xit('no', () => {});
});
";
    let found: Vec<_> = spans("a.spec.tsx", text).into_iter().collect();
    assert_eq!(
        found,
        [
            ("phone".to_string(), true, 1, 12),
            ("a".to_string(), false, 2, 4),
            ("b".to_string(), false, 5, 5),
            (String::new(), false, 6, 8),
            (String::new(), false, 9, 9),
        ]
    );
    for ext in ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"] {
        assert_eq!(
            spans(&format!("x.{ext}"), "it('a', () => {\n});\n"),
            [("a".to_string(), false, 1, 2)],
            "{ext}"
        );
    }
}

#[test]
fn scenario_a_playwright_group() {
    // req:check.coverage/test-spans@056ed909
    let text = format!("// {}\ntest.describe('phone', () => {{\n}});\n", m("a/x"));
    assert!(outside("e.spec.ts", &text).is_empty());
}

#[test]
fn scenario_a_python_test() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "def test_x():".to_string(),
        format!("    # {}", m("a/x")),
        String::new(),
        "    pass".to_string(),
        format!("# {}", m("a/y")),
    ]
    .join("\n");
    assert_eq!(outside("test_a.py", &text), [("a/y".to_string(), 5)]);
    assert_eq!(
        spans("test_a.py", &text),
        [("test_x".to_string(), false, 1, 4)]
    );
}

#[test]
fn python_classes_decorators_and_signatures() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
import pytest

# a comment
@pytest.mark.slow
class TestThing:
    def test_a(self):
        assert 1

    async def test_b(
        self,
        x,
    ):
        assert 2

    def helper(self):
        pass

def test_c(): pass
def other(): pass
class Tesla:
    def helper(self):
        pass
";
    assert_eq!(
        spans("test_a.py", text),
        [
            ("TestThing".to_string(), true, 3, 16),
            ("test_a".to_string(), false, 6, 7),
            ("test_b".to_string(), false, 9, 13),
            ("test_c".to_string(), false, 18, 18),
        ]
    );
}

#[test]
fn scenario_another_language() {
    // req:check.coverage/test-spans@056ed909
    let text = format!("// {}\nfunc TestX(t *testing.T) {{\n}}\n", m("a/x"));
    assert!(tests_in(Path::new("a_test.go"), &text).is_empty());
    assert!(markers_outside_tests(Path::new("a_test.go"), &text).is_empty());
}

#[test]
fn markdown_has_no_tests_and_no_outside_markers() {
    // req:check.coverage/test-spans@056ed909
    let text = format!("{}\n", m("a/x"));
    assert!(tests_in(Path::new("n.md"), &text).is_empty());
    assert!(markers_outside_tests(Path::new("n.md"), &text).is_empty());
}

#[cfg(feature = "checkout")]
#[test]
fn scan_holds_the_outside_markers_beside_req() {
    // req:check.coverage/test-spans@056ed909
    let dir = tempfile::TempDir::new().unwrap();
    let text = format!(
        "#[test]\nfn t() {{\n    // {}\n}}\nfn h() {{\n    // {}\n}}\n",
        m("a/in"),
        m("a/out")
    );
    std::fs::write(dir.path().join("t.rs"), text).unwrap();
    let scan = clm_coverage::scan(dir.path());
    let ids = |v: &[clm_coverage::Marker]| v.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&scan.outside), ["a/out"]);
}

#[test]
fn rust_one_line_attribute_and_array_signature() {
    // req:check.coverage/test-spans@056ed909
    let text = "#[test] fn one() {\n}\n#[test]\nfn two(buf: [u8; 4]) {\n    let _ = buf;\n}\n";
    let found: Vec<_> = spans("t.rs", text)
        .into_iter()
        .map(|(n, _, a, b)| (n, a, b))
        .collect();
    assert_eq!(
        found,
        [("one".to_string(), 1, 2), ("two".to_string(), 3, 6)]
    );
}

fn rust_spans(text: &str) -> Vec<(String, usize, usize)> {
    spans("t.rs", text)
        .into_iter()
        .map(|(n, _, a, b)| (n, a, b))
        .collect()
}

#[test]
fn scenario_braces_in_literals() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "#[test]".to_string(),
        "fn a() {".to_string(),
        "    let at = rest.find('{');".to_string(),
        "    let url = \"https://x/{\";".to_string(),
        "}".to_string(),
        "#[test]".to_string(),
        "fn b() {".to_string(),
        format!("    // {}", m("a/y")),
        "}".to_string(),
    ]
    .join("\n");
    assert_eq!(
        rust_spans(&text),
        [("a".to_string(), 1, 5), ("b".to_string(), 6, 9)]
    );
    // With no test after it to bound it, `a` still closes on line 5: the literals are read.
    let alone = text.lines().take(5).collect::<Vec<_>>().join("\n") + "\nfn helper() {\n}";
    assert_eq!(rust_spans(&alone), [("a".to_string(), 1, 5)]);
}

#[test]
fn rust_prefixed_raw_strings() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
#[test]
fn a() {
    let b = br#\"x\"}\"#;
    let c = cr#\"y\"{\"#;
}
fn helper() {
}
";
    assert_eq!(rust_spans(text), [("a".to_string(), 1, 5)]);
}

#[test]
fn scenario_a_brace_left_open() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "#[test]".to_string(),
        "fn a() {".to_string(),
        "    if x {".to_string(),
        "}".to_string(),
        format!("// {}", m("a/y")),
        "#[test]".to_string(),
        "fn b() {".to_string(),
        "}".to_string(),
    ]
    .join("\n");
    assert_eq!(
        rust_spans(&text),
        [("a".to_string(), 1, 4), ("b".to_string(), 5, 8)]
    );
}

#[test]
fn scenario_a_parenthesis_in_a_string() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "it('a', () => {".to_string(),
        "  f(')');".to_string(),
        "});".to_string(),
        String::new(),
        format!("// {}", m("a/y")),
    ]
    .join("\n");
    assert_eq!(spans("a.test.ts", &text), [("a".to_string(), false, 1, 3)]);
    assert_eq!(outside("a.test.ts", &text), [("a/y".to_string(), 5)]);
}

#[test]
fn rust_test_attribute_inside_a_string_starts_no_test() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
#[test]
fn fixture() {
    let src = \"
#[test]
fn inner() {
\";
    let raw = r#\"
#[test]
fn raw_inner() { \"}\" }
\"#;
    let bytes = br\"{\";
}
fn helper() {
}
";
    assert_eq!(rust_spans(text), [("fixture".to_string(), 1, 12)]);
}

#[test]
fn rust_escapes_lifetimes_and_char_literals() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
#[test]
fn a<'x>(s: &'x str) -> &'x str {
    let q = '\\'';
    let b = '\\\\';
    let close = '}';
    let e = \"\\\"}\";
    'outer: loop { break 'outer; }
    s
}
fn helper() {
}
";
    assert_eq!(rust_spans(text), [("a".to_string(), 1, 9)]);
}

#[test]
fn rust_one_line_tests_in_a_row() {
    // req:check.coverage/test-spans@056ed909
    let text = "#[test] fn one() {}\n#[test] fn two() {}\n";
    assert_eq!(
        rust_spans(text),
        [("one".to_string(), 1, 1), ("two".to_string(), 2, 2)]
    );
}

#[test]
fn script_literals_across_lines() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
it('a', () => {
  const t = `)
  ${x}(`;
  const u = \"https://x/)\"; // )
  const v = 'it\\'s)';
});
it(\"b\", () => {
  f(\"(\");
});
const z = 1;
";
    assert_eq!(
        spans("a.test.js", text),
        [
            ("a".to_string(), false, 1, 6),
            ("b".to_string(), false, 7, 9),
        ]
    );
}

#[test]
fn script_quote_ends_at_its_line() {
    // req:check.coverage/test-spans@056ed909
    // An apostrophe in a regular expression or JSX text costs that line alone.
    let text = "\
it('a', () => {
  const r = /'/;
  f(1);
});
const z = 1;
";
    assert_eq!(spans("a.test.jsx", text), [("a".to_string(), false, 1, 4)]);
}

#[test]
fn rust_comment_inside_a_string_is_no_line_of_the_test_below() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
const S: &str = \"
// in the string\";
#[test]
fn a() {
}
";
    assert_eq!(rust_spans(text), [("a".to_string(), 3, 5)]);
}

#[test]
fn scenario_a_quote_in_a_block_comment() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "/* a 12\" ruler".to_string(),
        "   /* nested */ #[test] fn c() {".to_string(),
        "*/".to_string(),
        "#[test]".to_string(),
        "fn a() {".to_string(),
        "}".to_string(),
        "#[test]".to_string(),
        "fn b() {".to_string(),
        format!("    // {}", m("a/y")),
        "}".to_string(),
    ]
    .join("\n");
    assert_eq!(
        rust_spans(&text),
        [("a".to_string(), 1, 6), ("b".to_string(), 7, 10)]
    );
}

#[test]
fn rust_braces_in_block_comments_are_not_counted() {
    // req:check.coverage/test-spans@056ed909
    // With no test after it to bound it, `a` still closes on line 6: the comments are read.
    let text = "\
#[test]
fn a() {
    /* { /* { */ { */
    let s = \"/*\"; // /*
    /* x */ let c = '{'; /* } */
}
fn helper() {
}
";
    assert_eq!(rust_spans(text), [("a".to_string(), 1, 6)]);
}

#[test]
fn scenario_a_backtick_in_a_block_comment() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "describe('g', () => {",
        "  /* a ` */",
        "  it('a', () => {});",
        "});",
        "it('b', () => {});",
    ]
    .join("\n");
    assert_eq!(
        spans("a.test.ts", &text),
        [
            ("g".to_string(), true, 1, 4),
            ("a".to_string(), false, 2, 3),
            ("b".to_string(), false, 5, 5),
        ]
    );
}

#[test]
fn script_block_comment_across_lines() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
describe('g', () => {
  it('a', () => { /* (
  ) ' */
    f(1);
  });
  /* it's */ const r = '/*';
});
const z = 1;
";
    assert_eq!(
        spans("a.test.js", text),
        [
            ("g".to_string(), true, 1, 7),
            ("a".to_string(), false, 2, 5),
        ]
    );
}

#[test]
fn scenario_a_backtick_the_scanner_does_not_read() {
    // req:check.coverage/test-spans@056ed909
    let text = [
        "it('a', () => {".to_string(),
        "  expect(s).toMatch(/`/);".to_string(),
        "});".to_string(),
        "it('b', () => {".to_string(),
        format!("  // {}", m("a/y")),
        "});".to_string(),
    ]
    .join("\n");
    assert_eq!(
        spans("a.test.ts", &text),
        [
            ("a".to_string(), false, 1, 3),
            ("b".to_string(), false, 4, 6),
        ]
    );
    assert!(outside("a.test.ts", &text).is_empty());
}

#[test]
fn script_group_span_is_not_bounded_by_its_tests() {
    // req:check.coverage/test-spans@056ed909
    // The next call bounds a test only: a group holds its tests. A test misread at the end
    // of the file runs to it.
    let text = "\
describe('g', () => {
  it('a', () => {});
  it('b', () => { f(/`/);
});
";
    assert_eq!(
        spans("a.test.ts", text),
        [
            ("g".to_string(), true, 1, 4),
            ("a".to_string(), false, 2, 2),
            ("b".to_string(), false, 3, 4),
        ]
    );
}

#[test]
fn rust_block_comment_lines_above_a_test() {
    // req:check.coverage/test-spans@056ed909
    // A middle line of a block comment is a comment line; a line that closes one and then
    // holds code is a code line.
    let text = "\
/*
  a 12\" ruler
*/
#[test]
fn a() {}
/* x
*/ const X: u8 = 1;
#[test]
fn b() {}
";
    assert_eq!(
        rust_spans(text),
        [("a".to_string(), 1, 5), ("b".to_string(), 8, 9)]
    );
}

#[test]
fn script_block_comment_lines_above_a_test() {
    // req:check.coverage/test-spans@056ed909
    let text = "\
/*
  a note
*/
it('a', () => {});
/* x
*/ const y = 1;
it('b', () => {});
";
    assert_eq!(
        spans("a.test.ts", text),
        [
            ("a".to_string(), false, 1, 4),
            ("b".to_string(), false, 7, 7),
        ]
    );
}

#[test]
fn script_call_reading_starts_on_its_own_line() {
    // req:check.coverage/test-spans@056ed909
    // `a` leaves a template literal open; `b` is read from its own line and closes there.
    let text = "\
it('a', () => {
  f(/`/);
});
it('b', () => {
});
const z = 1;
";
    assert_eq!(
        spans("a.test.ts", text),
        [
            ("a".to_string(), false, 1, 3),
            ("b".to_string(), false, 4, 5),
        ]
    );
}

#[test]
fn scenario_a_test_in_a_block_comment() {
    // req:check.coverage/test-spans@056ed909
    let text = "/*\nit('a', () => {});\n*/\nit('b', () => {});\n";
    assert_eq!(
        spans("a.test.ts", text),
        [
            ("a".to_string(), false, 1, 2),
            ("b".to_string(), false, 3, 4),
        ]
    );
}
