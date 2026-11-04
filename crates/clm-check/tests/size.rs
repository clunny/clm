use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_check::{Budgets, Size, size};
use tempfile::TempDir;

const DEFAULTS: Budgets = Budgets {
    spec_tokens: 1000,
    code_tokens: 1000,
};

fn write(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// A node's `SPEC.md` of exactly `bytes` bytes, with `extra` frontmatter.
fn spec(paths: &str, extra: &str, bytes: usize) -> Vec<u8> {
    let head = format!(
        "---\ntitle: A\nstatus: active\nowners: []\npaths: {paths}\n{extra}---\n## Contract\n"
    );
    let mut text = head.into_bytes();
    assert!(text.len() < bytes);
    text.resize(bytes - 1, b'x');
    text.push(b'\n');
    text
}

/// A git repo with an empty root node and node `a` (`paths: ["src/a/**"]`, a `SPEC.md` of
/// 4001 bytes with `extra` frontmatter); `files` are written and added to the index.
fn repo(extra: &str, files: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    write(
        root,
        "specs/SPEC.md",
        b"---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        &spec("[\"src/a/**\"]", extra, 4001),
    );
    for (rel, content) in files {
        write(root, rel, content);
        git(root, &["add", rel]);
    }
    dir
}

/// Every node's size, with the markers `clm check` would find.
fn sizes(root: &Path) -> Vec<Size> {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    size(root, &graph, &clm_check::scan(root).req, DEFAULTS).unwrap()
}

/// The line for `node`.
fn line(root: &Path, node: &str) -> String {
    let sizes = sizes(root);
    sizes
        .iter()
        .find(|s| s.node == node)
        .unwrap_or_else(|| panic!("no {node} in {sizes:?}"))
        .to_string()
}

#[test]
fn over_its_spec_budget() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/x.rs", &[b'a'; 400])]);
    assert_eq!(
        line(dir.path(), "a"),
        "a\tspec 1001/1000\tcode 100/1000\toversized"
    );
}

#[test]
fn files_that_dont_count() {
    // req:check/node-size@7fb9fb0c
    let mut logo = b"\x89PNG\0".to_vec();
    logo.resize(4000, b'p');
    let dir = repo(
        "",
        &[
            ("src/a/x.rs", &[b'a'; 400]),
            (
                ".gitattributes",
                b"src/a/gen.rs linguist-generated\nsrc/a/vendor/** linguist-vendored=true\n",
            ),
            ("src/a/gen.rs", &[b'g'; 4000]),
            ("src/a/vendor/z.js", &[b'v'; 4000]),
            ("src/a/logo.png", &logo),
            ("src/a/deleted.rs", &[b'd'; 4000]),
        ],
    );
    write(dir.path(), "src/a/y.rs", &[b'u'; 4000]);
    fs::remove_file(dir.path().join("src/a/deleted.rs")).unwrap();
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
}

#[test]
fn rounded_once() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/p.rs", b"p"), ("src/a/q.rs", b"qqq")]);
    assert!(line(dir.path(), "a").contains("\tcode 1/1000"));
}

#[test]
fn attribute_set_to_false() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "",
        &[
            (
                ".gitattributes",
                b"src/a/x.rs linguist-generated=false\nsrc/a/y.rs linguist-vendored=maybe\n",
            ),
            ("src/a/x.rs", &[b'a'; 400]),
            ("src/a/y.rs", &[b'b'; 400]),
        ],
    );
    assert!(line(dir.path(), "a").contains("\tcode 200/1000"));
}

#[test]
fn budget_raised_in_the_spec() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "size_budget: { spec_tokens: 2000 }\n",
        &[("src/a/x.rs", &[b'a'; 400])],
    );
    assert_eq!(line(dir.path(), "a"), "a\tspec 1001/2000\tcode 100/1000");
}

#[test]
fn code_budget_raised_in_the_spec() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "size_budget: { code_tokens: 50 }\n",
        &[("src/a/x.rs", &[b'a'; 400])],
    );
    assert_eq!(
        line(dir.path(), "a"),
        "a\tspec 1001/1000\tcode 100/50\toversized"
    );
}

#[test]
fn exactly_at_budget_is_not_over() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "size_budget: { spec_tokens: 1001, code_tokens: 100 }\n",
        &[("src/a/x.rs", &[b'a'; 400])],
    );
    assert_eq!(line(dir.path(), "a"), "a\tspec 1001/1001\tcode 100/100");
}

#[test]
fn every_node_sorted_by_id() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[]);
    write(dir.path(), "specs/b/SPEC.md", &spec("[]", "", 200));
    let nodes: Vec<_> = sizes(dir.path()).into_iter().map(|s| s.node).collect();
    assert_eq!(nodes, ["a", "b", "clm"]);
}

#[test]
fn a_node_without_paths_has_no_code() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/x.rs", &[b'a'; 400])]);
    let bytes = fs::metadata(dir.path().join("specs/SPEC.md"))
        .unwrap()
        .len();
    assert_eq!(
        line(dir.path(), "clm"),
        format!("clm\tspec {}/1000\tcode 0/1000", bytes.div_ceil(4))
    );
}

#[test]
fn a_nul_after_the_first_8000_bytes_still_counts() {
    // req:check/node-size@7fb9fb0c
    let mut text = vec![b't'; 8000];
    text.extend(b"\0tail");
    let dir = repo("", &[("src/a/late.txt", &text)]);
    assert!(line(dir.path(), "a").contains("\tcode 2002/1000"));
}

#[test]
fn bare_vendored_attribute_excludes() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "",
        &[
            (".gitattributes", b"src/a/vendor/** linguist-vendored\n"),
            ("src/a/x.rs", &[b'a'; 400]),
            ("src/a/vendor/z.js", &[b'v'; 4000]),
        ],
    );
    assert!(line(dir.path(), "a").contains("\tcode 100/1000"));
}

#[test]
fn a_conflicted_file_counts_once() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/x.rs", &[b'a'; 400])]);
    let root = dir.path();
    // Replace stage 0 with the three stages of a merge conflict on the same file.
    let blob = Command::new("git")
        .args(["hash-object", "-w", "src/a/x.rs"])
        .current_dir(root)
        .output()
        .unwrap();
    let blob = String::from_utf8(blob.stdout).unwrap();
    let blob = blob.trim();
    let info = format!(
        "0 {zero}\tsrc/a/x.rs\n100644 {blob} 1\tsrc/a/x.rs\n100644 {blob} 2\tsrc/a/x.rs\n100644 {blob} 3\tsrc/a/x.rs\n",
        zero = "0".repeat(40)
    );
    let mut child = Command::new("git")
        .args(["update-index", "--index-info"])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(child.stdin.as_mut().unwrap(), info.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success());
    assert!(
        line(root, "a").contains("\tcode 100/1000"),
        "{}",
        line(root, "a")
    );
}

#[cfg(unix)]
#[test]
fn symlinks_and_directories_dont_count() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/x.rs", &[b'a'; 400]), ("big", &[b'b'; 4000])]);
    let root = dir.path();
    std::os::unix::fs::symlink("../../big", root.join("src/a/link")).unwrap();
    git(root, &["add", "src/a/link"]);
    // A tracked symlink later replaced by a directory, as a submodule's checkout is.
    std::os::unix::fs::symlink("../../big", root.join("src/a/sub")).unwrap();
    git(root, &["add", "src/a/sub"]);
    fs::remove_file(root.join("src/a/sub")).unwrap();
    fs::create_dir(root.join("src/a/sub")).unwrap();
    assert!(
        line(root, "a").contains("\tcode 100/1000"),
        "{}",
        line(root, "a")
    );
}

#[cfg(unix)]
#[test]
fn file_names_that_arent_utf8_count() {
    // req:check/node-size@7fb9fb0c
    use std::os::unix::ffi::OsStrExt;
    let dir = repo("", &[]);
    let root = dir.path();
    let name = std::ffi::OsStr::from_bytes(b"src/a/caf\xe9.rs");
    fs::create_dir_all(root.join("src/a")).unwrap();
    fs::write(root.join(name), [b'a'; 400]).unwrap();
    let status = Command::new("git")
        .arg("add")
        .arg(name)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        line(root, "a").contains("\tcode 100/1000"),
        "{}",
        line(root, "a")
    );
}

#[test]
fn file_names_with_spaces_count() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("", &[("src/a/my file.rs", &[b'a'; 400])]);
    assert!(line(dir.path(), "a").contains("\tcode 100/1000"));
}

/// A file of `bytes` bytes whose first line is `first`.
fn sized(first: &str, bytes: usize) -> Vec<u8> {
    let mut text = format!("{first}\n").into_bytes();
    assert!(text.len() < bytes);
    text.resize(bytes, b'x');
    text
}

/// `repo` plus node `b` (`paths: ["src/b/**"]`). Markers name nodes, not requirements, so
/// the requirements they cite needn't exist.
fn two_nodes(files: &[(&str, &[u8])]) -> TempDir {
    let dir = repo("", files);
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &spec("[\"src/b/**\"]", "", 200),
    );
    dir
}

const CITES_B: &str = concat!("// req", ":b/x");
const CITES_A_AND_B: &str = concat!("// req", ":a/y req", ":b/x");

#[test]
fn a_test_counts_where_it_is_cited() {
    // req:check/node-size@7fb9fb0c
    let test = sized(CITES_B, 800);
    let dir = two_nodes(&[("src/a/tests/b.rs", &test), ("src/a/lib.rs", &[b'a'; 400])]);
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 200/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn a_test_citing_two_nodes_counts_for_both() {
    // req:check/node-size@7fb9fb0c
    let test = sized(CITES_A_AND_B, 400);
    let dir = two_nodes(&[("tests/both.rs", &test), ("src/a/lib.rs", &[b'a'; 400])]);
    assert!(
        line(dir.path(), "a").contains("\tcode 200/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn budget_outgrown() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(
        "size_budget: { spec_tokens: 2000 }\n",
        &[("src/a/x.rs", &[b'a'; 400])],
    );
    let path = dir.path().join("specs/a/SPEC.md");
    let mut text = fs::read(&path).unwrap();
    text.truncate(4000);
    text.resize(8000, b'x');
    text.push(b'\n');
    fs::write(&path, text).unwrap();
    assert_eq!(
        line(dir.path(), "a"),
        "a\tspec 2001/2000\tcode 100/1000\toversized"
    );
}

#[test]
fn markers_naming_no_node_count_nowhere() {
    // req:check/node-size@7fb9fb0c
    let test = sized(concat!("// req", ":zz/x"), 800);
    let dir = two_nodes(&[("src/a/t.rs", &test), ("src/a/lib.rs", &[b'a'; 400])]);
    let sizes = sizes(dir.path());
    let code: Vec<_> = sizes
        .iter()
        .map(|s| (s.node.as_str(), s.code.tokens))
        .collect();
    assert_eq!(code, [("a", 100), ("b", 0), ("clm", 0)]);
}

#[test]
fn a_removed_requirement_still_counts() {
    // req:check/node-size@7fb9fb0c
    // `b` has no requirement `gone`: the marker still names node `b`.
    let test = sized(concat!("// req", ":b/gone"), 800);
    let dir = two_nodes(&[("src/a/t.rs", &test)]);
    assert!(
        line(dir.path(), "b").contains("\tcode 200/1000"),
        "{}",
        line(dir.path(), "b")
    );
    assert!(
        line(dir.path(), "a").contains("\tcode 0/1000"),
        "{}",
        line(dir.path(), "a")
    );
}

#[test]
fn markdown_counts_by_paths() {
    // req:check/node-size@7fb9fb0c
    let doc = sized(CITES_B, 400);
    let dir = two_nodes(&[("src/a/notes.md", &doc)]);
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 0/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn an_excluded_file_with_markers_counts_nowhere() {
    // req:check/node-size@7fb9fb0c
    let generated = sized(CITES_B, 4000);
    let dir = two_nodes(&[
        (".gitattributes", b"src/a/gen.rs linguist-generated\n"),
        ("src/a/gen.rs", &generated),
    ]);
    assert!(
        line(dir.path(), "a").contains("\tcode 0/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 0/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn untracked_files_with_markers_dont_count() {
    // req:check/node-size@7fb9fb0c
    let dir = two_nodes(&[]);
    write(dir.path(), "src/a/t.rs", &sized(CITES_B, 800));
    assert!(
        line(dir.path(), "b").contains("\tcode 0/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

/// 400 bytes of source, then a test part of 800 bytes starting `#[cfg(test)]` whose next
/// line is `marker`.
fn with_inline_tests(marker: &str) -> Vec<u8> {
    let mut text = vec![b's'; 399];
    text.push(b'\n');
    let mut tests = format!("#[cfg(test)]\n{marker}\n").into_bytes();
    tests.resize(800, b't');
    text.extend(tests);
    text
}

#[test]
fn inline_tests_count_where_they_are_cited() {
    // req:check/node-size@7fb9fb0c
    let dir = two_nodes(&[("src/a/w.rs", &with_inline_tests(CITES_B))]);
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 200/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn inline_tests_without_markers() {
    // req:check/node-size@7fb9fb0c
    let mut text = b"fn a() {}\n    #[cfg(test)]\nmod tests {}\n".to_vec();
    text.resize(400, b'x');
    let dir = two_nodes(&[("src/a/v.rs", &text)]);
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
}

#[test]
fn only_an_exact_cfg_test_line_splits_a_rust_file() {
    // req:check/node-size@7fb9fb0c
    // Not exactly `#[cfg(test)]`: one part, holding a marker, so `b`'s only.
    let mut rust = format!("#[cfg(test)] mod t;\n{CITES_B}\n").into_bytes();
    rust.resize(400, b'r');
    // Not Rust: one part holding a marker, so `b`'s only.
    let mut other = format!("#[cfg(test)]\n{CITES_B}\n").into_bytes();
    other.resize(800, b'p');
    let dir = two_nodes(&[
        ("src/a/u.rs", &rust),
        ("src/a/u.py", &other),
        ("src/a/lib.rs", &[b'a'; 400]),
    ]);
    assert!(
        line(dir.path(), "a").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 300/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn a_marked_source_part_and_an_unmarked_test_part() {
    // req:check/node-size@7fb9fb0c
    let mut text = sized(CITES_B, 399);
    text.push(b'\n');
    let mut tests = b"#[cfg(test)]\n".to_vec();
    tests.resize(800, b't');
    text.extend(tests);
    let dir = two_nodes(&[("src/a/w.rs", &text)]);
    assert!(
        line(dir.path(), "a").contains("\tcode 200/1000"),
        "{}",
        line(dir.path(), "a")
    );
    assert!(
        line(dir.path(), "b").contains("\tcode 100/1000"),
        "{}",
        line(dir.path(), "b")
    );
}

#[test]
fn size_of_a_listing_matches_the_working_tree() {
    // req:check/size-of-listing@38e607b5
    let mut logo = b"\x89PNG\0".to_vec();
    logo.resize(4000, b'p');
    let dir = repo(
        "size_budget: { spec_tokens: 1000 }\n",
        &[
            ("src/a/x.rs", &[b'a'; 400]),
            ("src/a/logo.png", &logo),
            (
                "tests/t.rs",
                &sized(concat!("// req", ":a/x@00000000"), 800),
            ),
            (".gitattributes", b"Cargo.lock linguist-generated\n"),
            ("Cargo.lock", &[b'l'; 4000]),
            ("src/b/y.rs", &[b'b'; 100]),
        ],
    );
    let root = dir.path();
    write(root, "specs/b/SPEC.md", &spec("[\"src/b/**\"]", "", 200));
    git(root, &["add", "specs"]);
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    let markers = clm_check::scan(root).req;
    let files: BTreeMap<PathBuf, clm_check::Length> = [
        "specs/SPEC.md",
        "specs/a/SPEC.md",
        "specs/b/SPEC.md",
        "src/a/x.rs",
        "tests/t.rs",
        ".gitattributes",
        "src/b/y.rs",
    ]
    .into_iter()
    .map(|f| (f.into(), fs::metadata(root.join(f)).unwrap().len().into()))
    .collect();
    let listed = clm_check::size_of(&graph, &files, &markers, DEFAULTS);
    assert_eq!(listed, size(root, &graph, &markers, DEFAULTS).unwrap());
    let a = listed.iter().find(|s| s.node == "a").unwrap();
    assert!(a.oversized());
    assert_eq!((a.spec.tokens, a.spec.budget), (1001, 1000));
    assert_eq!(a.code.tokens, 300);
    assert!(!listed.iter().find(|s| s.node == "b").unwrap().oversized());
}

#[test]
fn size_of_a_listing_with_inline_tests_matches_the_working_tree() {
    // req:check/size-of-listing@38e607b5
    let text = with_inline_tests(CITES_B);
    let dir = two_nodes(&[("src/a/w.rs", &text)]);
    let root = dir.path();
    git(root, &["add", "specs"]);
    let (graph, _) = clm_model::load(root);
    let markers = clm_check::scan(root).req;
    let mut files: BTreeMap<PathBuf, clm_check::Length> =
        ["specs/SPEC.md", "specs/a/SPEC.md", "specs/b/SPEC.md"]
            .into_iter()
            .map(|f| (f.into(), fs::metadata(root.join(f)).unwrap().len().into()))
            .collect();
    let w = Path::new("src/a/w.rs");
    files.insert(w.into(), clm_check::Length::of(w, &text));
    let listed = clm_check::size_of(&graph, &files, &markers, DEFAULTS);
    assert_eq!(listed, size(root, &graph, &markers, DEFAULTS).unwrap());
    let code: Vec<_> = listed
        .iter()
        .map(|s| (s.node.as_str(), s.code.tokens))
        .collect();
    assert_eq!(code, [("a", 100), ("b", 200), ("clm", 0)]);
}
