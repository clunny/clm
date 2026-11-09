//! `overlap`: what two selections can both break.

use clm_affected::{Selection, overlap};

fn nodes(nodes: &[&str], files: &[&str]) -> Selection {
    Selection::Nodes {
        nodes: nodes.iter().map(|s| s.to_string()).collect(),
        files: files.iter().map(|s| s.to_string()).collect(),
    }
}

fn all(path: &str) -> Selection {
    Selection::All(path.to_string())
}

#[test]
fn both_everything_gives_the_first() {
    // req:check.affected/overlap-selection@2171c658
    assert_eq!(overlap(all("a"), all("b")), all("a"));
}

#[test]
fn one_everything_gives_the_other_unchanged() {
    // req:check.affected/overlap-selection@2171c658
    let other = nodes(&["c"], &["f"]);
    assert_eq!(overlap(all("a"), other.clone()), other);
    assert_eq!(overlap(other.clone(), all("a")), other);
}

#[test]
fn otherwise_the_components_and_files_both_hold() {
    // req:check.affected/overlap-selection@2171c658
    let a = nodes(&["a", "c", "d"], &["f1", "f2", "f4"]);
    let b = nodes(&["b", "c", "d"], &["f2", "f3", "f4"]);
    assert_eq!(overlap(a, b), nodes(&["c", "d"], &["f2", "f4"]));
    assert_eq!(
        overlap(nodes(&["a"], &["x"]), nodes(&["b"], &[])),
        nodes(&[], &[])
    );
}
