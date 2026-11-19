//! `test-lists`: the markers outside tests by what they cite, the tests a change's removals
//! leave with nothing to cite, and the citing tests one tree holds that another doesn't.

mod common;

use std::path::PathBuf;

use clm_coverage::{
    DeletedTest, bare_name, deleted_tests, markers_outside_tests, obsolete_tests, uncited_tests,
    uncounted,
};
use common::{ROOT, change, graph, spec, write};
use tempfile::TempDir;

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";

fn files(list: &[(&str, String)]) -> Vec<(PathBuf, String)> {
    list.iter()
        .map(|(p, t)| (PathBuf::from(p), t.clone()))
        .collect()
}

fn repo(removed: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &spec("[\"crates/a/**\"]", &["x", "y"]),
    );
    change(d, "20260101-c", "applied", &[], removed);
    dir
}

#[test]
fn scenario_uncounted() {
    // req:check.coverage/test-lists@1f461b0a
    let dir = repo(&["gone"]);
    let g = graph(dir.path());
    let text = format!(
        "fn a() {{}}\nfn b() {{}}\n// {REQ}:a/x\nfn c() {{}}\nfn d() {{}}\nfn e() {{}}\nfn f() {{}}\nfn g() {{}}\n// {REQ}:a/gone@11111111\n// {REQ}:nobody/here\n"
    );
    let outside = markers_outside_tests(std::path::Path::new("src/lib.rs"), &text);
    assert_eq!(outside.len(), 3);
    let found = uncounted(&g, &outside);
    let shown: Vec<_> = found
        .iter()
        .map(|u| {
            (
                u.id.as_str(),
                u.markers
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("a/gone", vec!["src/lib.rs:9".to_string()]),
            ("a/x", vec!["src/lib.rs:3".to_string()]),
        ]
    );
}

#[test]
fn uncounted_lists_each_marker_of_an_id() {
    // req:check.coverage/test-lists@1f461b0a
    let dir = repo(&[]);
    let g = graph(dir.path());
    let text = format!("// {REQ}:a/y\n// {REQ}:a/y\n");
    let mut outside = markers_outside_tests(std::path::Path::new("b.rs"), &text);
    outside.extend(markers_outside_tests(std::path::Path::new("a.rs"), &text));
    let found = uncounted(&g, &outside);
    assert_eq!(found.len(), 1);
    let at: Vec<_> = found[0].markers.iter().map(ToString::to_string).collect();
    assert_eq!(at, ["a.rs:1", "a.rs:2", "b.rs:1", "b.rs:2"]);
}

const TESTS: &str = "tests/t.rs";

fn two_tests() -> String {
    format!(
        "#[test]\nfn both() {{\n    // {REQ}:a/x\n    // {REQ}:a/y\n}}\n\n#[test]\nfn only_x() {{\n    // {REQ}:a/x@11111111\n}}\n"
    )
}

#[test]
fn scenario_a_removed_requirements_tests() {
    // req:check.coverage/test-lists@1f461b0a
    let dir = repo(&["x"]);
    // `a/x` is still held by component `a`, so only a removal the spec no longer holds
    // counts: remove it from the component's spec.
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &spec("[\"crates/a/**\"]", &["y"]),
    );
    let g = graph(dir.path());
    let found = obsolete_tests(&g, &files(&[(TESTS, two_tests())]));
    let shown: Vec<_> = found
        .iter()
        .map(|t| (t.test.as_str(), t.line, t.ids.clone(), t.every))
        .collect();
    assert_eq!(
        shown,
        [
            ("tests/t.rs::both", 1, vec!["a/x".to_string()], false),
            ("tests/t.rs::only_x", 7, vec!["a/x".to_string()], true),
        ]
    );
}

#[test]
fn obsolete_tests_ignore_groups_and_files_without_removals() {
    // req:check.coverage/test-lists@1f461b0a
    let dir = repo(&["x"]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &spec("[\"crates/a/**\"]", &["y"]),
    );
    let g = graph(dir.path());
    let group = format!("describe('g', () => {{\n  // {REQ}:a/x\n}});\n");
    let kept = format!("#[test]\nfn keeps() {{\n    // {REQ}:a/y\n}}\n");
    let found = obsolete_tests(&g, &files(&[("a.test.ts", group), ("k.rs", kept)]));
    assert!(found.is_empty(), "{found:?}");
}

fn base() -> String {
    format!(
        "#[test]\nfn one() {{\n    // {REQ}:a/x\n}}\n#[test]\nfn two() {{\n    // {REQ}:a/x\n}}\n#[test]\nfn quiet() {{}}\n"
    )
}

#[test]
fn scenario_a_deleted_test() {
    // req:check.coverage/test-lists@1f461b0a
    let head = format!("#[test]\nfn one() {{\n    // {REQ}:a/x\n}}\n");
    let found = deleted_tests(&files(&[(TESTS, base())]), &files(&[(TESTS, head)]));
    let shown: Vec<_> = found
        .iter()
        .map(|t| (t.test.as_str(), t.ids.clone()))
        .collect();
    assert_eq!(shown, [("tests/t.rs::two", vec!["a/x".to_string()])]);
}

#[test]
fn scenario_a_renamed_test() {
    // req:check.coverage/test-lists@1f461b0a
    let head = format!(
        "#[test]\nfn one() {{\n    // {REQ}:a/x\n}}\n#[test]\nfn second() {{\n    // {REQ}:a/x\n}}\n"
    );
    let found = deleted_tests(&files(&[(TESTS, base())]), &files(&[(TESTS, head)]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn scenario_a_renamed_test_that_cites_less() {
    // req:check.coverage/test-lists@1f461b0a
    let base = format!(
        "#[test]\nfn one() {{\n    // {REQ}:a/x\n}}\n#[test]\nfn two() {{\n    // {REQ}:a/x\n    // {REQ}:a/y\n}}\n"
    );
    let head = rust_test("one", true) + &rust_test("second", true);
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    let shown: Vec<_> = found
        .iter()
        .map(|t| (t.test.as_str(), t.ids.clone()))
        .collect();
    assert_eq!(
        shown,
        [(
            "tests/t.rs::two",
            vec!["a/x".to_string(), "a/y".to_string()]
        )]
    );
}

#[test]
fn scenario_a_test_renamed_into_another_file() {
    // req:check.coverage/test-lists@1f461b0a
    let head = files(&[
        (TESTS, rust_test("one", true)),
        ("tests/u.rs", rust_test("second", true)),
    ]);
    let found = deleted_tests(&files(&[(TESTS, base())]), &head);
    assert_eq!(names(&found), ["tests/t.rs::two"]);
}

#[test]
fn scenario_one_new_name_for_two_gone_tests() {
    // req:check.coverage/test-lists@1f461b0a
    let base = rust_test("one", true) + &rust_test("two", true) + &rust_test("three", true);
    let head = rust_test("one", true) + &rust_test("second", true);
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    assert_eq!(names(&found), ["tests/t.rs::three"]);
}

#[test]
fn a_rename_may_cite_more_and_at_other_versions() {
    // req:check.coverage/test-lists@1f461b0a
    let base = format!("#[test]\nfn two() {{\n    // {REQ}:a/x@11111111\n}}\n");
    let head =
        format!("#[test]\nfn second() {{\n    // {REQ}:a/y\n    // {REQ}:a/x@22222222\n}}\n");
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_rename_takes_only_a_name_no_base_test_carries() {
    // req:check.coverage/test-lists@1f461b0a
    // `tests/t.rs` loses `two` and gains `kept`, but `kept` is a name `tests/u.rs` held
    // at the base: it is a move, not a rename, so `two` is gone. A head test with no
    // marker isn't a rename either.
    let base = files(&[
        (TESTS, rust_test("two", true)),
        ("tests/u.rs", rust_test("kept", true)),
    ]);
    let head = files(&[(TESTS, rust_test("kept", true) + &rust_test("fresh", false))]);
    assert_eq!(names(&deleted_tests(&base, &head)), ["tests/t.rs::two"]);
    // A base test of that name with no marker makes it an old name too, in another file
    // as in its own.
    let base = files(&[
        (TESTS, rust_test("two", true)),
        ("tests/u.rs", rust_test("second", false)),
    ]);
    let head = files(&[(TESTS, rust_test("second", true))]);
    assert_eq!(names(&deleted_tests(&base, &head)), ["tests/t.rs::two"]);
}

#[test]
fn a_group_name_at_the_base_leaves_a_rename_fresh() {
    // req:check.coverage/test-lists@1f461b0a
    // A `describe` group named `second` at the base isn't a test: a head test `second` is
    // still a new name, so `two` was renamed.
    let base =
        format!("test('two', () => {{\n  // {REQ}:a/x\n}});\ndescribe('second', () => {{\n}});\n");
    let head = format!("test('second', () => {{\n  // {REQ}:a/x\n}});\n");
    let found = deleted_tests(
        &files(&[("a.test.ts", base)]),
        &files(&[("a.test.ts", head)]),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn scenario_a_test_that_gains_its_first_marker() {
    // req:check.coverage/test-lists@1f461b0a
    let head = rust_test("one", true) + &rust_test("quiet", true);
    let found = deleted_tests(&files(&[(TESTS, base())]), &files(&[(TESTS, head)]));
    let shown: Vec<_> = found
        .iter()
        .map(|t| (t.test.as_str(), t.ids.clone()))
        .collect();
    assert_eq!(shown, [("tests/t.rs::two", vec!["a/x".to_string()])]);
}

#[test]
fn scenario_two_renames_that_each_keep_their_citations() {
    // req:check.coverage/test-lists@1f461b0a
    let base = format!(
        "{}#[test]\nfn second() {{\n    // {REQ}:a/x\n    // {REQ}:a/y\n}}\n",
        rust_test("first", true)
    );
    let head = format!(
        "#[test]\nfn wide() {{\n    // {REQ}:a/x\n    // {REQ}:a/y\n}}\n{}",
        rust_test("narrow", true)
    );
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn renames_pair_as_many_as_can_be_and_the_earlier_wins_a_tie() {
    // req:check.coverage/test-lists@1f461b0a
    // One head test fits both taken tests: the earlier counts as renamed.
    let base = rust_test("one", true) + &rust_test("two", true);
    let head = rust_test("fresh", true);
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    assert_eq!(names(&found), ["tests/t.rs::two"]);
    // A taken test that fits nothing holds up none after it, and a chain of moves frees a
    // head test for a later one: `a` fits `p` and `q`, `b` fits `q` alone, `c` `p` alone.
    let cite = |name: &str, ids: &[&str]| {
        let lines: String = ids
            .iter()
            .map(|i| format!("    // {REQ}:a/{i}\n"))
            .collect();
        format!("#[test]\nfn {name}() {{\n{lines}}}\n")
    };
    let base = cite("a", &["x"]) + &cite("lost", &["z"]) + &cite("b", &["y"]) + &cite("c", &["w"]);
    let head = cite("p", &["x", "w"]) + &cite("q", &["x", "y"]);
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head.clone())]));
    // `a` takes `p`, `b` takes `q`, then `c` needs `p`: `a` can't move to `q` without
    // leaving `b` unpaired, so `c` is gone, with `lost`.
    assert_eq!(names(&found), ["tests/t.rs::lost", "tests/t.rs::c"]);
    // With `b` gone from the base, `a` moves to `q` and `c` takes `p`.
    let base = cite("a", &["x"]) + &cite("c", &["w"]);
    let found = deleted_tests(&files(&[(TESTS, base)]), &files(&[(TESTS, head)]));
    assert!(found.is_empty(), "{found:?}");
}

fn names(found: &[DeletedTest]) -> Vec<&str> {
    found.iter().map(|t| t.test.as_str()).collect()
}

fn rust_test(name: &str, marker: bool) -> String {
    let cite = if marker {
        format!("    // {REQ}:a/x\n")
    } else {
        String::new()
    };
    format!("#[test]\nfn {name}() {{\n{cite}}}\n")
}

#[test]
fn scenario_a_moved_test() {
    // req:check.coverage/test-lists@1f461b0a
    let head = files(&[
        (TESTS, rust_test("one", true)),
        ("tests/u.rs", rust_test("two", true)),
    ]);
    let found = deleted_tests(&files(&[(TESTS, base())]), &head);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn scenario_a_name_two_files_share() {
    // req:check.coverage/test-lists@1f461b0a
    let both = files(&[
        (TESTS, rust_test("two", true)),
        ("tests/u.rs", rust_test("two", true)),
    ]);
    let head = files(&[
        (TESTS, String::new()),
        ("tests/u.rs", rust_test("two", true)),
    ]);
    assert_eq!(names(&deleted_tests(&both, &head)), ["tests/t.rs::two"]);
    // Whichever file loses it is the one listed.
    let head = files(&[(TESTS, rust_test("two", true))]);
    assert_eq!(names(&deleted_tests(&both, &head)), ["tests/u.rs::two"]);
}

#[test]
fn scenario_a_test_that_loses_its_marker() {
    // req:check.coverage/test-lists@1f461b0a
    let head = rust_test("one", true) + &rust_test("two", false);
    let found = deleted_tests(&files(&[(TESTS, base())]), &files(&[(TESTS, head)]));
    assert_eq!(names(&found), ["tests/t.rs::two"]);
}

#[test]
fn scenario_a_title_that_carries_its_marker() {
    // req:check.coverage/test-lists@1f461b0a
    let base = format!("test('keeps the list ({REQ}:a/x@11111111)', () => {{\n}});\n");
    let head = format!("test('keeps the list ({REQ}:a/z@22222222)', () => {{\n}});\n");
    let found = deleted_tests(
        &files(&[("a.test.ts", base)]),
        &files(&[("a.test.ts", head)]),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn scenario_one_of_two_tests_of_a_name_in_one_file() {
    // req:check.coverage/test-lists@1f461b0a
    let one = format!("test('two', () => {{\n  // {REQ}:a/x\n}});\n");
    let base = format!("{one}{one}");
    let found = deleted_tests(
        &files(&[("a.test.ts", base)]),
        &files(&[("a.test.ts", one)]),
    );
    let shown: Vec<_> = found.iter().map(|t| (t.test.as_str(), t.line)).collect();
    // The last of the two is the one taken.
    assert_eq!(shown, [("a.test.ts::two", 4)]);
}

#[test]
fn deleted_tests_notice_a_missing_file_and_a_moved_test() {
    // req:check.coverage/test-lists@1f461b0a
    let found = deleted_tests(&files(&[(TESTS, base())]), &[]);
    assert_eq!(names(&found), ["tests/t.rs::one", "tests/t.rs::two"]);
    assert_eq!(found[0].line, 1);
    assert_eq!(found[1].line, 5);
    // The file is gone but `one` moved: only `two` is gone.
    let head = files(&[("tests/other.rs", rust_test("one", true))]);
    let found = deleted_tests(&files(&[(TESTS, base())]), &head);
    assert_eq!(names(&found), ["tests/t.rs::two"]);
}

#[test]
fn deleted_tests_sort_by_file_then_line() {
    // req:check.coverage/test-lists@1f461b0a
    // Names sort against their lines (`b` above `a`), and the files' names against theirs.
    let t = rust_test("zz", true) + &rust_test("aa", true);
    let base = files(&[("tests/b.rs", t.clone()), ("tests/a.rs", t)]);
    let shown: Vec<_> = deleted_tests(&base, &[])
        .iter()
        .map(|t| (t.test.clone(), t.line))
        .collect();
    let want = [
        ("tests/a.rs::zz", 1),
        ("tests/a.rs::aa", 5),
        ("tests/b.rs::zz", 1),
        ("tests/b.rs::aa", 5),
    ];
    assert_eq!(
        shown,
        want.map(|(n, l)| (n.to_string(), l)),
        "deleted tests out of order"
    );
}

#[test]
fn deleted_tests_take_no_more_than_are_gone() {
    // req:check.coverage/test-lists@1f461b0a
    // `a.rs` loses both its tests `two`, `b.rs` gains one: one is gone, the last of `a.rs`.
    let two = rust_test("two", true);
    let base = files(&[("tests/a.rs", two.clone() + &two)]);
    let head = files(&[("tests/a.rs", String::new()), ("tests/b.rs", two)]);
    let shown: Vec<_> = deleted_tests(&base, &head)
        .iter()
        .map(|t| (t.test.clone(), t.line))
        .collect();
    assert_eq!(shown, [("tests/a.rs::two".to_string(), 5)]);
}

#[test]
fn uncited_tests_leave_groups_out_and_sort() {
    // req:check.coverage/test-lists@1f461b0a
    let ts = "describe('g', () => {\n  test('t', () => {\n  });\n});\n".to_string();
    let rust = rust_test("zz", false) + &rust_test("aa", false);
    let found = uncited_tests(&files(&[("tests/t.rs", rust), ("a.test.ts", ts)]));
    let shown: Vec<_> = found.iter().map(|t| (t.test.as_str(), t.line)).collect();
    assert_eq!(
        shown,
        [
            ("a.test.ts::t", 2),
            ("tests/t.rs::zz", 1),
            ("tests/t.rs::aa", 4)
        ]
    );
}

#[test]
fn deleted_tests_take_the_files_in_sorted_order() {
    // req:check.coverage/test-lists@1f461b0a
    // Three tests `two` at the base, one at the head in a file that had none: two are
    // gone, taken from the files in sorted order, each giving what it lost.
    let base = files(&[
        ("tests/c.rs", rust_test("two", true)),
        ("tests/b.rs", rust_test("two", true)),
        ("tests/a.rs", rust_test("two", true)),
    ]);
    let head = files(&[("tests/d.rs", rust_test("two", true))]);
    assert_eq!(
        names(&deleted_tests(&base, &head)),
        ["tests/a.rs::two", "tests/b.rs::two"]
    );
}

#[test]
fn bare_name_leaves_markers_out() {
    // req:check.coverage/test-lists@1f461b0a
    assert_eq!(
        bare_name(&format!(
            "keeps the list {REQ}:a/x@11111111 and {REQ}:b.c/y"
        )),
        "keeps the list and"
    );
    assert_eq!(
        bare_name(&format!("keeps ({REQ}:a/x@11111111)")),
        "keeps ()"
    );
    assert_eq!(bare_name("plain_name"), "plain_name");
    // Markers that overlap leave their text out once, without panicking.
    assert_eq!(bare_name(&format!("t {REQ}:a/{REQ}:b/c end")), "t end");
}

#[test]
fn scenario_uncited_tests() {
    // req:check.coverage/test-lists@1f461b0a
    let rust = format!(
        "#[test]\nfn one() {{\n    // {REQ}:a/x\n}}\n\n\n\n\n#[test]\nfn helper_rounds_up() {{\n}}\n"
    );
    let ts =
        format!("describe('g', () => {{\n  test('t', () => {{\n    // {REQ}:a/y\n  }});\n}});\n");
    let found = uncited_tests(&files(&[("a.test.ts", ts), (TESTS, rust)]));
    let shown: Vec<_> = found.iter().map(|t| (t.test.as_str(), t.line)).collect();
    assert_eq!(shown, [("tests/t.rs::helper_rounds_up", 9)]);
}
