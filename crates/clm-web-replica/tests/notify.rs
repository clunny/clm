//! The object's Web Push sends: what a recompute or a `pull_request` event announces,
//! posted to the repo's `Live` object's `/send` for the live subscriptions of people who
//! can still read the repo.

use clm_web_live::Item;
use clm_web_platform::fake::{Fake, FakeRepo, clm_repo, ledger};
use clm_web_platform::platform::Auth;
use clm_web_replica::{Permissions, review, waiting};
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
    block_on(waiting(fake, &auth(), permissions, "o/a", previous, new));
}

#[test]
fn a_question_lands() {
    // req:web.worker.replica/push-waiting@8e710ff8
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
    // req:web.worker.replica/push-waiting@8e710ff8
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
    // req:web.worker.replica/push-waiting@8e710ff8
    let fake = setup();
    run_waiting(&fake, Some(&[]), &[item("i", "idea", "Maybe later")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn no_vapid_key() {
    // req:web.worker.replica/push-waiting@8e710ff8
    let mut fake = setup();
    fake.secrets.remove("VAPID_PUBLIC_KEY");
    run_waiting(&fake, Some(&[]), &[item("q", "question", "Which region?")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn no_previous_recompute() {
    // req:web.worker.replica/push-waiting@8e710ff8
    let fake = setup();
    run_waiting(&fake, None, &[item("q", "question", "Which region?")]);
    assert!(sends(&fake).is_empty());
}

#[test]
fn at_most_ten_messages() {
    // req:web.worker.replica/push-waiting@8e710ff8
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
    // req:web.worker.replica/object-class@2ce37c07
    // req:web.worker.replica/push-waiting@8e710ff8
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
    // req:web.worker.replica/push-review@0b5b98e1
    let fake = review_setup();
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
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
    // req:web.worker.replica/push-review@0b5b98e1
    let fake = review_setup();
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
        "o/a",
        &pull_request("closed"),
    ));
    assert!(sends(&fake).is_empty());
}

#[test]
fn review_without_a_vapid_key() {
    // req:web.worker.replica/push-review@0b5b98e1
    let mut fake = review_setup();
    fake.secrets.remove("VAPID_PUBLIC_KEY");
    block_on(review(
        &fake,
        &auth(),
        &mut Permissions::default(),
        "o/a",
        &pull_request("opened"),
    ));
    assert!(sends(&fake).is_empty());
}
