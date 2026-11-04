//! `triage-unattended`: the backlog is grouped and drafted in the background; the drafts that
//! land on checks land, and the ones that need review are gathered into one branch for one
//! approval.

use crate::{Repo, landed, printed};

/// A node `name`, owning `<name>/**`, with requirement `x`.
fn node(name: &str) -> String {
    format!(
        "---\ntitle: {name}\nstatus: active\nowners: []\npaths: [\"{name}/**\"]\nserves: [capability/c]\n---\n\n## Contract\n{name} does x.\n\n## Requirements\n\n### Requirement: x\n{name} SHALL x.\n\n#### Scenario: x\n- GIVEN g\n- WHEN w\n- THEN t\n"
    )
}

/// A `bug` item on `node`.
fn bug(node: &str) -> String {
    format!(
        "---\ntitle: Fix {node}\nkind: bug\ncomponents: [{node}]\n---\nThe {node} rule is wrong.\n"
    )
}

/// A repository whose backlog holds one bug on each of nodes `a`, `b` and `c`.
fn backlog() -> Repo {
    let repo = Repo::init();
    for name in ["b", "c"] {
        repo.write(&format!("specs/{name}/SPEC.md"), &node(name));
    }
    for name in ["a", "b", "c"] {
        repo.write(&format!("backlog/fix-{name}.md"), &bug(name));
    }
    repo.commit("a backlog");
    repo.land_on_origin("true");
    repo
}

/// What `clm draft <slug>` prints on stderr last: the change ID it chose.
fn draft(repo: &Repo, slug: &str) -> String {
    let out = repo.clm(&["draft", slug]);
    let text = printed(&out);
    assert!(out.status.success(), "{text}");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let id = stderr
        .lines()
        .last()
        .and_then(|l| l.strip_prefix("draft "))
        .unwrap_or_else(|| panic!("no draft line: {text}"))
        .to_string();
    // The launcher names the prompt file the draft's session follows.
    let prompt = repo.root.join(format!(".clm/prompts/draft/{id}.md"));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(prompt.to_str().unwrap()),
        "{text}"
    );
    id
}

/// What a draft's session does on its branch `spec/<id>` in a worktree of its own: it writes
/// the change `id` that resolves `slug` and rewords `node`'s `x`, with `burdens` when given,
/// applies it, and pushes.
fn design(repo: &Repo, id: &str, slug: &str, node: &str, burdens: Option<&str>) -> Repo {
    let dir = repo.root.join(format!(".claude/worktrees/agent-{id}"));
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        &format!("spec/{id}"),
        dir.to_str().unwrap(),
        "origin/main",
    ]);
    let session = repo.at(&dir);
    let base = session.hash(node, "requirement:x");
    let burdens = burdens.map_or(String::new(), |b| format!("burdens: [{b:?}]\n"));
    session.write(
        &format!("changes/{id}/change.md"),
        &format!(
            "---\nstatus: proposed\nresolves: [{slug}]\n{burdens}---\n\n## Intent\n\n{node} does x properly.\n"
        ),
    );
    session.write(
        &format!("changes/{id}/deltas/{node}.md"),
        &format!(
            "---\ncomponent: {node}\ncreate: false\n---\n## MODIFIED Requirements\n\n### Requirement: x <!-- base: {base} -->\n{node} SHALL x properly.\n\n#### Scenario: x\n- GIVEN g\n- WHEN w\n- THEN t, properly\n"
        ),
    );
    let apply = session.clm(&["apply", id]);
    assert!(apply.status.success(), "{}", printed(&apply));
    session.commit(&format!("design {id}"));
    session.git(&["push", "-q", "-u", "origin", &format!("spec/{id}")]);
    session
}

/// The first line `clm gate` prints for the session's branch.
fn gate(session: &Repo) -> String {
    let out = session.clm(&["gate", "--base", "origin/main"]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines().next().unwrap_or_default().to_string()
}

#[test]
fn triage_a_backlog() {
    // req:capability/triage-unattended@5567dfc1
    let repo = backlog();

    // `/clm:triage` groups the backlog: one group of bugs per node.
    let out = repo.clm(&["triage"]);
    let text = printed(&out);
    assert!(out.status.success(), "{text}");
    let groups: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("group "))
        .map(str::to_string)
        .collect();
    assert_eq!(groups.len(), 3, "{text}");
    for name in ["a", "b", "c"] {
        let slug = format!("fix-{name}");
        assert!(
            groups.iter().any(|g| g.ends_with(&slug)),
            "no group for {slug}: {text}"
        );
    }

    // Each group is drafted in the background: its items are claimed, and its session
    // designs the change on a branch of its own.
    let (a, b, c) = (
        draft(&repo, "fix-a"),
        draft(&repo, "fix-b"),
        draft(&repo, "fix-c"),
    );
    let session_a = design(&repo, &a, "fix-a", "a", None);
    let session_b = design(&repo, &b, "fix-b", "b", Some("a paid service"));
    let session_c = design(
        &repo,
        &c,
        "fix-c",
        "c",
        Some("a step only the human can take"),
    );

    // The draft that needs no review lands on checks.
    repo.git(&["fetch", "-q", "origin"]);
    assert_eq!(gate(&session_a), "spec");
    let commit = landed(&session_a.clm(&["land", &format!("spec/{a}")]));
    assert_eq!(repo.origin_main(), commit);
    let done = repo.clm(&["draft", "--finish", &a]);
    assert!(done.status.success(), "{}", printed(&done));

    // The ones that need review are not landed, and wait on their branches.
    repo.git(&["fetch", "-q", "origin"]);
    assert_eq!(gate(&session_b), "spec review");
    assert_eq!(gate(&session_c), "spec review");
    let refused = session_b.clm(&["land", &format!("spec/{b}")]);
    assert!(!refused.status.success(), "{}", printed(&refused));

    // They are folded into one spec branch.
    let fold = repo.clm(&["draft", "--fold", &b, &c]);
    assert!(fold.status.success(), "{}", printed(&fold));
    let batch = format!("{b}-fold");
    assert!(
        repo.root.join(format!(".clm/prompts/fold/{b}.md")).exists(),
        "{}",
        printed(&fold)
    );
    let dir = repo.root.join(".claude/worktrees/agent-fold");
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        &format!("spec/{batch}"),
        dir.to_str().unwrap(),
        "origin/main",
    ]);
    let folder = repo.at(&dir);
    for id in [&b, &c] {
        folder.git(&[
            "checkout",
            "-q",
            &format!("origin/spec/{id}"),
            "--",
            &format!("changes/{id}"),
        ]);
    }
    folder.commit("fold");
    let rebase = folder.clm(&["rebase"]);
    let rebase_text = printed(&rebase);
    assert_eq!(
        rebase_text.lines().last(),
        Some("rebase ok"),
        "{rebase_text}"
    );
    folder.git(&["push", "-q", "-u", "origin", &format!("spec/{batch}")]);

    // One approval covers both: before it the fold is refused, after it, it lands.
    repo.git(&["fetch", "-q", "origin"]);
    assert_eq!(gate(&folder), "spec review");
    let refused = folder.clm(&["land", &format!("spec/{batch}")]);
    assert!(!refused.status.success(), "{}", printed(&refused));
    let approve = folder.clm(&["approve", &format!("spec/{batch}")]);
    assert!(approve.status.success(), "{}", printed(&approve));
    let commit = landed(&folder.clm(&["land", &format!("spec/{batch}")]));
    assert_eq!(repo.origin_main(), commit);
    let specs = |node: &str| repo.git(&["show", &format!("{commit}:specs/{node}/SPEC.md")]);
    for name in ["a", "b", "c"] {
        assert!(specs(name).contains("SHALL x properly."), "{name}");
    }
    let approvals = repo.ledger().iter().filter(|r| r[0] == "approved").count();
    assert_eq!(approvals, 1);
}
