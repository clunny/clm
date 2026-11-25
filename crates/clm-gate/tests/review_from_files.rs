//! `review_from_files` (gate/review-from-files): a spec branch's review and cuts read from the
//! files a caller hands the gate, as `evaluate_at` reads them from git.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use clm_gate::{Cut, FileReview, FileSide, Kind, evaluate_at, review_from_files};
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &[u8]) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit(d: &Path, message: &str) {
    git(d, &["add", "-A"]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            message,
        ],
    );
}

/// The architecture, whose one entry `law` component `n` serves, so a graph loaded without
/// it doesn't load.
const ARCH: &str = "# Arch\n\n## Constraints\n\n### Constraint: law\n**Principle.** A law SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n";

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

fn requirement(slug: &str, scenarios: &[&str]) -> String {
    let mut out = format!("\n### Requirement: {slug}\n{slug} SHALL hold.\n");
    for name in scenarios {
        out.push_str(&format!(
            "\n#### Scenario: {name}\n- GIVEN g\n- WHEN w\n- THEN t\n"
        ));
    }
    out
}

/// Component `n` with `frontmatter` lines and `requirements`.
fn component(frontmatter: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\nserves: [architecture/law]\npaths: [\"src/n/**\"]\n{frontmatter}---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n{requirements}"
    )
}

fn r_and_s() -> String {
    format!(
        "{}{}",
        requirement("r", &["first"]),
        requirement("s", &["second"])
    )
}

/// A test file holding the tests `one` and `two`, each citing `n/r`; the marker is built
/// from pieces so this file cites nothing.
fn test_file() -> String {
    let marker = format!("{}:n/r@00000000", "req");
    format!("// {marker}\n#[test]\nfn one() {{}}\n\n// {marker}\n#[test]\nfn two() {{}}\n")
}

/// A repository whose `main` holds the root, component `n` with `r` and `s`, the human
/// layers, `tests/t.rs` and `base` (`None` deletes), with `pr` checked out from it.
fn repo(base: &[(&str, Option<&str>)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT.as_bytes());
    write(d, "specs/n/SPEC.md", component("", &r_and_s()).as_bytes());
    write(d, "ARCHITECTURE.md", ARCH.as_bytes());
    write(d, "CAPABILITIES.md", b"# Capabilities\n");
    write(d, "tests/t.rs", test_file().as_bytes());
    for (rel, text) in base {
        if let Some(text) = text {
            write(d, rel, text.as_bytes());
        }
    }
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

/// Commits `edits` on `pr` (`None` deletes the file).
fn edit(d: &Path, edits: &[(&str, Option<&[u8]>)]) {
    for (rel, content) in edits {
        match content {
            Some(c) => write(d, rel, c),
            None => fs::remove_file(d.join(rel)).unwrap(),
        }
    }
    commit(d, "edit");
}

/// A side of the branch as a caller that fetched its files holds it: the files `rev` holds,
/// each read with `git show`, but those `unreadable` names, whose bytes can't be had.
struct Held {
    revision: String,
    paths: BTreeSet<String>,
}

impl Held {
    fn at(d: &Path, rev: &str) -> Held {
        let revision = git(d, &["rev-parse", rev]);
        let listed = git(d, &["ls-tree", "-r", "-z", "--name-only", &revision]);
        let paths = listed
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        Held { revision, paths }
    }
}

/// `review_from_files` over the branch `pr` against `main` as `git diff --no-renames
/// --name-status` lists it, with the archive moves `moved`, every file readable but
/// `unreadable`, each `(side, path)` with `side` `"base"` or `"head"`.
fn from_files(d: &Path, moved: &[&str], unreadable: &[(&str, &str)]) -> FileReview {
    let merge_base = git(d, &["merge-base", "main", "pr"]);
    let base = Held::at(d, &merge_base);
    let head = Held::at(d, "pr");
    let listed = git(
        d,
        &[
            "diff",
            "--no-renames",
            "--name-status",
            "-z",
            &format!("{merge_base}..pr"),
        ],
    );
    let fields: Vec<&str> = listed.split('\0').collect();
    let paths: Vec<(char, &str)> = fields
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].chars().next().unwrap(), c[1]))
        .collect();
    let toml = |held: &Held| {
        held.paths.contains("clm.toml").then(|| {
            clm_git::run(
                d,
                "git",
                &["show", &format!("{}:clm.toml", held.revision)],
                Default::default(),
            )
            .unwrap()
        })
    };
    let approval = clm_gate::approval_changed(toml(&base).as_deref(), toml(&head).as_deref())
        && paths.iter().any(|(_, p)| *p == "clm.toml");
    let reader = |side: &'static str, held: &Held| {
        let revision = held.revision.clone();
        let unreadable: Vec<String> = unreadable
            .iter()
            .filter(|(s, _)| *s == side)
            .map(|(_, p)| p.to_string())
            .collect();
        move |path: &str| -> Option<Vec<u8>> {
            if unreadable.iter().any(|p| p == path) {
                return None;
            }
            clm_git::run(
                d,
                "git",
                &["show", &format!("{revision}:{path}")],
                Default::default(),
            )
            .ok()
        }
    };
    let (read_base, read_head) = (reader("base", &base), reader("head", &head));
    review_from_files(
        &paths,
        moved,
        approval,
        &FileSide {
            revision: &base.revision,
            paths: &base.paths,
            read: &read_base,
        },
        &FileSide {
            revision: &head.revision,
            paths: &head.paths,
            read: &read_head,
        },
    )
}

/// `clm gate --base main` on `pr`: its standard output.
fn gate(d: &Path) -> String {
    let args: Vec<&OsStr> = ["--base", "main"].iter().map(OsStr::new).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    clm_gate::command(args, d, &|_| None, &mut out, &mut err);
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
    String::from_utf8(out).unwrap()
}

#[test]
fn scenario_the_same_answer_from_files() {
    // req:gate/review-from-files@c46aeacc
    let only_s = component("", &requirement("s", &["second"]));
    let no_scenario = component(
        "",
        &format!("{}\n### Requirement: t\nT SHALL hold.\n", r_and_s()),
    );
    let added = component(
        "",
        &format!("{}{}", r_and_s(), requirement("t", &["third"])),
    );
    let exempt = component(
        "exemptions:\n  - rule: skipped-tests\n    files: [\"src/n/**\"]\n    until: backlog/x\n",
        &r_and_s(),
    );
    let burdened = "---\nstatus: proposed\nhuman_burdens: [a paid run]\n---\n\n## Intent\n\nx\n";
    let plain = "---\nstatus: applied\n---\n\n## Intent\n\nx\n";
    let listed = "DELETE tests/t.rs::one\n";
    let more_arch = format!("{ARCH}\nmore\n");
    type Branch<'a> = (
        &'a str,
        Vec<(&'a str, Option<&'a str>)>,
        Vec<(&'a str, Option<&'a [u8]>)>,
        &'a [&'a str],
        bool,
    );
    let both = format!("{listed}DELETE tests/t.rs::two\n");
    let branches: Vec<Branch> = vec![
        (
            "removes n/r",
            vec![],
            vec![("specs/n/SPEC.md", Some(only_s.as_bytes()))],
            &[],
            true,
        ),
        (
            "head's specs don't load",
            vec![],
            vec![("specs/n/SPEC.md", Some(no_scenario.as_bytes()))],
            &[],
            true,
        ),
        (
            "a broken backlog item beside a new requirement",
            vec![],
            vec![
                ("specs/n/SPEC.md", Some(added.as_bytes())),
                ("backlog/b.md", Some(b"no frontmatter\n")),
            ],
            &[],
            false,
        ),
        (
            "adds an exemption",
            vec![("backlog/x.md", Some("---\ntitle: X\nkind: idea\n---\n"))],
            vec![("specs/n/SPEC.md", Some(exempt.as_bytes()))],
            &[],
            true,
        ),
        (
            "only edits ARCHITECTURE.md",
            vec![],
            vec![("ARCHITECTURE.md", Some(more_arch.as_bytes()))],
            &[],
            true,
        ),
        (
            "adds a change with burdens",
            vec![],
            vec![("changes/x/change.md", Some(burdened.as_bytes()))],
            &[],
            true,
        ),
        (
            "adds a line to an archived list",
            vec![
                ("changes/c/change.md", Some(plain)),
                ("changes/c/deleted-tests.txt", Some(listed)),
            ],
            vec![
                ("changes/c/change.md", None),
                ("changes/c/deleted-tests.txt", None),
                ("changes/archive/c/change.md", Some(plain.as_bytes())),
                ("changes/archive/c/deleted-tests.txt", Some(both.as_bytes())),
            ],
            &["c"],
            true,
        ),
    ];
    for (name, base, edits, moved, review) in branches {
        let dir = repo(&base);
        let d = dir.path();
        edit(d, &edits);
        let outcome = evaluate_at(d, "main", "pr", false).unwrap();
        assert_eq!(outcome.kind, Kind::Spec, "{name}");
        let files = from_files(d, moved, &[]);
        assert_eq!(files.review, outcome.review, "{name}");
        assert_eq!(files.review, review, "{name}");
        assert_eq!(files.cuts, outcome.cuts, "{name}");
        assert!(files.withheld.is_empty(), "{name}: {:?}", files.withheld);
        let lines: Vec<String> = files.cuts.iter().map(ToString::to_string).collect();
        match name {
            "removes n/r" => assert_eq!(
                lines,
                ["specs/n/SPEC.md: gate/cut: requirement n/r is gone"]
            ),
            // The archived list's text at the merge base is the live list's: only `two` is new.
            "adds a line to an archived list" => assert_eq!(
                lines,
                [
                    "changes/archive/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted"
                ]
            ),
            _ => {}
        }
    }
}

#[test]
fn scenario_archive_moves_and_status_edits() {
    // req:gate/review-from-files@c46aeacc
    let burdened = |status: &str| {
        format!("---\nstatus: {status}\nhuman_burdens: [b]\n---\n\n## Intent\n\nx\n")
    };
    let added = component(
        "",
        &format!("{}{}", r_and_s(), requirement("t", &["third"])),
    );

    // Edits `specs/n/SPEC.md` and only the `status` of change `x`, which has burdens.
    let dir = repo(&[("changes/x/change.md", Some(&burdened("proposed")))]);
    let d = dir.path();
    edit(
        d,
        &[
            ("specs/n/SPEC.md", Some(added.as_bytes())),
            ("changes/x/change.md", Some(burdened("applied").as_bytes())),
        ],
    );
    assert_eq!(gate(d).lines().next(), Some("spec review"));
    assert!(from_files(d, &[], &[]).review);

    // Moves change `y`, which has burdens, into the archive and edits a decision.
    let dir = repo(&[
        ("changes/y/change.md", Some(&burdened("applied"))),
        ("docs/decisions/d.md", Some("# D\n")),
    ]);
    let d = dir.path();
    fs::create_dir_all(d.join("changes/archive")).unwrap();
    git(d, &["mv", "changes/y", "changes/archive/y"]);
    write(
        d,
        "changes/archive/y/change.md",
        burdened("done").as_bytes(),
    );
    edit(d, &[("docs/decisions/d.md", Some(b"# D\n\nmore\n"))]);
    assert_eq!(gate(d).lines().next(), Some("spec"));
    assert!(!from_files(d, &["y"], &[]).review);
    assert!(from_files(d, &[], &[]).review);
}

#[test]
fn scenario_what_cant_be_read() {
    // req:gate/review-from-files@c46aeacc
    let list = "changes/c/deleted-tests.txt";
    let spec = "specs/n/SPEC.md";
    let diagram = "specs/n/diagram.png";
    let added = component(
        "",
        &format!("{}{}", r_and_s(), requirement("t", &["third"])),
    );
    let dir = repo(&[]);
    let d = dir.path();
    edit(
        d,
        &[
            (spec, Some(added.as_bytes())),
            (
                "changes/c/change.md",
                Some(b"---\nstatus: applied\n---\n\n## Intent\n\nx\n"),
            ),
            (list, Some(b"# nothing to delete\n")),
            (diagram, Some(&[0x89, b'P', b'N', b'G', 0, 0xff, 0xfe])),
        ],
    );
    let merge_base = git(d, &["merge-base", "main", "pr"]);
    let head = git(d, &["rev-parse", "pr"]);
    let readable = from_files(d, &[], &[]);
    assert!(!readable.review && readable.cuts.is_empty() && readable.withheld.is_empty());

    // The head's list can't be read: it is withheld at the head, and the branch needs review.
    let read = from_files(d, &[], &[("head", list)]);
    assert_eq!(read.withheld, [(list.to_string(), head.clone())]);
    assert!(read.review);
    assert!(
        !read.cuts.iter().any(|c| matches!(c, Cut::Test { .. })),
        "{:?}",
        read.cuts
    );

    // The merge base's spec can't be read: that side doesn't load.
    let read = from_files(d, &[], &[("base", spec)]);
    assert_eq!(read.cuts.len(), 1, "{:?}", read.cuts);
    assert_eq!(
        read.cuts[0].to_string(),
        format!(
            "specs: gate/cut: the specs at {merge_base} don't load, so what the branch cuts isn't listed"
        )
    );
    assert!(read.review);

    // A file under `specs/` the model doesn't read changes nothing when it can't be read.
    let read = from_files(d, &[], &[("head", diagram)]);
    assert_eq!(read.cuts, readable.cuts);
    assert_eq!(read.review, readable.review);
    assert!(read.withheld.is_empty());

    // A live list on `main` gains a line: withheld at the head, its lines aren't among the
    // cuts; withheld at both sides, the merge base is named.
    let dir = repo(&[
        (
            "changes/c/change.md",
            Some("---\nstatus: applied\n---\n\n## Intent\n\nx\n"),
        ),
        (list, Some("DELETE tests/t.rs::one\n")),
    ]);
    let d = dir.path();
    edit(
        d,
        &[
            (spec, Some(added.as_bytes())),
            (
                list,
                Some(b"DELETE tests/t.rs::one\nDELETE tests/t.rs::two\n"),
            ),
        ],
    );
    let merge_base = git(d, &["merge-base", "main", "pr"]);
    let head = git(d, &["rev-parse", "pr"]);
    assert_eq!(
        from_files(d, &[], &[]).cuts,
        [Cut::Test {
            list: list.into(),
            file: "tests/t.rs".into(),
            name: "two".into()
        }]
    );
    let read = from_files(d, &[], &[("head", list)]);
    assert_eq!(read.withheld, [(list.to_string(), head)]);
    assert!(read.cuts.is_empty() && read.review, "{:?}", read.cuts);
    let read = from_files(d, &[], &[("base", list), ("head", list)]);
    assert_eq!(read.withheld, [(list.to_string(), merge_base)]);
    assert!(read.cuts.is_empty() && read.review, "{:?}", read.cuts);

    // A branch that changes only a decision the head holds unreadably.
    let dir = repo(&[("docs/decisions/d.md", Some("# D\n"))]);
    let d = dir.path();
    edit(d, &[("docs/decisions/d.md", Some(b"# D\n\nmore\n"))]);
    let read = from_files(d, &[], &[("head", "docs/decisions/d.md")]);
    assert!(!read.review);
    assert!(read.cuts.is_empty() && read.withheld.is_empty());
}
