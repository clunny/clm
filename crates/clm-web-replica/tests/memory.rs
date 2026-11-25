//! What the object keeps in its own memory while it is awake, where the Worker keeps it in
//! the Cache API (`object-class`).

mod common;

use clm_web_platform::fake::files;
use common::*;
use futures::executor::block_on;

#[test]
fn an_installation_token_is_minted_once() {
    // req:web.worker.replica/object-class@2f468bf7
    let mut all = repo_files(&[]);
    for i in 0..400 {
        all.extend(files(&[(&format!("src/f{i}.rs"), "fn f() {}")]));
    }
    let fake = fake_with("111", all);
    let (mut object, _mem, slot) = object();
    assert_eq!(block_on(object.view(&fake, "snapshot", false)).status, 503);
    let alarms = run_alarms(&fake, &mut object, &slot);
    assert!(
        alarms.len() > 2,
        "the build takes several alarms: {alarms:?}"
    );
    let mints = urls(&fake, 0)
        .iter()
        .filter(|u| u.ends_with("/access_tokens"))
        .count();
    assert_eq!(mints, 1, "one token serves every alarm of the build");
}
