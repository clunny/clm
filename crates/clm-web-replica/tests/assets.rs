//! The binary files whose hash the replica keeps, fetched whole through the blob API, and
//! storage an object that didn't hash them wrote (`fetches-binary-assets`).

mod common;

use clm_web_platform::fake::blob_sha;
use common::*;
use futures::executor::block_on;
use serde_json::Value;

const LOGO: &[u8] = &[0x89, b'P', b'N', b'G', 0, 0xff, 0xfe, 1];
const LOST: &[u8] = &[0x89, b'P', b'N', b'G', 0, 0xff, 0xfe, 2];
const BLOB: &[u8] = &[0, 0xff, 0xfe, 3];

/// The hash `clm` gives `LOGO` as an asset: a binary file's is the first 8 bytes of its
/// SHA-256, in hex (`clm_model::asset_hash`).
const LOGO_HASH: &str = "223b3be326427335";

fn node_b(snapshot: &Value) -> &Value {
    snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "b")
        .unwrap()
}

#[test]
fn a_logo() {
    // req:web.worker.replica/fetches-binary-assets@c64985ae
    let mut all = repo_files(&[]);
    all.insert("specs/b/assets/logo.png".into(), LOGO.to_vec());
    // 11 MiB, not text.
    all.insert("specs/b/assets/big.png".into(), vec![0xff; 11 << 20]);
    all.insert("specs/b/assets/lost.png".into(), LOST.to_vec());
    all.insert("src/blob.bin".into(), BLOB.to_vec());
    let fake = fake_with("111", all);
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .blob_fails
        .insert(blob_sha(LOST));

    let (mut object, mem, _slot) = built(&fake);
    let blobs: Vec<String> = urls(&fake, 0)
        .into_iter()
        .filter_map(|u| u.split("/git/blobs/").nth(1).map(String::from))
        .collect();
    let mut asked = blobs.clone();
    asked.sort();
    let mut expected = vec![blob_sha(LOGO), blob_sha(LOST)];
    expected.sort();
    assert_eq!(
        asked, expected,
        "the blob API was asked for logo.png and lost.png only"
    );

    let first = view(&fake, &mut object, "snapshot");
    let b = node_b(&first["view"]);
    let assets = b["assets"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["logo.png"],
        "{b}"
    );
    assert_eq!(assets["logo.png"], LOGO_HASH);

    // A second object, whose storage an object that didn't hash binary assets wrote: the
    // replica as saved, beside a record that names no version.
    let old = Mem::default();
    old.0.replace(mem.0.borrow().clone());
    {
        let mut map = old.0.borrow_mut();
        let mut meta: Value = serde_json::from_slice(&map["meta"]).unwrap();
        assert!(meta.as_object_mut().unwrap().remove("version").is_some());
        map.insert("meta".into(), serde_json::to_vec(&meta).unwrap());
    }
    let slot = Slot::default();
    let mut second = clm_web_replica::Object::load("o/a", old.clone(), slot.clone());
    let answer = block_on(second.view(&fake, "snapshot", false));
    assert_eq!(
        (answer.status, answer.body.as_str()),
        (503, r#"{"preparing":0}"#)
    );
    run_alarms(&fake, &mut second, &slot);
    let again = view(&fake, &mut second, "snapshot");
    assert_eq!(again["view"], first["view"]);
}
