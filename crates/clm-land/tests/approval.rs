mod common;

use clm_ledger::Body;
use common::{Fixture, refused};

/// A `spec/x` that needs review, authored by the work clone's `Christopher Lunny`, approved from `approver`.
fn approved_by(settings: &str, approver: Option<&str>) -> Fixture {
    let f = Fixture::new(settings);
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let repo = match approver {
        Some(user) => f.clone_as("approver", user),
        None => f.work(),
    };
    let record = f.append(
        &repo,
        Body::approved(f.fingerprint("spec/x"), "spec/x".into(), Some("x".into())),
    );
    let person = record.writer.split('/').next().unwrap().to_owned();
    assert_eq!(
        person,
        approver.unwrap_or("christopher-lunny").to_lowercase()
    );
    f
}

#[test]
fn self_approval_refused() {
    // req:land/approval-policy@822b3d40
    let f = approved_by("approval = \"other\"", None);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("approval = \"other\""), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn another_person_approves() {
    // req:land/approval-policy@822b3d40
    let f = approved_by("approval = \"other\"", Some("Dana"));
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn any() {
    // req:land/approval-policy@822b3d40
    let f = approved_by("approval = \"any\"", None);
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn why_no_approval_counts() {
    // req:land/approval-policy@822b3d40
    // Under `other`, the author's own approval: the policy, why it doesn't count, and whom
    // to ask.
    let f = approved_by("approval = \"other\"", None);
    let writer = f.records()[0].writer.clone();
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    assert!(
        lines.contains(
            &"approval = \"other\" from origin/main:clm.toml: only an approval by someone who \
              isn't an author of the branch counts"
        ),
        "{why}"
    );
    let at = lines
        .iter()
        .position(|l| *l == "no approval of this diff counts:")
        .unwrap_or_else(|| panic!("{why}"));
    let refusal = lines[at + 1];
    assert!(refusal.starts_with(&format!("{writer} at ")), "{why}");
    assert!(refusal.contains("author"), "{why}");
    assert_eq!(
        lines.last(),
        Some(&"ask another person to run `clm approve spec/x`"),
        "{why}"
    );
    assert_eq!(f.remote("main"), old);
    // Under `any`, with no approval: no policy line, and the human is asked.
    let g = Fixture::new("approval = \"any\"");
    g.write("gate-out", "spec review\n");
    g.write("needs-approval", "");
    g.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let old = g.remote("main");
    let why = refused(clm_land::land(&g.work(), Some("spec/x"), &g.env(&[])));
    assert!(!why.contains("approval = \"other\""), "{why}");
    assert!(!why.contains("no approval of this diff counts"), "{why}");
    assert_eq!(
        why.lines().last(),
        Some("ask the human to run `clm approve spec/x`"),
        "{why}"
    );
    assert_eq!(g.remote("main"), old);
}

/// A `spec/x` that needs review, authored by `Christopher Lunny` whose head's `clm.toml` holds `head` where
/// `main`'s holds `base`, self-approved, with the work clone left on `spec/x`.
fn branch_sets(f: &Fixture, base: &str, head: &str) {
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    let toml = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    let toml = toml.replacen(base, head, 1);
    f.branch(
        "spec/x",
        &[("changes/x/change.md", "x\n"), ("clm.toml", &toml)],
    );
    let fingerprint = f.fingerprint("spec/x");
    f.git(&f.work(), &["checkout", "-q", "spec/x"]);
    f.append(
        &f.work(),
        Body::approved(fingerprint, "spec/x".into(), Some("x".into())),
    );
}

#[test]
fn a_branch_that_weakens_the_policy_is_judged_by_the_base() {
    // req:land/approval-from-base@2e526f76
    let f = Fixture::new("approval = \"other\"");
    branch_sets(&f, "approval = \"other\"", "approval = \"any\"");
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), None, &f.env(&[])));
    assert!(why.contains("gate/approval"), "{why}");
    assert!(why.contains("approval = \"other\""), "{why}");
    assert_eq!(why.matches("origin/main:clm.toml").count(), 1, "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn a_host_none_clone_is_judged_by_origin_main() {
    // req:land/approval-from-base@2e526f76
    let f = Fixture::local_with("approval = \"other\"");
    branch_sets(&f, "approval = \"other\"", "approval = \"any\"");
    // The local `main` says `any` and `spec/x` is on `origin` alone: neither is the base.
    let work = f.work();
    f.git(&work, &["checkout", "-q", "main"]);
    f.git(&work, &["branch", "-q", "-D", "spec/x"]);
    let toml = std::fs::read_to_string(work.join("clm.toml")).unwrap();
    let toml = toml.replacen("approval = \"other\"", "approval = \"any\"", 1);
    f.commit_files(&[("clm.toml", &toml)], "local only");
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/x"), &f.env(&[])));
    assert!(why.contains("gate/approval"), "{why}");
    assert_eq!(why.matches("origin/main:clm.toml").count(), 1, "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn a_branch_that_tightens_the_policy_is_judged_by_the_base() {
    // req:land/approval-from-base@2e526f76
    let f = Fixture::new("# no approval");
    branch_sets(&f, "# no approval", "approval = \"other\"");
    clm_land::land(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn the_clones_local_file_wins() {
    // req:land/approval-from-base@2e526f76
    let f = approved_by("approval = \"other\"", None);
    f.write("work/clm.local.toml", "approval = \"any\"\n");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn the_clones_local_file_can_tighten() {
    // req:land/approval-from-base@2e526f76
    let f = approved_by("approval = \"any\"", None);
    f.write("work/clm.local.toml", "approval = \"other\"\n");
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("clm.local.toml"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn unreadable_policy_at_the_base() {
    // req:land/approval-from-base@2e526f76
    let f = Fixture::new("approval = \"everyone\"");
    branch_sets(&f, "approval = \"everyone\"", "approval = \"any\"");
    let old = f.remote("main");
    let records = f.records().len();
    let why = match clm_land::land(&f.work(), None, &f.env(&[])) {
        Err(clm_land::Error::Git(why)) => why,
        other => panic!("expected exit 2, got {other:?}"),
    };
    assert!(why.contains("origin/main:clm.toml"), "{why}");
    assert!(why.contains("approval"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("gate.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert_eq!(f.records().len(), records);
}

#[test]
fn unparseable_policy_at_the_base() {
    // req:land/approval-from-base@2e526f76
    let f = Fixture::new("approval = = =");
    branch_sets(&f, "approval = = =", "approval = \"any\"");
    let old = f.remote("main");
    let records = f.records().len();
    let why = match clm_land::land(&f.work(), None, &f.env(&[])) {
        Err(clm_land::Error::Git(why)) => why,
        other => panic!("expected exit 2, got {other:?}"),
    };
    assert!(why.contains("origin/main:clm.toml"), "{why}");
    assert!(why.contains("approval"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("gate.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert_eq!(f.records().len(), records);
}
