//! `readers`: the tests that read files a change touches, named by a marker in the test.

use std::collections::BTreeMap;

use crate::select::Changed;

/// The marker's text. Built so this file isn't a reader of whatever follows it.
const MARK: &str = concat!("clm", ":reads ");

/// The globs of the markers in `text`: each is the marker's text and the glob after it, up to
/// the next whitespace; a marker with no glob after it is ignored.
pub fn reads_in(text: &str) -> Vec<String> {
    text.match_indices(MARK)
        .filter_map(|(at, _)| {
            let rest = &text[at + MARK.len()..];
            if rest.starts_with(char::is_whitespace) {
                return None;
            }
            rest.split_whitespace().next().map(str::to_string)
        })
        .collect()
}

/// The files of `reads` (each scanned file with its globs) one of whose globs matches a path of
/// `differing`, sorted. A glob reads as a component's `paths` do.
pub fn readers(differing: &[Changed], reads: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    reads
        .iter()
        .filter(|(_, globs)| {
            globs.iter().any(|g| {
                differing
                    .iter()
                    .any(|c| clm_check::path_matches(g, &c.path))
            })
        })
        .map(|(path, _)| path.clone())
        .collect()
}
