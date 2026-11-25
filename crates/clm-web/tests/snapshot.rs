//! The snapshot names its repo, which only the Worker knows (web.worker/snapshot-names-its-repo).

mod common;

use common::*;

#[test]
fn the_snapshot_names_its_repo() {
    // req:web.worker/snapshot-names-its-repo@e8e296e1
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    fake.real_object("o/a");
    let mut want = vec![
        "repo",
        "sha",
        "coverage",
        "sizes",
        "spec",
        "nodes",
        "changes",
        "backlog",
        "resolved",
        "decisions",
        "violations",
    ];
    want.sort();
    for path in [
        "/api/repos/o/a/snapshot",
        "/api/repos/o/a/snapshot",
        "/api/repos/O/A/snapshot?fresh=d1",
    ] {
        let res = fake.handle(get(path, &cookie));
        assert_eq!(res.status, 200, "{path}: {res:?}");
        let body = body_json(&res);
        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(keys, want, "{path}");
        assert_eq!(body["repo"], "o/a", "{path}");
    }
    assert_eq!(
        fake.replicas.asked("o/a").len(),
        2,
        "the second read came from the Worker's cache"
    );
}
