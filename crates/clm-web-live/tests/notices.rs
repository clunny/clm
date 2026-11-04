use clm_web_live::{Item, Notice, review_notice, waiting_notice};
use serde_json::{Value, json};

fn item(slug: &str, kind: &str, title: &str) -> Item {
    Item {
        slug: slug.into(),
        kind: kind.into(),
        title: title.into(),
    }
}

#[test]
fn two_questions_land() {
    // req:web.live/notices@30857aa9
    let previous = [item("i", "idea", "An idea")];
    let new = [
        item("i", "idea", "An idea"),
        item("q", "question", "Which region?"),
        item("h", "human-task", "Set the key"),
    ];
    let notice = waiting_notice("o/a", &previous, &new).unwrap();
    assert_eq!(
        serde_json::to_value(&notice).unwrap(),
        json!({"title": "o/a: waiting on you", "body": "Which region? and 1 more", "url": "/app/#/o/a/backlog"})
    );
}

#[test]
fn one_waiting_item_is_named_alone() {
    // req:web.live/notices@30857aa9
    let new = [item("q", "question", "Which region?")];
    let notice = waiting_notice("o/a", &[], &new).unwrap();
    assert_eq!(notice.body, "Which region?");
}

#[test]
fn nothing_new_waits() {
    // req:web.live/notices@30857aa9
    let previous = [item("q", "question", "Which region?")];
    let new = [
        item("q", "question", "Which region?"),
        item("i", "idea", "An idea"),
    ];
    assert_eq!(waiting_notice("o/a", &previous, &new), None);
}

fn event(action: &str, head: &str, repo: &str, draft: bool) -> Value {
    json!({
        "action": action,
        "pull_request": {
            "title": "Add x",
            "draft": draft,
            "head": {"ref": head, "repo": {"full_name": repo}},
        },
    })
}

#[test]
fn opened_for_review() {
    // req:web.live/notices@30857aa9
    let notice = review_notice("o/a", &event("opened", "spec/x", "o/a", false));
    assert_eq!(
        notice,
        Some(Notice {
            title: "o/a: open for review".into(),
            body: "spec/x: Add x".into(),
            url: "/app/#/o/a/prs/spec%2Fx".into(),
        })
    );
    for action in ["reopened", "ready_for_review"] {
        assert!(review_notice("o/a", &event(action, "spec/x", "o/a", false)).is_some());
    }
    for other in [
        event("closed", "spec/x", "o/a", false),
        event("opened", "impl/y", "o/a", false),
        event("opened", "spec/x", "fork/a", false),
        event("opened", "spec/x", "o/a", true),
        json!({"action": "opened"}),
    ] {
        assert_eq!(review_notice("o/a", &other), None, "{other}");
    }
}

#[test]
fn branch_is_one_encoded_segment() {
    // req:web.live/notices@30857aa9
    let notice = review_notice("o/a", &event("opened", "spec/a b/ü", "o/a", false)).unwrap();
    assert_eq!(notice.url, "/app/#/o/a/prs/spec%2Fa%20b%2F%C3%BC");
}
