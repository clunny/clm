//! Who may read a repo (`access-check`), and a login's permission on it (`permission`).

use clm_web_platform::access::{self, token_hash};
use clm_web_platform::fake::*;
use clm_web_platform::gh;
use clm_web_platform::platform::Auth;

fn asked(fake: &Fake, path: &str) -> usize {
    let url = format!("{}{path}", gh::API);
    fake.gh
        .calls
        .borrow()
        .iter()
        .filter(|c| c.url == url)
        .count()
}

#[test]
fn who_may_read_a_repo() {
    // req:web.worker.platform/access-check@af49f3ca
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.default_branch = "trunk".into();
    fake.gh.add_repo("O/A", repo);
    fake.gh.grant("u", "O/A");
    let mut failing = FakeRepo::new("222", clm_repo());
    failing.lookup_fails = true;
    fake.gh.add_repo("q/c", failing);
    fake.gh.grant("u", "q/c");

    let acc = block_on(access::check(&fake, "u", "o", "a")).unwrap();
    assert_eq!(
        (acc.owner(), acc.repo(), acc.default_branch()),
        ("O", "A", "trunk")
    );
    assert_eq!(acc.auth(), &Auth::User("u".into()));
    let direct = block_on(access::checked_direct(&fake, "u", "o", "a"))
        .unwrap()
        .unwrap();
    assert_eq!(
        (direct.full_name(), direct.default_branch()),
        ("O/A".to_string(), "trunk")
    );

    for (owner, repo) in [("r", "none"), ("o", ".."), ("o", "a b"), ("", "a")] {
        let before = fake.gh.calls.borrow().len();
        assert!(block_on(access::check(&fake, "u", owner, repo)).is_none());
        assert_eq!(
            block_on(access::checked_direct(&fake, "u", owner, repo)).map(|a| a.is_some()),
            Ok(false),
            "{owner}/{repo}"
        );
        let made = fake.gh.calls.borrow().len() - before;
        assert_eq!(made, usize::from(owner == "r") * 2, "{owner}/{repo}");
    }

    assert!(block_on(access::check(&fake, "u", "q", "c")).is_none());
    assert_eq!(
        block_on(access::checked_direct(&fake, "u", "q", "c")).map(|a| a.is_some()),
        Err(gh::Failed)
    );

    // `check` remembers GitHub's answer for 5 minutes, and never a failure.
    let (o_a, q_c) = (asked(&fake, "/repos/o/a"), asked(&fake, "/repos/q/c"));
    fake.now.set(NOW + 240);
    assert!(block_on(access::check(&fake, "u", "o", "a")).is_some());
    assert!(block_on(access::check(&fake, "u", "q", "c")).is_none());
    assert_eq!(
        asked(&fake, "/repos/o/a"),
        o_a,
        "the second o/a asks nothing"
    );
    assert_eq!(asked(&fake, "/repos/q/c"), q_c + 1);
    fake.now.set(NOW + 300);
    assert!(block_on(access::check(&fake, "u", "o", "a")).is_some());
    assert!(block_on(access::check(&fake, "u", "q", "c")).is_none());
    assert_eq!(asked(&fake, "/repos/o/a"), o_a + 1, "the third asks again");
    assert_eq!(asked(&fake, "/repos/q/c"), q_c + 2);

    let cache = fake.cache.borrow();
    assert!(cache.keys().any(|k| k.contains(&token_hash("u"))));
    let hash = token_hash("u");
    assert!(hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    assert!(
        cache.keys().all(|k| k
            .replace(&hash, "")
            .split([':', '/'])
            .all(|part| part != "u")),
        "no key holds the token: {:?}",
        cache.keys().collect::<Vec<_>>()
    );
}

#[test]
fn checked_direct_tells_a_401_and_an_odd_name_from_a_refusal() {
    // req:web.worker.platform/access-check@af49f3ca
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    // A token the fake doesn't know is answered 404, as GitHub answers a repo it hides.
    assert_eq!(
        block_on(access::checked_direct(&fake, "x", "o", "a")).map(|a| a.is_some()),
        Ok(false)
    );
    {
        let mut repos = fake.gh.repos.borrow_mut();
        repos.get_mut("o/a").unwrap().fail_paths.push(String::new());
    }
    assert_eq!(
        block_on(access::checked_direct(&fake, "u", "o", "a")).map(|a| a.is_some()),
        Err(gh::Failed),
        "a 502"
    );
}

#[test]
fn a_logins_permission() {
    // req:web.worker.platform/permission@dd587d01
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.permissions.insert("alice".into(), "write".into());
    repo.permission_fails.insert("carol".into());
    fake.gh.add_repo("o/a", repo);
    let auth = Auth::Installation("inst-9".into());
    let path = |login: &str| format!("/repos/o/a/collaborators/{login}/permission");
    let logins = ["alice", "bob", "carol"];
    let want = [Some("write"), Some("none"), None];
    let round = || {
        for (login, want) in logins.iter().zip(want) {
            let got = block_on(access::permission(&fake, &auth, "o/a", login));
            assert_eq!(got.as_deref(), want, "{login}");
        }
    };
    let counts = || logins.map(|l| asked(&fake, &path(l)));
    round();
    assert_eq!(counts(), [1, 1, 1]);
    fake.now.set(NOW + 240);
    round();
    assert_eq!(counts(), [1, 1, 2], "the second round asks only for carol");
    fake.now.set(NOW + 300);
    round();
    assert_eq!(counts(), [2, 2, 3], "the third asks for all three");
    for _ in 0..2 {
        for (login, want) in logins.iter().zip(want) {
            let got = block_on(access::permission_direct(&fake, &auth, "o/a", login));
            assert_eq!(got.as_deref(), want, "{login}");
        }
    }
    assert_eq!(counts(), [4, 4, 5], "permission_direct asks every time");
}
