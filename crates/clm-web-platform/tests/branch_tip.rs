//! A branch's tip, told from a missing branch and a failed read (`branch-tip`).

use clm_web_platform::access;
use clm_web_platform::fake::*;
use clm_web_platform::{gh, ledger};

#[test]
fn a_tip_a_missing_branch_and_a_failed_read() {
    // req:web.worker.platform/branch-tip@69ad8a87
    let fake = Fake::default();
    let mut repo = FakeRepo::new("000", clm_repo());
    repo.branches.insert("spec/x".into(), "111".into());
    repo.branches.insert("spec/z".into(), "333".into());
    repo.ref_fails.insert("spec/z".into());
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let acc = block_on(access::check(&fake, "u", "o", "a")).unwrap();
    let asked = |branch: &str| {
        let calls = fake.gh.calls.borrow();
        let url = format!("{}/repos/o/a/git/ref/heads/{branch}", gh::API);
        let mine: Vec<_> = calls.iter().filter(|c| c.url == url).collect();
        assert!(
            mine.iter()
                .all(|c| c.method == "GET" && c.auth == *acc.auth())
        );
        mine.len()
    };
    let tips: Vec<_> = ["spec/x", "spec/y", "spec/z"]
        .iter()
        .map(|b| block_on(ledger::branch_tip(&fake, &acc, b)))
        .collect();
    assert_eq!(
        tips,
        [Ok(Some("111".to_string())), Ok(None), Err(gh::Failed)]
    );
    for b in ["spec/x", "spec/y", "spec/z"] {
        assert_eq!(asked(b), 1, "{b}: branch_tip asked once");
    }
    let tips: Vec<_> = ["spec/x", "spec/y", "spec/z"]
        .iter()
        .map(|b| block_on(ledger::tip(&fake, &acc, b)))
        .collect();
    assert_eq!(tips, [Some("111".to_string()), None, None]);
    for b in ["spec/x", "spec/y", "spec/z"] {
        assert_eq!(asked(b), 2, "{b}: tip asked once");
    }
}
