//! A commit's tree listing (`batched-fetches`) and its spec files (`spec-files`).

use std::path::PathBuf;

use clm_web_platform::fake::*;
use clm_web_platform::files;
use clm_web_platform::platform::Auth;

fn auth() -> Auth {
    Auth::User("u".into())
}

fn blob_calls(fake: &Fake) -> usize {
    let calls = fake.gh.calls.borrow();
    calls
        .iter()
        .filter(|c| c.url.contains("/git/blobs/"))
        .count()
}

#[test]
fn a_listing_and_its_truncated_flag() {
    // req:web.worker.platform/batched-fetches@d19c47cf
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", files(&[("a.md", "a"), ("b/c.md", "c")]));
    repo.commits.insert("222".into(), files(&[("a.md", "a")]));
    repo.truncated_trees.insert("222".into());
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let whole = block_on(files::listing(&fake, &auth(), "o/a", "111")).unwrap();
    assert!(!whole.truncated);
    assert!(whole.has("a.md") && whole.has("b/c.md"));
    let cut = block_on(files::listing(&fake, &auth(), "o/a", "222")).unwrap();
    assert!(cut.truncated);
    assert!(
        block_on(files::listing(&fake, &auth(), "o/a", "333")).is_none(),
        "a 404"
    );
}

fn png(i: u8) -> Vec<u8> {
    vec![0x89, b'P', b'N', b'G', 0, 0xff, i]
}

#[test]
fn a_commits_spec_files() {
    // req:web.worker.platform/spec-files@07ebeb2f
    let fake = Fake::default();
    let texts = [
        ("specs/a/SPEC.md", "spec"),
        ("changes/c/change.md", "change"),
        ("docs/guide/start.md", "guide"),
        ("CLAUDE.md", "claude"),
        ("clm.toml", "toml"),
        ("web/.gitattributes", "attrs"),
    ];
    let mut first = files(&texts);
    first.extend(files(&[("src/lib.rs", "code"), ("README.md", "readme")]));
    first.insert("specs/a/assets/logo.png".into(), png(0));
    first.insert("docs/img.png".into(), png(1));
    let mut repo = FakeRepo::new("111", first);
    let second: FileMap = (0..21)
        .map(|i| (format!("specs/a/assets/{i}.png"), png(i + 2)))
        .collect();
    repo.commits.insert("222".into(), second.clone());
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");

    let got = block_on(files::spec_files(&fake, &auth(), "o/a", "111", None)).unwrap();
    let mut want: Vec<(PathBuf, Vec<u8>)> = texts
        .iter()
        .map(|(p, t)| (PathBuf::from(p), t.as_bytes().to_vec()))
        .collect();
    want.push(("specs/a/assets/logo.png".into(), png(0)));
    want.sort();
    assert_eq!(got.into_iter().collect::<Vec<_>>(), want);
    let calls = fake.gh.calls.borrow().len();
    assert!(block_on(files::spec_files(&fake, &auth(), "o/a", "111", None)).is_some());
    assert_eq!(
        fake.gh.calls.borrow().len(),
        calls,
        "the second call asks nothing"
    );

    let blobs = blob_calls(&fake);
    assert!(block_on(files::spec_files(&fake, &auth(), "o/a", "222", None)).is_none());
    assert_eq!(
        blob_calls(&fake) - blobs,
        20,
        "at most 20 blobs in one call"
    );
    let got = block_on(files::spec_files(&fake, &auth(), "o/a", "222", None)).unwrap();
    assert_eq!(
        blob_calls(&fake) - blobs,
        21,
        "the second call goes on from there"
    );
    let want: Vec<(PathBuf, Vec<u8>)> = second.into_iter().map(|(p, b)| (p.into(), b)).collect();
    assert_eq!(got.into_iter().collect::<Vec<_>>(), want);
    let calls = fake.gh.calls.borrow().len();
    assert!(block_on(files::spec_files(&fake, &auth(), "o/a", "222", None)).is_some());
    assert_eq!(
        fake.gh.calls.borrow().len(),
        calls,
        "the third asks nothing"
    );
}

#[test]
fn a_failed_fetch_gives_none() {
    // req:web.worker.platform/spec-files@07ebeb2f
    let fake = Fake::default();
    let mut map = files(&[("specs/a/SPEC.md", "spec")]);
    map.insert("specs/a/assets/logo.png".into(), png(0));
    let mut repo = FakeRepo::new("111", map);
    repo.blob_fails.insert(blob_sha(&png(0)));
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    assert!(block_on(files::spec_files(&fake, &auth(), "o/a", "111", None)).is_none());
    assert!(
        fake.cache
            .borrow()
            .keys()
            .all(|k| !k.starts_with("files3:"))
    );
}

#[test]
fn a_cut_short_text_is_kept_a_day() {
    // req:web.worker.platform/batched-fetches@d19c47cf
    let fake = Fake::default();
    let long = "x".repeat(60_000);
    let repo = FakeRepo::new("111", files(&[("long.md", &long)]));
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let exprs = vec!["111:long.md".to_string()];
    for _ in 0..2 {
        let got = block_on(files::objects(&fake, &auth(), "o/a", "Blobs", &exprs)).unwrap();
        assert_eq!(got, [Some(long.clone())]);
    }
    assert_eq!(blob_calls(&fake), 1, "the blob is read once and kept");
    fake.now.set(NOW + 86_400);
    block_on(files::objects(&fake, &auth(), "o/a", "Blobs", &exprs)).unwrap();
    assert_eq!(blob_calls(&fake), 2, "and read again after a day");
}

#[test]
fn the_tree_given() {
    // req:web.worker.platform/spec-files@07ebeb2f
    let fake = Fake::default();
    let map = files(&[("specs/a/SPEC.md", "spec"), ("CLAUDE.md", "c")]);
    fake.gh.add_repo("o/a", FakeRepo::new("111", map));
    fake.gh.grant("u", "o/a");
    let tree = vec![files::TreeEntry {
        path: "specs/a/SPEC.md".into(),
        sha: blob_sha(b"spec"),
    }];
    let got = block_on(files::spec_files(&fake, &auth(), "o/a", "111", Some(&tree))).unwrap();
    let paths: Vec<&PathBuf> = got.keys().collect();
    assert_eq!(
        paths,
        [&PathBuf::from("specs/a/SPEC.md")],
        "the tree given, not a listing"
    );
    let calls = fake.gh.calls.borrow();
    assert!(calls.iter().all(|c| !c.url.contains("/git/trees/")));
}
