//! The notification payloads the Worker's webhook sends.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A backlog item, as the snapshot holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub slug: String,
    pub kind: String,
    pub title: String,
}

/// What a notification shows, and where clicking it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub title: String,
    pub body: String,
    pub url: String,
}

/// What a push to the default branch announces: the `question` and `human-task` items of `new`
/// whose slug `previous` doesn't hold, as one notice, or none when there are none.
pub fn waiting_notice(repo: &str, previous: &[Item], new: &[Item]) -> Option<Notice> {
    let mut waiting = new.iter().filter(|i| {
        matches!(i.kind.as_str(), "question" | "human-task")
            && !previous.iter().any(|p| p.slug == i.slug)
    });
    let first = waiting.next()?;
    let more = waiting.count();
    let body = if more == 0 {
        first.title.clone()
    } else {
        format!("{} and {more} more", first.title)
    };
    Some(Notice {
        title: format!("{repo}: waiting on you"),
        body,
        url: format!("/app/#/{repo}/backlog"),
    })
}

/// What a `pull_request` webhook announces: a spec branch of `repo` itself opened, reopened
/// or marked ready for review, and not a draft; none for anything else.
pub fn review_notice(repo: &str, event: &Value) -> Option<Notice> {
    let action = event["action"].as_str()?;
    if !matches!(action, "opened" | "reopened" | "ready_for_review") {
        return None;
    }
    let pr = &event["pull_request"];
    if pr["draft"].as_bool() != Some(false) {
        return None;
    }
    let head = &pr["head"];
    let branch = head["ref"].as_str()?;
    if !branch.starts_with("spec/") || head["repo"]["full_name"].as_str() != Some(repo) {
        return None;
    }
    let title = pr["title"].as_str()?;
    Some(Notice {
        title: format!("{repo}: open for review"),
        body: format!("{branch}: {title}"),
        url: format!("/app/#/{repo}/prs/{}", segment(branch)),
    })
}

/// `s` percent-encoded as one URL path segment: every byte but RFC 3986's unreserved ones,
/// as `clm-web`'s `http::percent_encode` does; this crate can't depend on `clm-web`, which
/// links it.
fn segment(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
