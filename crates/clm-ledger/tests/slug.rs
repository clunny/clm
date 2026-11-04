//! `slug`, with or without the `git` feature (`ledger/person-slug`).

use clm_ledger::slug;

#[test]
fn names_slugged() {
    // req:ledger/person-slug@da174cce
    assert_eq!(slug("Christopher Lunny"), "christopher-lunny");
    assert_eq!(slug("  Dana_O'Neil "), "dana-o-neil");
}
