mod common;

use std::path::Path;

use clm_health::Churn;
use common::{git, repo, report, section, spec, write};

/// A line citing `id` at `version`, built so the literal never appears whole.
fn cite(id: &str, version: &str) -> String {
    format!("// {}:{id}{version}\n", "req")
}

fn commit(root: &Path, message: &str) {
    common::commit_ago(root, message, 0);
}

#[test]
fn scenario_re_cited_across_commits_and_a_rename() {
    // req:check.health/churn-signal@563e439f
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[\"tests/**\"]", &["x", "y"]));
    write(d, "tests/old.rs", &cite("a/x", "@11111111"));
    write(d, "tests/y.rs", &cite("a/y", "@aaaaaaaa"));
    commit(d, "one");
    write(d, "tests/old.rs", &cite("a/x", "@22222222"));
    write(d, "tests/y.rs", &cite("a/y", "@bbbbbbbb"));
    commit(d, "two");
    git(d, &["mv", "tests/old.rs", "tests/new.rs"]);
    write(d, "tests/new.rs", &cite("a/x", "@33333333"));
    commit(d, "three");
    let out = report(d);
    assert_eq!(
        section(&out, "churn"),
        [
            "health/churn: 1 requirements",
            "specs/a/SPEC.md: health/churn: a/x cited at 3 versions",
        ],
        "{out}"
    );
}

#[test]
fn scenario_what_doesnt_count() {
    // req:check.health/churn-signal@563e439f
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[\"tests/**\"]", &["z"]));
    write(d, "tests/z.rs", &cite("a/z", "@cccccccc"));
    commit(d, "c");
    write(d, "tests/z.rs", &cite("a/z", "@dddddddd"));
    write(d, "notes.md", &cite("a/z", "@eeeeeeee"));
    commit(d, "d");
    write(d, "tests/z.rs", &cite("a/z", ""));
    for v in ["@01010101", "@02020202", "@03030303", "@04040404"] {
        write(d, "tests/gone.rs", &cite("a/gone", v));
        commit(d, v);
    }
    write(d, "tests/z.rs", &cite("a/z", "@ffffffff"));
    let out = report(d);
    assert_eq!(
        section(&out, "churn"),
        ["health/churn: 0 requirements"],
        "{out}"
    );
    let churn = clm_health::churn(d).unwrap().unwrap();
    assert_eq!(
        churn,
        [
            Churn {
                id: "a/gone".into(),
                versions: ["01010101", "02020202", "03030303", "04040404"]
                    .map(String::from)
                    .into(),
            },
            Churn {
                id: "a/z".into(),
                versions: vec!["cccccccc".into(), "dddddddd".into()],
            },
        ]
    );
}

#[test]
fn no_commit_is_no_history() {
    // req:check.health/churn-signal@563e439f
    let dir = repo();
    let d = dir.path();
    assert_eq!(clm_health::churn(d).unwrap(), None);
    let out = report(d);
    assert_eq!(
        section(&out, "churn"),
        ["health/churn: no history"],
        "{out}"
    );
}

#[test]
fn a_path_git_quotes_is_skipped() {
    // req:check.health/churn-signal@563e439f
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[\"tests/**\"]", &["x"]));
    write(d, "tests/a\"b.rs", &cite("a/x", "@11111111"));
    write(d, "tests/ok.rs", &cite("a/x", "@22222222"));
    commit(d, "one");
    let churn = clm_health::churn(d).unwrap().unwrap();
    assert_eq!(
        churn,
        [Churn {
            id: "a/x".into(),
            versions: vec!["22222222".into()],
        }]
    );
}

#[test]
fn paths_with_spaces_and_a_users_prefixes() {
    // req:check.health/churn-signal@563e439f
    let dir = repo();
    let d = dir.path();
    // A user's configured prefixes don't change what is read, nor does hiding the
    // root commit's patch.
    git(d, &["config", "diff.dstPrefix", "new/"]);
    git(d, &["config", "log.showRoot", "false"]);
    write(d, "specs/a/SPEC.md", &spec("[\"tests/**\"]", &["x"]));
    write(d, "tests/my test.rs", &cite("a/x", "@11111111"));
    write(d, "docs/my notes.md", &cite("a/x", "@22222222"));
    commit(d, "one");
    let churn = clm_health::churn(d).unwrap().unwrap();
    assert_eq!(
        churn,
        [Churn {
            id: "a/x".into(),
            versions: vec!["11111111".into()],
        }]
    );
}
