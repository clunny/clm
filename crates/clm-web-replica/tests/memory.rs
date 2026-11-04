//! What the object keeps in its own memory while it is awake, where the Worker keeps it in
//! the Cache API (`object-class`).

mod common;

use clm_web_platform::fake::files;
use common::*;

#[test]
fn an_installation_token_is_minted_once() {
    // req:web.worker.replica/object-class@2ce37c07
    let mut all = repo_files(&[]);
    for i in 0..400 {
        all.extend(files(&[(&format!("src/f{i}.rs"), "fn f() {}")]));
    }
    let fake = fake_with("111", all);
    let (_object, _mem, _slot) = built(&fake);
    let mints = urls(&fake, 0)
        .iter()
        .filter(|u| u.ends_with("/access_tokens"))
        .count();
    assert_eq!(mints, 1, "one token serves every alarm of the build");
}
