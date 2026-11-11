//! The Worker adapter's asset fetch, over a fake `ASSETS` binding (web.worker/asset-adapter).

use std::cell::RefCell;
use std::collections::BTreeMap;

use clm_web::assets::{AssetBinding, AssetCache, BindingAnswer, fetch_asset};
use clm_web_platform::http::Request;
use futures::executor::block_on;

type Asked = (String, String, String, BTreeMap<String, String>);

/// A binding that answers from `answers` by path, or fails, and records what it was asked.
#[derive(Default)]
struct Fake {
    answers: BTreeMap<String, BindingAnswer>,
    fetch_fails: bool,
    asked: RefCell<Vec<Asked>>,
}

impl AssetBinding for Fake {
    async fn fetch(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &BTreeMap<String, String>,
    ) -> Result<BindingAnswer, ()> {
        self.asked
            .borrow_mut()
            .push((method.into(), path.into(), query.into(), headers.clone()));
        if self.fetch_fails {
            return Err(());
        }
        Ok(self.answers.get(path).cloned().unwrap_or(BindingAnswer {
            status: 404,
            headers: vec![],
            body: Ok(vec![]),
        }))
    }
}

fn request(method: &str, path: &str, query: &str, headers: &[(&str, &str)]) -> Request {
    Request {
        method: method.into(),
        path: path.into(),
        query: query.into(),
        headers: headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: vec![],
        origin: "https://clm-app.dev".into(),
    }
}

fn ok(bytes: &[u8]) -> BindingAnswer {
    BindingAnswer {
        status: 200,
        headers: vec![("content-type".into(), "text/plain".into())],
        body: Ok(bytes.to_vec()),
    }
}

#[test]
fn passed_through() {
    // req:web.worker/asset-adapter@8c4f0569
    let mut fake = Fake::default();
    fake.answers.insert(
        "/fonts/inter.woff2".into(),
        BindingAnswer {
            status: 304,
            headers: vec![("etag".into(), "\"x\"".into())],
            body: Ok(vec![]),
        },
    );
    let cache = AssetCache::default();
    let req = request(
        "HEAD",
        "/fonts/inter.woff2",
        "v=1",
        &[
            ("if-none-match", "\"x\""),
            ("cookie", "s=1"),
            ("authorization", "Bearer t"),
        ],
    );
    let res = block_on(fetch_asset(Some(&fake), &cache, &req));
    let asked = fake.asked.borrow();
    assert_eq!(asked.len(), 1);
    let (method, path, query, headers) = &asked[0];
    assert_eq!(
        (method.as_str(), path.as_str(), query.as_str()),
        ("HEAD", "/fonts/inter.woff2", "v=1")
    );
    assert_eq!(
        headers,
        &BTreeMap::from([("if-none-match".to_string(), "\"x\"".to_string())])
    );
    assert_eq!(res.status, 304);
    assert_eq!(res.headers, vec![("etag".to_string(), "\"x\"".to_string())]);
    assert!(res.body.is_empty());
}

#[test]
fn no_binding_or_failed_answer() {
    // req:web.worker/asset-adapter@8c4f0569
    let req = request("GET", "/app/index.html", "", &[]);
    let none = block_on(fetch_asset(None::<&Fake>, &AssetCache::default(), &req));
    assert_eq!(none.status, 503);

    let failing = Fake {
        fetch_fails: true,
        ..Fake::default()
    };
    let res = block_on(fetch_asset(Some(&failing), &AssetCache::default(), &req));
    assert_eq!(res.status, 502);

    let mut unreadable = Fake::default();
    unreadable.answers.insert(
        "/app/index.html".into(),
        BindingAnswer {
            status: 200,
            headers: vec![],
            body: Err(()),
        },
    );
    let res = block_on(fetch_asset(Some(&unreadable), &AssetCache::default(), &req));
    assert_eq!(res.status, 502);
}

#[test]
fn kept_per_isolate() {
    // req:web.worker/asset-adapter@8c4f0569
    let mut fake = Fake::default();
    fake.answers
        .insert("/_clm/invited.json".into(), ok(b"[\"a\"]"));
    fake.answers.insert("/app/index.html".into(), ok(b"<html>"));
    let cache = AssetCache::default();
    let get = |path: &str, query: &str| {
        block_on(fetch_asset(
            Some(&fake),
            &cache,
            &request("GET", path, query, &[]),
        ))
    };
    for _ in 0..2 {
        for (path, status, bytes) in [
            ("/_clm/invited.json", 200, &b"[\"a\"]"[..]),
            ("/app/index.html", 200, &b"<html>"[..]),
            ("/_clm/missing.json", 404, &b""[..]),
        ] {
            let res = get(path, "");
            assert_eq!((res.status, res.body.as_slice()), (status, bytes), "{path}");
        }
    }
    assert_eq!(get("/_clm/invited.json", "x=1").body, b"[\"a\"]");
    let count = |path: &str, query: &str| {
        fake.asked
            .borrow()
            .iter()
            .filter(|a| a.1 == path && a.2 == query)
            .count()
    };
    assert_eq!(count("/_clm/invited.json", ""), 1);
    assert_eq!(count("/_clm/invited.json", "x=1"), 1);
    assert_eq!(count("/app/index.html", ""), 2);
    assert_eq!(count("/_clm/missing.json", ""), 2);
}
