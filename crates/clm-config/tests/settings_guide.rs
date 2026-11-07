// clm:reads docs/guide/settings.md
//! `settings-guide-covers-keys`: the guide's settings page has a section for every key.

use clm_config::KEYS;

/// The keys of `keys` that no heading `` ### `<key>` `` of `page` names, in order.
fn keys_without_heading<'a>(page: &str, keys: &[&'a str]) -> Vec<&'a str> {
    keys.iter()
        .copied()
        .filter(|key| {
            let heading = format!("### `{key}`");
            !page.lines().any(|line| line.trim_end() == heading)
        })
        .collect()
}

#[test]
fn this_repositorys_page_has_a_section_for_every_key() {
    // req:config/settings-guide-covers-keys@b06928f4
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/guide/settings.md");
    let page = std::fs::read_to_string(path).unwrap();
    let missing = keys_without_heading(&page, &KEYS);
    assert!(
        missing.is_empty(),
        "no `### ` heading in {path} for: {missing:?}"
    );
}

#[test]
fn a_key_left_out_is_reported() {
    // req:config/settings-guide-covers-keys@b06928f4
    let page: String = KEYS
        .iter()
        .filter(|key| **key != "max_builds")
        .map(|key| format!("### `{key}`\n\ntext\n\n"))
        .collect();
    assert_eq!(keys_without_heading(&page, &KEYS), ["max_builds"]);
}

#[test]
fn a_heading_for_a_key_not_yet_parsed_is_no_failure() {
    // req:config/settings-guide-covers-keys@b06928f4
    let mut page: String = KEYS
        .iter()
        .map(|key| format!("### `{key}`\n\ntext\n\n"))
        .collect();
    page.push_str("### `colour`\n\ntext\n");
    assert!(keys_without_heading(&page, &KEYS).is_empty());
}
