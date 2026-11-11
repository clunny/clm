//! `Assets::fetch_asset` answers a request whole; `asset` is text over it (`asset-responses`).

use clm_web_platform::fake::*;
use clm_web_platform::http::Request;
use clm_web_platform::platform::Assets;

fn get(path: &str) -> Request {
    Request {
        method: "GET".into(),
        path: path.into(),
        ..Request::default()
    }
}

#[test]
fn a_binary_asset() {
    // req:web.worker.platform/asset-responses@6084735c
    let fake = Fake::default();
    let bytes = vec![0xff, 0xfe, 0x00, 0x80];
    fake.assets.borrow_mut().insert(
        "/fonts/inter.woff2".into(),
        ("font/woff2".into(), bytes.clone()),
    );
    let res = block_on(fake.fetch_asset(&get("/fonts/inter.woff2")));
    assert_eq!(res.status, 200);
    assert_eq!(res.header("content-type"), Some("font/woff2"));
    assert_eq!(res.body, bytes);
    assert_eq!(block_on(fake.asset("/fonts/inter.woff2")), None);
    let res = block_on(fake.fetch_asset(&get("/guide/nothing-here")));
    assert_eq!(res.status, 404);
    assert!(res.body.is_empty());
    assert_eq!(block_on(fake.asset("/guide/nothing-here")), None);
}

#[test]
fn text_through_the_provided_method() {
    // req:web.worker.platform/asset-responses@6084735c
    let fake = Fake::default();
    assert_eq!(
        block_on(fake.asset("/_clm/invited.json")).as_deref(),
        Some(r#"["clunny"]"#)
    );
}
