//! The Web Push notifications the object sends after a recompute or a `pull_request` event
//! (`push-waiting`, `push-review`), posted to the repo's `Live` object's `/send`.

use clm_web_live::{Item, MAX_MESSAGES, Notice};
use std::collections::{BTreeMap, BTreeSet};

use clm_web_platform::access::{self, valid_name};
use clm_web_platform::ledger::{self, Sources};
use clm_web_platform::platform::Auth;

use crate::Host;
use serde_json::{Value, json};

/// The most logins one notification considers; any further login counts as having none.
const MAX_LOGINS: usize = 10;

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

/// The `id`s of the `subscribed` records whose endpoint a send answered as gone, which
/// receive no later message. The object keeps them beside the replica, a cache a rebuild
/// from git forgets.
pub type Gone = BTreeSet<String>;

/// After a recompute whose tip moved: posts what `waiting_notice` announces for the backlog
/// items before (`None` when there was no previous recompute) and after. `auth` is the
/// installation's token.
pub async fn waiting<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    gone: &mut Gone,
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
    send(p, auth, permissions, gone, repo, &notice).await;
}

/// On a `pull_request` event: posts what `review_notice` announces, if anything.
pub async fn review<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    gone: &mut Gone,
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
    send(p, auth, permissions, gone, repo, &notice).await;
}

/// A message for one subscription: the `subscribed` record's `id`, and the message.
struct Message {
    id: String,
    endpoint: String,
    body: Value,
}

/// Posts `notice` to `repo`'s `Live` object's `/send`, one message per web subscription of
/// each `clm/ledger/<login>/web` branch, read now, for a login GitHub says has a permission
/// on the repo, at most [`MAX_MESSAGES`], the rest dropped with a log line. A subscription
/// whose endpoint the send answers as gone joins `gone`.
async fn send<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    gone: &mut Gone,
    repo: &str,
    notice: &Notice,
) {
    let Some(sources) = ledger::read(p, auth, repo).await else {
        p.log(&format!("push {repo}: the ledgers can't be read"));
        return;
    };
    let mut messages = recipients(p, auth, permissions, gone, repo, &sources, notice).await;
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
    let bodies: Vec<&Value> = messages.iter().map(|m| &m.body).collect();
    let body = json!({ "messages": bodies }).to_string();
    match p.live_post(repo, "/send", &body).await {
        Some((200..=299, answer)) => {
            let answer: Value = serde_json::from_str(&answer).unwrap_or(Value::Null);
            for endpoint in answer["gone"].as_array().into_iter().flatten() {
                let ended = messages.iter().filter(|m| *endpoint == *m.endpoint);
                gone.extend(ended.map(|m| m.id.clone()));
            }
        }
        other => p.log(&format!("push {repo}: send failed: {other:?}")),
    }
}

/// A message for each live subscription of each web ledger branch's login that may read
/// the repo, the branch's records folded alone (`web-subscriptions-by-branch`), but those
/// in `gone`. An id no web branch holds any longer leaves `gone`.
async fn recipients<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    gone: &mut Gone,
    repo: &str,
    sources: &Sources,
    notice: &Notice,
) -> Vec<Message> {
    let mut held = BTreeSet::new();
    let mut logins = 0;
    let mut messages = Vec::new();
    for (login, fold) in ledger::web_folds(sources) {
        held.extend(fold.subscriptions.keys().cloned());
        // A login with nothing left to send to takes none of the logins' places.
        if fold.subscriptions.keys().all(|id| gone.contains(id))
            || !may_read(p, auth, permissions, repo, login, &mut logins).await
        {
            continue;
        }
        for (id, record) in &fold.subscriptions {
            // A subscription's `endpoint`, `p256dh` and `auth`, as its ledger line holds them.
            let line = serde_json::to_value(record).unwrap_or(Value::Null);
            if line["kind"] != "subscribed" || gone.contains(id) {
                continue;
            }
            messages.push(Message {
                id: id.clone(),
                endpoint: line["endpoint"].as_str().unwrap_or_default().into(),
                body: json!({
                    "subscription": {
                        "endpoint": line["endpoint"],
                        "p256dh": line["p256dh"],
                        "auth": line["auth"],
                    },
                    "payload": notice,
                }),
            });
        }
    }
    gone.retain(|id| held.contains(id));
    messages
}

/// Whether GitHub answers `login`'s permission on `repo` as anything but `none`; a 404 is
/// no. The answer is remembered for 5 minutes. Once `logins` reaches [`MAX_LOGINS`], a
/// further login counts as no.
async fn may_read<P: Host>(
    p: &P,
    auth: &Auth,
    permissions: &mut Permissions,
    repo: &str,
    login: &str,
    logins: &mut usize,
) -> bool {
    if *logins >= MAX_LOGINS {
        p.log(&format!(
            "push {repo}: {login} skipped, {MAX_LOGINS} logins considered"
        ));
        return false;
    }
    *logins += 1;
    if let Some(known) = permissions.get(login, p.now()) {
        return known != "none";
    }
    let Some(known) = access::permission_direct(p, auth, repo, login).await else {
        return false;
    };
    permissions.put(login, &known, p.now());
    known != "none"
}
