//! The model's term lists: the words its enums and `change.md` keys are written as.
//!
//! Serde names every variant or field it accepts in the error for an unknown one, so
//! each list is compared with what the type itself reads, and a new variant or field
//! fails here until the list holds it. The words the model only reads, from before
//! `plain-terms-renames`, are named here and listed nowhere (`renamed-words-still-load`).

use std::collections::BTreeSet;

use clm_model::{
    BACKLOG_KIND_TERMS, BacklogKind, CHANGE_KEYS, CHANGE_STATUS_TERMS, ChangeFrontmatter,
    ChangeStatus, STATUS_TERMS, Status,
};
use serde::de::DeserializeOwned;

/// The words and keys the model reads and no longer writes.
const READ_ONLY: [&str; 4] = ["specd", "human", "friction", "burdens"];

/// The backtick-quoted names serde lists after "expected" when it rejects `yaml`.
fn accepted<T: DeserializeOwned>(yaml: &str) -> BTreeSet<String> {
    let err = serde_yaml::from_str::<T>(yaml)
        .err()
        .expect("rejected")
        .to_string();
    let after = &err[err.find("expected").expect("serde names what it expected")..];
    after
        .split('`')
        .skip(1)
        .step_by(2)
        .map(String::from)
        .collect()
}

/// `accepted`, less the words the model only reads.
fn written<T: DeserializeOwned>() -> BTreeSet<String> {
    let mut words = accepted::<T>("zzz");
    words.retain(|w| !READ_ONLY.contains(&w.as_str()));
    words
}

/// Each listed variant once, read by its word, the words are all serde reads and writes,
/// and none is a word the model only reads.
fn check<T: Copy + PartialEq + std::fmt::Debug + DeserializeOwned>(terms: &[(T, &str)]) {
    let words: BTreeSet<String> = terms.iter().map(|(_, w)| w.to_string()).collect();
    assert_eq!(words.len(), terms.len(), "a word is listed twice");
    assert_eq!(words, written::<T>(), "list and type differ");
    for (v, word) in terms {
        assert!(!READ_ONLY.contains(word), "{word} is only read");
        assert_eq!(serde_yaml::from_str::<T>(word).unwrap(), *v, "{word}");
    }
}

#[test]
fn enum_lists_match_the_types() {
    // req:model/term-lists@28b92c61
    check::<Status>(STATUS_TERMS);
    check::<ChangeStatus>(CHANGE_STATUS_TERMS);
    check::<BacklogKind>(BACKLOG_KIND_TERMS);
}

#[test]
fn lists_hold_the_words_clm_writes() {
    // req:model/term-lists@28b92c61
    let status: Vec<&str> = STATUS_TERMS.iter().map(|(_, w)| *w).collect();
    assert_eq!(status, ["unbuilt", "active", "deprecated"]);
    let kinds: Vec<&str> = BACKLOG_KIND_TERMS.iter().map(|(_, w)| *w).collect();
    assert_eq!(
        kinds,
        [
            "bug",
            "deferred",
            "idea",
            "question",
            "human-task",
            "workaround"
        ]
    );
    for (kind, word) in BACKLOG_KIND_TERMS {
        assert_eq!(&kind.to_string(), word);
    }
}

#[test]
fn key_list_holds_every_field_but_the_ignored_ones() {
    // req:model/term-lists@28b92c61
    let mut fields = accepted::<ChangeFrontmatter>("zzz: 1");
    for ignored in ["after", "prefer_after", "gate", "burdens"] {
        assert!(fields.remove(ignored), "{ignored} is a field");
    }
    let listed: BTreeSet<String> = CHANGE_KEYS.iter().map(|k| k.to_string()).collect();
    assert_eq!(listed.len(), CHANGE_KEYS.len(), "a key is listed twice");
    assert!(listed.contains("human_burdens"));
    assert_eq!(listed, fields);
}
