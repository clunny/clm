//! `clm triage`'s `answered` lines against a bare `origin` holding web ledger branches.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;

use clm_triage::triage;
use common::*;

/// The repo with node `model` on main and `items` as `(slug, kind)` in `backlog/`.
fn with_items(items: &[(&str, &str)]) -> Repo {
    let r = repo();
    write(&r.work, "specs/model/SPEC.md", &node("", &["z"]));
    for (slug, kind) in items {
        write(
            &r.work,
            &format!("backlog/{slug}.md"),
            &format!("---\ntitle: T\nkind: {kind}\ncomponents: [model]\n---\nBody of {slug}.\n"),
        );
    }
    push_main(&r.work, "items");
    r
}

/// Pushes `records` to `origin` as the ledger branch of `writer` (`alice/web`).
fn push_ledger(r: &Repo, writer: &str, records: &[String]) {
    let file = r.work.join(".git/records.jsonl");
    fs::write(&file, records.join("\n") + "\n").unwrap();
    let blob = git(&r.work, &["hash-object", "-w", file.to_str().unwrap()]);
    let index = r.work.join(".git/ledger-index");
    let env = [("GIT_INDEX_FILE", index.to_str().unwrap())];
    let info = format!("100644,{},records.jsonl", blob.trim());
    git_env(
        &r.work,
        &["update-index", "--add", "--cacheinfo", &info],
        &env,
    );
    let tree = git_env(&r.work, &["write-tree"], &env);
    let commit = git(&r.work, &["commit-tree", tree.trim(), "-m", "ledger"]);
    let refspec = format!("+{}:refs/heads/clm/ledger/{writer}", commit.trim());
    git(&r.work, &["push", "-q", "origin", &refspec]);
}

fn answered(id: &str, writer: &str, item: &str, answer: &str, at: &str) -> String {
    format!(
        r#"{{"id":"{id}","kind":"answered","writer":"{writer}","at":"{at}","item":"{item}","answer":"{answer}"}}"#
    )
}

#[test]
fn scenario_answered_in_the_web_app() {
    // req:orchestrator.triage/triage-web-answers@267e75ce
    let r = with_items(&[("q", "question"), ("h", "human-task"), ("i", "idea")]);
    push_ledger(
        &r,
        "alice/web",
        &[
            answered("a2", "alice/web", "q", "no", "2026-11-12T10:00:00.000Z"),
            answered("a3", "alice/web", "i", "yes", "2026-11-12T08:00:00.000Z"),
        ],
    );
    push_ledger(
        &r,
        "bob/web",
        &[answered(
            "b1",
            "bob/web",
            "q",
            "yes",
            "2026-11-12T09:00:00.000Z",
        )],
    );
    let t = triage(&r.work, 4).unwrap();
    assert!(t.warnings.is_empty(), "{:?}", t.warnings);
    assert_eq!(
        t.lines,
        [
            "human h [model] P5 0",
            "human q [model] P5 0",
            "answered q bob/web 2026-11-12T09:00:00.000Z yes",
            "answered q alice/web 2026-11-12T10:00:00.000Z no",
            "idea i [model] P5 0",
        ]
    );
}

#[test]
fn no_answers_leave_the_lines_unchanged() {
    // req:orchestrator.triage/triage-web-answers@267e75ce
    let r = with_items(&[("q", "question")]);
    assert_eq!(triage(&r.work, 4).unwrap().lines, ["human q [model] P5 0"]);
}

#[test]
fn an_answer_stays_on_one_line() {
    // req:orchestrator.triage/triage-web-answers@267e75ce
    let r = with_items(&[("q", "question")]);
    push_ledger(
        &r,
        "alice/web",
        &[answered(
            "a1",
            "alice/web",
            "q",
            r"line one\nhuman x [y] P1 0",
            "2026-11-12T09:00:00.000Z",
        )],
    );
    assert_eq!(
        triage(&r.work, 4).unwrap().lines,
        [
            "human q [model] P5 0",
            "answered q alice/web 2026-11-12T09:00:00.000Z line one human x [y] P1 0",
        ]
    );
}
