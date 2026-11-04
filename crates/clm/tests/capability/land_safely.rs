//! `land-safely`: one command merges a branch, checks the result and records the land.

use crate::{Repo, printed};

/// The project's tests: each `tests/t_*.sh`, run in turn until one fails.
const CHECK: &str =
    "set -e\nfor t in tests/t_*.sh; do [ -e \"$t\" ] || continue; sh \"$t\"; done\n";

#[test]
fn a_branch_that_breaks_main() {
    // req:capability/land-safely@6d0c50dd
    let repo = Repo::init();
    repo.write("src/lib.sh", "greet() { echo hi; }\n");
    repo.write("tests/check.sh", CHECK);
    repo.commit("greet");
    repo.land_on_origin("sh tests/check.sh");

    // A test of `greet`, passing on its own branch.
    repo.write(
        "tests/t_greet.sh",
        ". src/lib.sh\n[ \"$(greet)\" = hi ] || { echo \"t_greet failed: greet said $(greet)\"; exit 1; }\n",
    );
    repo.push_branch("impl/greet-test", "test greet");
    repo.git(&["switch", "-q", "impl/greet-test"]);
    let alone = repo.clm(&["land", "--check-only"]);
    assert!(alone.status.success(), "{}", printed(&alone));
    repo.git(&["switch", "-q", "main"]);
    // Meanwhile `greet` changes on `main`, through a land of its own.
    repo.write("src/lib.sh", "greet() { echo hello; }\n");
    repo.push_branch("impl/hello", "greet says hello");
    let first = repo.clm(&["land", "impl/hello"]);
    assert!(first.status.success(), "{}", printed(&first));
    let before = repo.origin_main();

    let out = repo.clm(&["land", "impl/greet-test"]);
    let text = printed(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("t_greet failed: greet said hello"), "{text}");
    assert!(!text.lines().any(|l| l.starts_with("landed ")), "{text}");
    assert_eq!(repo.origin_main(), before, "{text}");
    let ledger = repo.ledger();
    assert!(
        !ledger
            .iter()
            .any(|r| r[0] == "landed" && r[2] == "impl/greet-test"),
        "{ledger:?}"
    );
}
