//! `slow-tests-after-land`: tests too slow for every land run after it, and a break comes back
//! as a bug on the components it broke, holding code lands there until it is fixed.

use crate::{Repo, printed};

const COMPONENT_B: &str = "---\ntitle: B\nstatus: active\nowners: []\npaths: [\"lib/**\"]\n---\n\n## Contract\nB does z.\n\n## Requirements\n";
const CHANGE: &str = "---\nstatus: done\n---\n\n## Intent\n\nx\n";

/// The slow test: fails while `lib/slow_fail` exists.
const SLOW: &str =
    "[ ! -e lib/slow_fail ] || { echo 'slow test failed: lib/slow_fail exists'; exit 1; }\n";

/// Brings the work tree's `main` up to `origin/main`, where lands have moved it.
fn catch_up(repo: &Repo) {
    repo.git(&["fetch", "-q", "origin"]);
    repo.git(&["merge", "-q", "--ff-only", "origin/main"]);
}

/// Lands `branch`, which must succeed, and returns what the land printed.
fn land(repo: &Repo, branch: &str) -> String {
    let out = repo.clm(&["land", branch]);
    let text = printed(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.lines().any(|l| l.starts_with("landed ")), "{text}");
    catch_up(repo);
    text
}

#[test]
fn a_slow_test_breaks_after_the_land() {
    // req:capability/slow-tests-after-land@a6d32596
    let repo = Repo::init();
    repo.write("specs/b/SPEC.md", COMPONENT_B);
    repo.write("lib/b.sh", "b() { :; }\n");
    repo.write("slow.sh", SLOW);
    repo.write(
        "clm.toml",
        "host = \"git\"\n\n[land]\ncheck = \"true\"\n\n[verify]\nmode = \"local\"\ncheck = \"sh slow.sh\"\ntriggers = [\"change-done\"]\n",
    );
    repo.commit("component b and its slow test");
    repo.git(&["push", "-q", "origin", "main"]);

    // The slow tests start green, on a first run the project's trigger fires.
    repo.write("changes/archive/first/change.md", CHANGE);
    repo.push_branch("spec/archive-first", "archive first");
    land(&repo, "spec/archive-first");
    let first = repo.clm(&["verify", "--due"]);
    assert!(first.status.success(), "{}", printed(&first));
    assert!(
        printed(&first).contains("verify passed"),
        "{}",
        printed(&first)
    );

    // A land breaks them, and the change that holds it finishes.
    repo.write("lib/slow_fail", "x\n");
    repo.push_branch("impl/b-breaks", "b breaks the slow test");
    land(&repo, "impl/b-breaks");
    let culprit = repo.git(&["rev-parse", "origin/main"]).trim().to_string();
    repo.write("changes/archive/second/change.md", CHANGE);
    repo.push_branch("spec/archive-second", "archive second");
    land(&repo, "spec/archive-second");
    let due = repo.clm(&["verify", "--due"]);
    let text = printed(&due);
    assert_eq!(due.status.code(), Some(1), "{text}");
    let line = text
        .lines()
        .find(|l| l.starts_with("verify failed "))
        .unwrap_or_else(|| panic!("{text}"));
    let short = &culprit[..7];
    assert!(line.contains(&format!("culprit {short}")), "{line}");
    assert!(line.ends_with("components b"), "{line}");
    let red = repo.git(&["rev-parse", "origin/main"]).trim().to_string();
    assert!(
        repo.ledger()
            .iter()
            .any(|r| r[0] == "verified" && r.iter().any(|w| w.starts_with(&red[..7]))),
        "{:?}",
        repo.ledger()
    );

    // A land touching b warns of the failure and lands.
    repo.write("lib/other.sh", "o() { :; }\n");
    repo.push_branch("impl/b-other", "b other");
    let warned = land(&repo, "impl/b-other");
    let warning = warned
        .lines()
        .find(|l| l.contains("warning: slow tests failed"))
        .unwrap_or_else(|| panic!("{warned}"));
    assert!(
        warning.contains(&format!("culprit {culprit}")) && warning.contains("on b"),
        "{warning}"
    );
    assert!(warning.contains("/clm:triage"), "{warning}");

    // Triage offers the failure, as a new bug on b.
    let intake = printed(&repo.clm(&["intake"]));
    assert!(
        intake
            .lines()
            .any(|l| l.starts_with("verify ") && l.contains(" new ") && l.ends_with(" b")),
        "{intake}"
    );

    // The person takes it in as a bug item naming the red commit.
    repo.write(
        "backlog/b-broke.md",
        &format!("---\ntitle: Slow test fails on b\nkind: bug\ncomponents: [b]\nverify_failures: [\"{red}\"]\n---\nlib/slow_fail breaks the slow test.\n"),
    );
    repo.push_branch("spec/take-in", "take in the slow failure");
    land(&repo, "spec/take-in");
    let intake = printed(&repo.clm(&["intake"]));
    assert!(
        intake
            .lines()
            .any(|l| l.starts_with("verify ") && l.ends_with(" imported b-broke")),
        "{intake}"
    );

    // Code lands on b are refused, others land.
    repo.write("lib/more.sh", "m() { :; }\n");
    repo.push_branch("impl/b-more", "b more");
    let before = repo.origin_main();
    let refused = repo.clm(&["land", "impl/b-more"]);
    let text = printed(&refused);
    assert!(!refused.status.success(), "{text}");
    assert!(
        text.contains("land/verify-stop") && text.contains("b-broke"),
        "{text}"
    );
    assert_eq!(repo.origin_main(), before, "{text}");
    repo.write("src/a.sh", "a() { :; }\n");
    repo.push_branch("impl/a-more", "a more");
    land(&repo, "impl/a-more");

    // The fix names the item and removes it, and lands; the slow tests pass again.
    repo.git(&["switch", "-q", "-c", "impl/b-fix"]);
    repo.git(&["rm", "-q", "lib/slow_fail", "backlog/b-broke.md"]);
    repo.git(&["commit", "-q", "-m", "fix b\n\nResolves: backlog/b-broke"]);
    repo.git(&["push", "-q", "-u", "origin", "impl/b-fix"]);
    repo.git(&["switch", "-q", "main"]);
    land(&repo, "impl/b-fix");
    let fixed = repo.clm(&["verify", "--commit", "origin/main"]);
    assert!(fixed.status.success(), "{}", printed(&fixed));
}
