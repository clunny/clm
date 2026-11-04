//! The Web Push notifications the object sends after a recompute or a `pull_request` event
//! (`push-waiting`, `push-review`), posted to the repo's `Live` object's `/send`.

use clm_web_live::{Item, Notice};
use std::collections::BTreeMap;

use clm_web_platform::access::{self, valid_name};
use clm_web_platform::ledger::{self, Sources};
use clm_web_platform::platform::Auth;

use crate::Host;
use serde_json::{Value, json};

/// The most messages one send carries (`web.live`'s `/send` takes no more).
const MAX_MESSAGES: usize = 10;
/// The most logins whose permission one notification asks GitHub for.
const MAX_ASKS: usize = 10;

/// How long a login's permission is remembered.
const PERMISSION_TTL: u64 = 300;

/// Each login's permission on a repo as GitHub answered it, kept in the object's memory for
/// five minutes, as `access::permission` keeps it for the Worker.
#[derive(Debug, Default)]
pub struct Permissions(BTreeMap<String, (String, u64)>);

impl Permissions {
    fn get(&self, login: &str, now: u64) -> Option<&str> {
        let (known, until) = self.0.get(login)?;
        (now < *until).then_some(known.as_str())
    }

    fn put(&mut self, login: &str, known: &str, now: u64) {
        self.0
            .insert(login.into(), (known.into(), now + PERMISSION_TTL));
    }
}

/// After a recompute whose tip moved: posts what `waiting_notice` announces for the backlog
/// items before (`None` when there was no previous recompute) and after. `auth` is the
/// installation's token.
pub async fn waiting<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    previous: Option<&[Item]>,
    new: &[Item],
) {
    if p.secret("VAPID_PUBLIC_KEY").is_none() {
        return;
    }
    let Some(notice) =
        previous.and_then(|previous| clm_web_live::waiting_notice(repo, previous, new))
    else {
        return;
    };
    send(p, auth, permissions, repo, &notice).await;
}

/// On a `pull_request` event: posts what `review_notice` announces, if anything.
pub async fn review<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    event: &Value,
) {
    let named = repo
        .split_once('/')
        .is_some_and(|(o, r)| valid_name(o) && valid_name(r));
    let Some(notice) = clm_web_live::review_notice(repo, event).filter(|_| named) else {
        return;
    };
    if p.secret("VAPID_PUBLIC_KEY").is_none() {
        return;
    }
    send(p, auth, permissions, repo, &notice).await;
}

/// Posts `notice` to `repo`'s `Live` object's `/send`, one message per web subscription of
/// each `clm/ledger/<login>/web` branch, read now, for a login GitHub says has a permission
/// on the repo, at most [`MAX_MESSAGES`], the rest dropped with a log line.
async fn send<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    notice: &Notice,
) {
    let Some(sources) = ledger::read(p, auth, repo).await else {
        p.log(&format!("push {repo}: the ledgers can't be read"));
        return;
    };
    let mut messages = recipients(p, auth, permissions, repo, &sources, notice).await;
    if messages.is_empty() {
        return;
    }
    if messages.len() > MAX_MESSAGES {
        let dropped = messages.len() - MAX_MESSAGES;
        p.log(&format!(
            "push {repo}: dropped {dropped} of {} messages",
            messages.len()
        ));
        messages.truncate(MAX_MESSAGES);
    }
    let body = json!({ "messages": messages }).to_string();
    match p.live_post(repo, "/send", &body).await {
        Some((200..=299, answer)) => {
            let answer: Value = serde_json::from_str(&answer).unwrap_or(Value::Null);
            for gone in answer["gone"].as_array().into_iter().flatten() {
                p.log(&format!("push {repo}: gone {gone}"));
            }
        }
        other => p.log(&format!("push {repo}: send failed: {other:?}")),
    }
}

/// A message for each live subscription of each web ledger branch's login that may read
/// the repo, the branch's records folded alone (`web-subscriptions-by-branch`).
async fn recipients<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    sources: &Sources,
    notice: &Notice,
) -> Vec<Value> {
    let mut asks = 0;
    let mut messages = Vec::new();
    for (login, fold) in ledger::web_folds(sources) {
        if fold.subscriptions.is_empty()
            || !may_read(p, auth, permissions, repo, login, &mut asks).await
        {
            continue;
        }
        for record in fold.subscriptions.values() {
            // A subscription's `endpoint`, `p256dh` and `auth`, as its ledger line holds them.
            let line = serde_json::to_value(record).unwrap_or(Value::Null);
            if line["kind"] == "subscribed" {
                messages.push(json!({
                    "subscription": {
                        "endpoint": line["endpoint"],
                        "p256dh": line["p256dh"],
                        "auth": line["auth"],
                    },
                    "payload": notice,
                }));
            }
        }
    }
    messages
}

/// Whether GitHub answers `login`'s permission on `repo` as anything but `none`; a 404 is
/// no. The answer is remembered for 5 minutes; once `asks` reaches [`MAX_ASKS`], a login
/// not remembered counts as no.
async fn may_read<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    login: &str,
    asks: &mut usize,
) -> bool {
    if let Some(known) = permissions.get(login, p.now()) {
        return known != "none";
    }
    if *asks >= MAX_ASKS {
        p.log(&format!(
            "push {repo}: {login} skipped, {MAX_ASKS} logins asked"
        ));
        return false;
    }
    *asks += 1;
    let Some(known) = access::permission_direct(p, auth, repo, login).await else {
        return false;
    };
    permissions.put(login, &known, p.now());
    known != "none"
}
