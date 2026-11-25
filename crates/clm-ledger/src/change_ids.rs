//! The change IDs a branch's paths name (`ledger/change-ids`).

use std::collections::BTreeSet;

/// For the paths a branch adds or modifies, the `<id>` of each that is
/// `changes/<id>/change.md`, `<id>` one non-empty path component other than `archive`,
/// in ascending byte order, each once. An `approved` record's `change` is the first,
/// and a `landed` record's `changes` are all of them.
pub fn change_ids<I>(paths: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let ids: BTreeSet<String> = paths
        .into_iter()
        .filter_map(|p| {
            let id = p
                .as_ref()
                .strip_prefix("changes/")?
                .strip_suffix("/change.md")?;
            (!id.is_empty() && id != "archive" && !id.contains('/')).then(|| id.to_string())
        })
        .collect();
    ids.into_iter().collect()
}
