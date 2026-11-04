//! `configure-clm`: settings for the repository, the clone and the person, with their source.

use crate::{Repo, printed};

#[test]
fn where_a_setting_comes_from() {
    // req:capability/configure-clm@a478bef4
    let repo = Repo::init();
    repo.write("clm.toml", "host = \"none\"\ndefects = \"backlog\"\n");
    repo.commit("defects to the backlog");
    repo.write("clm.local.toml", "defects = \"off\"\n");

    let out = repo.clm(&["config", "get", "defects"]);
    assert!(out.status.success(), "{}", printed(&out));
    let local = repo.root.join("clm.local.toml");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        format!("off\t{}", local.display())
    );
}
