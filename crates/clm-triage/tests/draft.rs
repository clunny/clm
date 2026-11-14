//! `clm_triage::draft` with a fixed clock: the change ID it chooses and the place it holds.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;

use clm_orchestrator::{PlaceKind, Placed, places, take_place};
use clm_triage::{Clock, DraftError, draft};
use common::*;

/// 2026-10-27T23:30:00Z: the 28th in Tokyo.
const NOW: u64 = 1_793_143_800;

fn clock(tz: &str) -> Clock {
    Clock {
        now: NOW,
        tz: Some(tz.into()),
    }
}

/// The repo with a `bug` item per slug in `origin/main`'s backlog.
fn with(slugs: &[&str]) -> Repo {
    let r = repo();
    for slug in slugs {
        write(
            &r.work,
            &format!("backlog/{slug}.md"),
            "---\ntitle: T\nkind: bug\n---\nBody.\n",
        );
    }
    push_main(&r.work, "items");
    r
}

/// The ID `draft` chose for `slugs`, with up to 3 places, in UTC.
fn id(r: &Repo, slugs: &[&str]) -> String {
    let slugs: Vec<String> = slugs.iter().map(|s| s.to_string()).collect();
    draft(&r.work, &slugs, None, None, 3, 4, &clock("UTC"))
        .unwrap()
        .drafted
        .expect("drafted")
        .id
}

fn place_names(r: &Repo) -> Vec<String> {
    places(&r.work, 4)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect()
}

fn place_file(r: &Repo, name: &str) -> String {
    fs::read_to_string(r.work.join(".git/clm/builds").join(name)).unwrap()
}

#[test]
fn the_date_is_the_given_clock_in_the_given_time_zone() {
    // req:orchestrator.triage/draft@66686cf9
    let r = with(&["a"]);
    assert_eq!(id(&r, &["a"]), "20261027-a");
    let r = with(&["a"]);
    let slugs = ["a".to_string()];
    let tokyo = draft(&r.work, &slugs, None, None, 3, 4, &clock("Asia/Tokyo")).unwrap();
    let drafted = tokyo.drafted.unwrap();
    assert_eq!(drafted.id, "20261028-a");
    let prompt = fs::read_to_string(&drafted.file).unwrap();
    assert!(
        prompt.contains("git switch -c spec/20261028-a origin/main"),
        "{prompt}"
    );
    assert!(
        drafted
            .launcher
            .contains(&drafted.file.display().to_string())
    );
}

#[test]
fn scenario_id_taken() {
    // req:orchestrator.triage/draft@66686cf9
    let r = with(&["long-item-slug-with-many-words"]);
    let dir = "changes/archive/20261027-long-item-slug-with";
    write(&r.work, &format!("{dir}/change.md"), &change("applied"));
    push_main(&r.work, "archived");
    assert_eq!(
        id(&r, &["long-item-slug-with-many-words"]),
        "20261027-long-item-slug-with-2"
    );
}

#[test]
fn a_change_on_main_holds_its_id() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["q"]);
    write(&r.work, "changes/20261027-q/change.md", &change("proposed"));
    push_main(&r.work, "proposed");
    assert_eq!(id(&r, &["q"]), "20261027-q-2");
}

#[test]
fn branches_on_origin_hold_their_ids() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["q"]);
    push_branch(&r.work, "spec/20261027-q", 0);
    push_branch(&r.work, "autosync/spec/20261027-q-2", 0);
    push_branch(&r.work, "propose/20261027-q-3", 0);
    assert_eq!(id(&r, &["q"]), "20261027-q-4");
}

#[test]
fn a_local_spec_branch_holds_its_id() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["q"]);
    git(&r.work, &["branch", "spec/20261027-q", "origin/main"]);
    assert_eq!(id(&r, &["q"]), "20261027-q-2");
}

#[test]
fn the_first_claimed_slug_names_the_change() {
    // req:orchestrator.triage/draft@66686cf9
    let r = with(&["a", "b"]);
    push_propose(&r.work, "a", 1);
    assert_eq!(id(&r, &["a", "b"]), "20261027-b");
}

#[test]
fn scenario_place_named_by_the_change() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["a", "b"]);
    let id = id(&r, &["a", "b"]);
    assert_eq!(place_names(&r), std::slice::from_ref(&id));
    let file = place_file(&r, &id);
    let lines: Vec<&str> = file.lines().collect();
    for line in [
        "kind draft",
        &format!("branch spec/{id}"),
        "branch propose/a",
        "branch propose/b",
    ] {
        assert!(lines.contains(&line), "{line}: {file}");
    }
    assert_eq!(lines.iter().filter(|l| l.starts_with("branch ")).count(), 3);
}

#[test]
fn scenario_nothing_claimed_frees_the_place() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["b"]);
    push_propose(&r.work, "b", 1);
    let d = draft(&r.work, &["b".to_string()], None, None, 3, 4, &clock("UTC")).unwrap();
    assert!(d.drafted.is_none());
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}

#[test]
fn scenario_id_held_by_a_place() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["q"]);
    let placed = take_place(&r.work, "20261027-q", PlaceKind::Draft, &[], 3, 4).unwrap();
    assert_eq!(placed, Placed::New);
    let before = place_file(&r, "20261027-q");
    assert_eq!(id(&r, &["q"]), "20261027-q-2");
    assert_eq!(place_file(&r, "20261027-q"), before);
    assert_eq!(place_names(&r), ["20261027-q", "20261027-q-2"]);
}

#[test]
fn scenario_refused_draft_frees_only_its_own_place() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["b"]);
    push_propose(&r.work, "b", 1);
    take_place(&r.work, "draft-1-b", PlaceKind::Draft, &[], 3, 4).unwrap();
    let d = draft(&r.work, &["b".to_string()], None, None, 3, 4, &clock("UTC")).unwrap();
    assert!(d.drafted.is_none());
    assert_eq!(place_names(&r), ["draft-1-b"]);
}

#[test]
fn a_full_clone_claims_nothing() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let r = with(&["a"]);
    take_place(&r.work, "x", PlaceKind::Build, &[], 1, 4).unwrap();
    match draft(&r.work, &["a".to_string()], None, None, 1, 4, &clock("UTC")) {
        Err(DraftError::NoPlace(why)) => {
            assert!(why.contains("1 of max_builds = 1 in use"), "{why}")
        }
        other => panic!("{other:?}"),
    }
    assert!(remote_tip(&r, "propose/a").is_none());
    assert_eq!(place_names(&r), ["x"]);
}

/// The prompt `draft` writes for `slugs`, after a live claim of each of `claimed`.
fn prompt_for(slugs: &[&str], claimed: &[&str]) -> String {
    let r = with(slugs);
    for slug in claimed {
        push_propose(&r.work, slug, 1);
    }
    let slugs: Vec<String> = slugs.iter().map(|s| s.to_string()).collect();
    let drafted = draft(&r.work, &slugs, None, None, 3, 4, &clock("UTC"))
        .unwrap()
        .drafted
        .expect("drafted");
    fs::read_to_string(&drafted.file).unwrap()
}

#[test]
fn draft_needing_review_folds() {
    // req:orchestrator.triage/draft@66686cf9
    // A draft the gate calls `spec review` stops for the fold; one it passes as `spec` lands.
    let p = prompt_for(&["a"], &[]);
    let at = |s: &str| p.find(s).unwrap_or_else(|| panic!("{s} missing from {p}"));
    let stops = at("When it is `spec review`, open no pull request and stop after your last push");
    assert!(at("go by the first line of your last `clm gate` run") < stops);
    assert!(stops < at("folds every change needing review"));
    let lands = at("When it is `spec`, write the pull request body");
    assert!(stops < lands && lands < at("clm land spec/<id>"));
    assert!(p.contains("`proposed <id> review` for a change that needs review"));
    // The approval the gate asks for is expected; a burden never goes to pass it.
    assert!(p.contains("a burden is never removed to make the gate pass"));
    assert!(p.contains(
        "A `human-task` item it adds is a burden: declare one for it in `human_burdens`"
    ));
    assert!(p.contains("list under `human_burdens` what the change spends"));
}

/// The repo with node `check` (requirement `x`) and `items` as `(slug, frontmatter lines,
/// body)` in `origin/main`'s backlog.
fn with_items(items: &[(&str, &str, &str)]) -> Repo {
    let r = repo();
    write(&r.work, "specs/check/SPEC.md", &node("", &["x"]));
    for (slug, fm, body) in items {
        write(
            &r.work,
            &format!("backlog/{slug}.md"),
            &format!("---\ntitle: T\n{fm}---\n{body}\n"),
        );
    }
    push_main(&r.work, "items");
    r
}

/// The prompt `draft` writes for `slugs` in `r`, with its change ID.
fn drafted_prompt(r: &Repo, slugs: &[&str]) -> (String, String) {
    let slugs: Vec<String> = slugs.iter().map(|s| s.to_string()).collect();
    let drafted = draft(&r.work, &slugs, None, None, 3, 4, &clock("UTC"))
        .unwrap()
        .drafted
        .expect("drafted");
    (drafted.id, fs::read_to_string(&drafted.file).unwrap())
}

/// The one paragraph of `p` that contains `needle`.
fn paragraph<'a>(p: &'a str, needle: &str) -> &'a str {
    let paras: Vec<&str> = p.split("\n\n").filter(|s| s.contains(needle)).collect();
    let [para] = paras[..] else {
        panic!("one paragraph with {needle}: {paras:?}");
    };
    para
}

#[test]
fn scenario_items_claimed_and_prompt_written() {
    // req:orchestrator.triage/draft@66686cf9
    let r = with_items(&[
        (
            "a",
            "kind: bug\ncomponents: [check]\npriority: P3\nblocks: [check/x]\n",
            "The body of item a.",
        ),
        ("b", "kind: bug\n", "Body b."),
    ]);
    let branches = git(&r.work, &["branch", "--format=%(refname)"]);
    let (id, p) = drafted_prompt(&r, &["a", "b"]);
    assert_eq!(id, "20261027-a");
    for slug in ["a", "b"] {
        assert!(
            remote_tip(&r, &format!("propose/{slug}")).is_some(),
            "{slug}"
        );
    }
    assert_eq!(
        git(&r.work, &["branch", "--format=%(refname)"]),
        branches,
        "no local branch was created"
    );
    let exclude = fs::read_to_string(r.work.join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|l| l == ".clm/"), "{exclude}");
    let item_a = p.lines().find(|l| l.contains("`a`")).expect("item a");
    for needle in ["bug", "P3", "components `[check]`", "check/x"] {
        assert!(item_a.contains(needle), "{needle}: {item_a}");
    }
    for needle in [
        "The body of item a.",
        "git branch --show-current",
        "git switch -c spec/20261027-a",
        "resolves",
        "left <slug>",
        "--diff-filter=A",
        "changes/archive/",
        "already fixes",
        "clm claim <component> --fix backlog/<slug>",
        "Resolves: backlog/<slug>",
        "clm hashes",
        "clm apply",
        "rebase ok",
        "clm gate --base origin/main",
        "human_burdens",
        "gate/undeclared-burden",
        "serves",
        "#### Serves",
        "check/untraced",
        "holds an entry",
        "spec review",
        "gate/approval",
        "a burden is never removed",
        "builds nothing",
        "re-applies the edited deltas",
        ".clm/pr-body.md",
        "clm pr open spec/<id>",
        "clm land spec/<id>",
        "clm defect",
        "clm branch delete <ref>",
        "landed <sha>",
        "proposed <id> review",
        "stopped:",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for absent in [
        "spec/<id> autosync/spec/<id>",
        "superpowers",
        "gh pr create",
        "--body ",
        "gh pr edit",
        "clm watch",
        "clm approve",
        "clm significant",
        "## Significant",
        "gate: ",
        "HARD",
        "SOFT",
        "AUTO",
        "CARGO_TARGET_DIR",
        "exits 0",
    ] {
        assert!(!p.contains(absent), "{absent} in {p}");
    }
}

/// The prompt `draft` writes for `slugs` in `r` with an answers file holding `answers`.
fn answered_prompt(r: &Repo, slugs: &[&str], answers: &str) -> String {
    let file = r.work.join("answers.md");
    fs::write(&file, answers).unwrap();
    let slugs: Vec<String> = slugs.iter().map(|s| s.to_string()).collect();
    let drafted = draft(&r.work, &slugs, Some(&file), None, 3, 4, &clock("UTC"))
        .unwrap()
        .drafted
        .expect("drafted");
    fs::read_to_string(&drafted.file).unwrap()
}

#[test]
fn scenario_answers_in_the_prompt() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("q", "kind: question\n", "Which default?")]);
    let p = answered_prompt(&r, &["q"], "q: use the default\n");
    let at = |s: &str| p.find(s).unwrap_or_else(|| panic!("{s} missing from {p}"));
    // After the items, and before the steps.
    assert!(at("Which default?") < at("## The human's answers"));
    assert!(at("## The human's answers") < at("q: use the default"));
    assert!(at("q: use the default") < at("## Steps"));
    let steps = &p[at("## Steps")..];
    for needle in [
        "given in the triage session",
        "outranking",
        "`ARCHITECTURE.md`, a decision or a requirement",
        "designed, not left",
        "quoted in",
        "A `question` or `human-task` item the answers settle stays in",
        "goes into the specs the change edits",
    ] {
        assert!(steps.contains(needle), "{needle} missing from {steps}");
    }
}

#[test]
fn scenario_an_answer_that_only_edits_an_item() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("h", "kind: human-task\n", "Do the steps.")]);
    let p = answered_prompt(
        &r,
        &["h"],
        "h: trim the done steps and keep the rest open\n",
    );
    let steps = &p[p.find("## Steps").expect("steps")..];
    for needle in [
        "only edit",
        "left out of",
        "stays open",
        "draft-backlog-only-change",
    ] {
        assert!(steps.contains(needle), "{needle} missing from {steps}");
    }
}

#[test]
fn scenario_an_answer_that_leaves_only_code() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("q", "kind: question\n", "Which?")]);
    let p = answered_prompt(
        &r,
        &["q"],
        "q: the rule already says so, only the code is left\n",
    );
    let steps = &p[p.find("## Steps").expect("steps")..];
    for needle in [
        "only code to change",
        "turned into a bug",
        "has landed the branch",
        "the next triage",
        "draft-code-fix-written-rule",
        "`kind` to `bug`",
        "remove its `blocks`",
        "components whose `paths` hold the fix",
        "quoting the answer",
        "the answer turns it into a bug",
        "other fields, such as `priority` and `issues`",
        "end with the spec change's `landed <sha>`",
    ] {
        assert!(steps.contains(needle), "{needle} missing from {steps}");
    }
}

#[test]
fn answers_go_in_verbatim_and_trimmed() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let p = answered_prompt(&r, &["a"], "\n\n  Use `x` <y> & {z}.\n\nAll: P1.  \n\n");
    assert!(
        p.contains("## The human's answers\n\nUse `x` <y> & {z}.\n\nAll: P1.\n\n## Steps"),
        "{p}"
    );
}

#[test]
fn scenario_unreadable_answers() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let dir = r.work.join("a-directory.md");
    fs::create_dir_all(&dir).unwrap();
    let not_utf8 = r.work.join("not-utf8.md");
    fs::write(&not_utf8, [0xff, 0xfe, 0x00]).unwrap();
    for (file, name) in [
        (r.work.join("missing.md"), "missing.md"),
        (dir, "a-directory.md"),
        (not_utf8, "not-utf8.md"),
    ] {
        let slugs = ["a".to_string()];
        match draft(&r.work, &slugs, Some(&file), None, 3, 4, &clock("UTC")) {
            Err(DraftError::Failed(clm_triage::Error::Refused(why))) => {
                assert!(why.contains(name), "{why}")
            }
            other => panic!("{other:?}"),
        }
        assert!(remote_tip(&r, "propose/a").is_none());
        assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    }
}

#[test]
fn answers_are_read_before_fetching() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    // With `origin` gone, a fetch would fail in git; the unreadable file refuses first.
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    fs::remove_dir_all(&r.origin).unwrap();
    let file = r.work.join("missing.md");
    match draft(
        &r.work,
        &["a".to_string()],
        Some(&file),
        None,
        3,
        4,
        &clock("UTC"),
    ) {
        Err(DraftError::Failed(clm_triage::Error::Refused(why))) => {
            assert!(why.contains("missing.md"), "{why}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn scenario_no_answers() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let (_, p) = drafted_prompt(&r, &["a"]);
    assert!(!p.contains("The human's answers"), "{p}");
}

#[test]
fn scenario_whitespace_only_answers() {
    // req:orchestrator.triage/draft-answers@3e3fc364
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let p = answered_prompt(&r, &["a"], "\n  \n\t\n");
    assert!(!p.contains("The human's answers"), "{p}");
}

#[test]
fn scenario_a_deferred_items_code_route() {
    // req:orchestrator.triage/draft-workaround-deferred@7ea3144e
    let r = with_items(&[("k", "kind: deferred\ncomponents: [check]\n", "Later.")]);
    let (_, p) = drafted_prompt(&r, &["k"]);
    let para = paragraph(&p, "clm claim <component> --fix backlog/<slug>");
    for needle in [
        "deferred",
        "deferred-work-resolves-in-code",
        "judged met",
        "designs the work",
        "lies in one of the item's `components`",
        "the item leaves `resolves`",
        "when the claim is refused",
        "designs the work into the change",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

#[test]
fn scenario_a_dropped_item() {
    // req:orchestrator.triage/draft-workaround-deferred@7ea3144e
    let r = with_items(&[("k", "kind: deferred\n", "Later.")]);
    let p = answered_prompt(&r, &["k"], "k: drop it\n");
    let steps = &p[p.find("## Steps").unwrap()..];
    assert!(
        steps.contains(
            "An item the human's answers drop stays in `resolves`: delete its `backlog/<slug>.md`"
        ),
        "{steps}"
    );
}

#[test]
fn scenario_draft_no_old_command_names() {
    // req:orchestrator.triage/draft@66686cf9
    let p = prompt_for(&["a"], &[]);
    let words: Vec<&str> = p
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .collect();
    assert!(!words.contains(&"page"), "the word page is in {p}");
    for pair in words.windows(2) {
        assert!(
            !(pair[0] == "clm" && ["sync", "checkpoint", "page"].contains(&pair[1])),
            "old command name `clm {}` in {p}",
            pair[1]
        );
    }
}

#[test]
fn scenario_a_workaround_items_prompt() {
    // req:orchestrator.triage/draft-workaround-deferred@7ea3144e
    let r = with_items(&[(
        "f",
        "kind: workaround\ncomponents: [check]\nrequirement: check/x@4f9cce1e\n",
        "Hard to build.",
    )]);
    let (_, p) = drafted_prompt(&r, &["f"]);
    assert!(
        p.contains("kind `workaround` (requirement `check/x@4f9cce1e`)"),
        "{p}"
    );
    let para = paragraph(&p, "clm claim <component> --fix backlog/<slug>");
    for needle in [
        "`workaround` item",
        "rewording or re-scoping",
        "`#### Needs` or Design it lacks",
        "deleting the flag",
        "re-recording its `requirement` at the new version",
        "(`workaround-flags`)",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

/// The prompt `draft` writes for item `a` when the repository's `[build] setup` is `setup`.
fn prompt_with_setup(r: &Repo, setup: Option<&str>) -> String {
    let drafted = draft(
        &r.work,
        &["a".to_string()],
        None,
        setup,
        3,
        4,
        &clock("UTC"),
    )
    .unwrap()
    .drafted
    .expect("drafted");
    fs::read_to_string(&drafted.file).unwrap()
}

#[test]
fn scenario_setup_declared() {
    // req:orchestrator.triage/draft-prompt-setup@45e6b734
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let p = prompt_with_setup(&r, Some("scripts/setup.sh"));
    let para = paragraph(&p, "code-fix-needs-a-written-rule");
    for needle in [
        "clm setup",
        "tests-run-in-their-own-tree",
        "setup failed",
        "left <slug>: setup failed",
        "as one plain command",
        "before you run any test there",
        "again after a `clm rebase` or an edit of your own",
        "clm defect",
        "dependency declarations",
        "to be trusted. Run the repository's checks",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

#[test]
fn scenario_nothing_declared() {
    // req:orchestrator.triage/draft-prompt-setup@45e6b734
    let r = with_items(&[("a", "kind: bug\n", "Body.")]);
    let p = prompt_with_setup(&r, None);
    assert!(!p.contains("clm setup"), "{p}");
}

#[test]
fn resume_command_works_beside_the_first_sessions_local_branch() {
    // req:orchestrator.triage/draft-resume@e71d0780
    let r = with(&["a"]);
    let id = "20261112-z";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(&format!("changes/{id}/change.md"), &change("proposed"))],
    );
    // What the draft's first session leaves behind: `spec/<id>` at an older commit, which no
    // worktree has checked out.
    git(&r.work, &["branch", &format!("spec/{id}"), "origin/main"]);
    let drafted = clm_triage::resume(&r.work, id, None, 3, 4).unwrap();
    let prompt = fs::read_to_string(&drafted.file).unwrap();
    let command = prompt
        .split('`')
        .skip(1)
        .step_by(2)
        .find(|span| span.starts_with("git switch"))
        .unwrap_or_else(|| panic!("no git switch command in {prompt}"));
    let tmp = tempfile::TempDir::new().unwrap();
    let wt = tmp.path().join("wt");
    git(
        &r.work,
        &["worktree", "add", "-q", "--detach", wt.to_str().unwrap()],
    );
    let ran = output(
        std::process::Command::new("sh")
            .args(["-c", command])
            .current_dir(&wt),
    );
    assert!(ran.status.success(), "{command}: {ran:?}");
    assert_eq!(
        git(&wt, &["branch", "--show-current"]).trim(),
        format!("spec/{id}")
    );
    assert_eq!(
        git(&wt, &["rev-parse", "HEAD"]).trim(),
        git(&r.work, &["rev-parse", &format!("origin/spec/{id}")]).trim()
    );
}
