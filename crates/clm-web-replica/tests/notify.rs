//! The object's Web Push sends: what a recompute or a `pull_request` event announces,
//! posted to the repo's `Live` object's `/send` for the live subscriptions of people who
//! can still read the repo.

use clm_web_live::Item;
use clm_web_platform::fake::{Fake, FakeRepo, clm_repo, ledger};
use clm_web_platform::platform::Auth;
use clm_web_replica::{Gone, Permissions, review, waiting};
use futures::executor::block_on;
use serde_json::{Value, json};

fn subscribed(id: &str, writer: &str, endpoint: &str) -> Value {
    json!({
        "id": id, "writer": writer, "at": "2027-01-15T07:00:00.000Z", "kind": "subscribed",
        "endpoint": endpoint, "p256dh": "BKey", "auth": "AuthSecret",
    })
}

fn endpoint(login: &str) -> String {
    format!("https://fcm.googleapis.com/fcm/send/{login}")
}

fn item(slug: &str, kind: &str, title: &str) -> Item {
    Item {
        slug: slug.into(),
        kind: kind.into(),
        title: title.into(),
    }
}

fn auth() -> Auth {
    Auth::Installation("inst-9".into())
}

/// `o/a` with live subscriptions by `alice/web` and `bob/web`; GitHub answers `read` for
/// `alice` and `none` for `bob`. With a VAPID key.
fn setup() -> Fake {
    let mut fake = Fake::default();
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    let mut repo = FakeRepo::new("111", clm_repo());
    ledger(
        &mut repo,
        "alice/web",
        "LA",
        &[subscribed("sa", "alice/web", &endpoint("alice"))],
    );
    ledger(
        &mut repo,
        "bob/web",
        "LB",
        &[subscribed("sb", "bob/web", &endpoint("bob"))],
    );
    repo.permissions.insert("alice".into(), "read".into());
    repo.permissions.insert("bob".into(), "none".into());
    fake.gh.add_repo("o/a", repo);
    fake
}

/// The sends posted to `Live` objects: (object, body).
fn sends(fake: &Fake) -> Vec<(String, Value)> {
    fake.live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.method == "POST" && c.path == "/send")
        .map(|c| (c.name.clone(), serde_json::from_str(&c.body).unwrap()))
        .collect()
}

fn run_waiting(fake: &Fake, previous: Option<&[Item]>, new: &[Item]) {
    run_waiting_with(fake, &mut Permissions::default(), previous, new);
}

fn run_waiting_with(
    fake: &Fake,
    permissions: &mut Permissions,
    previous: Option<&[Item]>,
    new: &[Item],
) {
    block_on(waiting(
        fake,
        &auth(),
        permissions,
        &mut Gone::default(),
        "o/a",
        previous,
        new,
    ));
}

#[test]
fn a_question_lands() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    let new = [item("q", "question", "Which region?")];
    run_waiting(&fake, Some(&[]), &new);
    let sends = sends(&fake);
    assert_eq!(sends.len(), 1, "{sends:?}");
    let (name, body) = &sends[0];
    assert_eq!(name, "o/a");
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "{body}");
    assert_eq!(
        messages[0]["subscription"],
        json!({"endpoint": endpoint("alice"), "p256dh": "BKey", "auth": "AuthSecret"})
    );
    assert_eq!(messages[0]["payload"]["body"], "Which region?");
    assert_eq!(messages[0]["payload"]["url"], "/app/#/o/a/backlog");
}

#[test]
fn a_claimed_writer_in_another_branch() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        // mallory's branch claims alice as a writer and ends her subscription `sa`.
        let ended = json!({
            "id": "um", "writer": "alice/web", "at": "2027-01-15T07:30:00.000Z",
            "kind": "unsubscribed", "subscription": "sa",
        });
        let forged = subscribed("sm", "alice/web", &endpoint("mallory"));
        ledger(repo, "mallory/web", "LM", &[forged, ended]);
        // A branch that isn't a web ledger holds nobody's web subscriptions.
        let other = subscribed("so", "alice/web", &endpoint("other"));
        ledger(repo, "alice/1a2b3c4d", "LO", &[other]);
        repo.permissions.insert("mallory".into(), "none".into());
    }
    run_waiting(&fake, Some(&[]), &[item("q", "question", "Which region?")]);
    let sends = sends(&fake);
    assert_eq!(sends.len(), 1, "{sends:?}");
    let messages = sends[0].1["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0]["subscription"]["endpoint"], endpoint("alice"));
}

#[test]
fn nothing_new_waits() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    run_waiting(&fake, Some(&[]), &[item("i", "idea", "Maybe later")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn no_vapid_key() {
    // req:web.worker.replica/push-waiting@e5643acc
    let mut fake = setup();
    fake.secrets.remove("VAPID_PUBLIC_KEY");
    run_waiting(&fake, Some(&[]), &[item("q", "question", "Which region?")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn no_previous_recompute() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    run_waiting(&fake, None, &[item("q", "question", "Which region?")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn at_most_ten_messages() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let records: Vec<Value> = (0..12)
            .map(|i| {
                subscribed(
                    &format!("s{i:02}"),
                    "alice/web",
                    &endpoint(&format!("a{i}")),
                )
            })
            .collect();
        ledger(repo, "alice/web", "LA", &records);
    }
    run_waiting(&fake, Some(&[]), &[item("q", "question", "Which region?")]);
    let sends = sends(&fake);
    assert_eq!(sends.len(), 1);
    assert_eq!(sends[0].1["messages"].as_array().unwrap().len(), 10);
    assert!(
        fake.logs.borrow().iter().any(|l| l.contains("dropped 2")),
        "{:?}",
        fake.logs.borrow()
    );
}

#[test]
fn permissions_are_remembered() {
    // req:web.worker.replica/push-waiting@e5643acc
    let fake = setup();
    let mut permissions = Permissions::default();
    run_waiting_with(
        &fake,
        &mut permissions,
        Some(&[]),
        &[item("q", "question", "Which region?")],
    );
    let asked = |fake: &Fake| {
        fake.gh
            .calls
            .borrow()
            .iter()
            .filter(|c| c.url.contains("/collaborators/"))
            .count()
    };
    assert_eq!(asked(&fake), 2, "alice and bob");
    // Another question within five minutes asks no one again.
    let first = [item("q", "question", "Which region?")];
    let both = [first[0].clone(), item("z", "question", "Which zone?")];
    run_waiting_with(&fake, &mut permissions, Some(&first), &both);
    assert_eq!(asked(&fake), 2);
    let sends = sends(&fake);
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[1].1["messages"][0]["payload"]["body"], "Which zone?");

    // Eleven web ledger branches, each with a live subscription: GitHub is asked each of
    // the first ten logins once over two notifications, and the eleventh receives nothing.
    let mut fake = Fake::default();
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    let mut repo = FakeRepo::new("111", clm_repo());
    for i in 0..11 {
        let login = format!("u{i:02}");
        let writer = format!("{login}/web");
        let sub = subscribed(&format!("s{i:02}"), &writer, &endpoint(&login));
        ledger(&mut repo, &writer, &format!("L{i:02}"), &[sub]);
        repo.permissions.insert(login, "read".into());
    }
    fake.gh.add_repo("o/a", repo);
    let mut permissions = Permissions::default();
    run_waiting_with(&fake, &mut permissions, Some(&[]), &first);
    run_waiting_with(&fake, &mut permissions, Some(&first), &both);
    let asked: Vec<String> = fake
        .gh
        .calls
        .borrow()
        .iter()
        .filter(|c| c.url.contains("/collaborators/"))
        .map(|c| c.url.clone())
        .collect();
    assert_eq!(asked.len(), 10, "{asked:?}");
    for i in 0..10 {
        let login = format!("/collaborators/u{i:02}/");
        assert_eq!(asked.iter().filter(|u| u.contains(&login)).count(), 1);
    }
    for (_, body) in self::sends(&fake) {
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 10, "{body}");
        assert!(
            !messages
                .iter()
                .any(|m| m["subscription"]["endpoint"] == endpoint("u10")),
            "{body}"
        );
    }
}

fn pull_request(action: &str) -> Value {
    json!({
        "action": action,
        "pull_request": {
            "title": "Add x",
            "draft": false,
            "head": {"ref": "spec/x", "repo": {"full_name": "o/a"}},
        },
        "repository": {"full_name": "o/a", "name": "a", "owner": {"login": "o"}, "default_branch": "main"},
        "installation": {"id": 9},
    })
}

/// `setup`'s repo, with GitHub answering `write` for `alice`.
fn review_setup() -> Fake {
    let fake = setup();
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .permissions
        .insert("alice".into(), "write".into());
    fake
}

#[test]
fn opened_for_review() {
    // req:web.worker.replica/push-review@692d328c
    let fake = review_setup();
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
        &mut Gone::default(),
        "o/a",
        &pull_request("opened"),
    ));
    let sends = sends(&fake);
    assert_eq!(sends.len(), 1, "{sends:?}");
    assert_eq!(sends[0].0, "o/a");
    let messages = sends[0].1["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["subscription"]["endpoint"], endpoint("alice"));
    assert_eq!(messages[0]["payload"]["body"], "spec/x: Add x");
}

#[test]
fn not_for_review() {
    // req:web.worker.replica/push-review@692d328c
    let fake = review_setup();
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
        &mut Gone::default(),
        "o/a",
        &pull_request("closed"),
    ));
    assert!(sends(&fake).is_empty());
}

#[test]
fn review_without_a_vapid_key() {
    // req:web.worker.replica/push-review@692d328c
    let mut fake = review_setup();
    fake.secrets.remove("VAPID_PUBLIC_KEY");
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
        &mut Gone::default(),
        "o/a",
        &pull_request("opened"),
    ));
    assert!(sends(&fake).is_empty());
}

#[test]
fn a_login_with_only_gone_subscriptions_takes_no_place() {
    // req:web.worker.replica/push-waiting@e5643acc
    // Ten logins whose every subscription is gone, and `zed`'s live one: `zed` is among the
    // ten logins the notification considers.
    let mut fake = Fake::default();
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    let mut repo = FakeRepo::new("111", clm_repo());
    let mut gone = Gone::default();
    for login in (0..10).map(|i| format!("a{i:02}")).chain(["zed".into()]) {
        let writer = format!("{login}/web");
        let id = format!("s-{login}");
        ledger(
            &mut repo,
            &writer,
            &format!("L-{login}"),
            &[subscribed(&id, &writer, &endpoint(&login))],
        );
        repo.permissions.insert(login.clone(), "read".into());
        if login != "zed" {
            gone.insert(id);
        }
    }
    fake.gh.add_repo("o/a", repo);
    block_on(waiting(
        &fake,
        &auth(),
        &mut Permissions::default(),
        &mut gone,
        "o/a",
        Some(&[]),
        &[item("q", "question", "Which region?")],
    ));
    let sends = sends(&fake);
    assert_eq!(sends.len(), 1, "{sends:?}");
    let messages = sends[0].1["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0]["subscription"]["endpoint"], endpoint("zed"));
    assert_eq!(gone.len(), 10, "the gone ids are still held, so remembered");
}
