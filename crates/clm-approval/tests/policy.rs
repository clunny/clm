//! The `approval` policy for a land onto a base commit (land.approval/policy-at-base).

mod common;

use common::Fixture;

/// Moves `origin/main` on to a commit whose `clm.toml` is `base`, writes the work clone's own
/// `clm.toml` as `root`, uncommitted, and `clm.local.toml` as `local` when given; returns
/// the policy for the base commit, or the error's message.
fn policy(base: &str, root: &str, local: Option<&str>) -> Result<(bool, String), String> {
    let f = Fixture::new("");
    f.advance_main("clm.toml", base);
    let work = f.work();
    f.git(&work, &["fetch", "-q", "origin"]);
    let sha = f.git(&work, &["rev-parse", "origin/main"]);
    std::fs::write(work.join("clm.toml"), root).unwrap();
    if let Some(local) = local {
        std::fs::write(work.join("clm.local.toml"), local).unwrap();
    }
    let env = f.env(&[]);
    let root = std::fs::canonicalize(&work).unwrap();
    let settings = clm_config::load(Some(&root), &env).unwrap();
    let repo = clm_approval::Repo::new(&root, &env, &settings);
    match repo.policy(&sha) {
        Ok(p) => Ok((p.other, p.source)),
        Err(e) => Err(e.to_string()),
    }
}

#[test]
fn the_bases_policy_not_the_checkouts() {
    // req:land.approval/policy-at-base@6dc55892
    let got = policy("approval = \"other\"\n", "approval = \"any\"\n", None);
    assert_eq!(got, Ok((true, "origin/main:clm.toml".into())));
}

#[test]
fn no_key_at_the_base() {
    // req:land.approval/policy-at-base@6dc55892
    let got = policy("# no approval\n", "approval = \"other\"\n", None);
    assert_eq!(got, Ok((false, "origin/main:clm.toml".into())));
}

#[test]
fn the_clones_local_file_wins() {
    // req:land.approval/policy-at-base@6dc55892
    let got = policy(
        "approval = \"other\"\n",
        "",
        Some("approval = \"any\"\n"),
    );
    assert_eq!(got, Ok((false, "clm.local.toml".into())));
    // And it tightens as well as loosens.
    let got = policy("# no approval\n", "", Some("approval = \"other\"\n"));
    assert_eq!(got, Ok((true, "clm.local.toml".into())));
}

#[test]
fn unreadable_policy_at_the_base() {
    // req:land.approval/policy-at-base@6dc55892
    let why = policy("approval = \"everyone\"\n", "", None).unwrap_err();
    assert!(why.starts_with("approval from origin/main:clm.toml:"), "{why}");
    assert!(why.contains("everyone"), "{why}");
    let why = policy("approval = [\n", "", None).unwrap_err();
    assert!(why.starts_with("approval from origin/main:clm.toml:"), "{why}");
}
