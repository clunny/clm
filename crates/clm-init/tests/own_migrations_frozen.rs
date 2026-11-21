//! This crate's own migrations are frozen once shipped, judged by `upgrade`'s `check_lock`.

use std::path::Path;

use clm_init::own_migrations;
use clm_upgrade::check_lock;

// clm:reads crates/clm-init/migrations.lock
// req:init/own-migrations-frozen@48ed42ca
#[test]
fn the_crates_own_migrations_match_its_lock() {
    let ids: Vec<&str> = own_migrations().iter().map(|m| m.id).collect();
    check_lock(Path::new(env!("CARGO_MANIFEST_DIR")), &ids).unwrap();
}
