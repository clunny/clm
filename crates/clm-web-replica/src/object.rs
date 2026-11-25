//! The repo's `Replica` object: `replica`'s core run on a GitHub source and the object's own
//! storage (`web-repo-replica`). It answers the Worker's three internal routes, `POST
//! /event`, `GET /view/<name>` and `POST /write`, and runs its alarm, which recomputes and
//! tells the open pages. Everything it reads from GitHub goes through `source`, within the
//! Free plan's subrequests per invocation.

use std::collections::{BTreeMap, BTreeSet};

use clm_ledger::unix_time;
use clm_replica::{
    Answer as Viewed, Change, Entry, Event, File, Pull, Refusal, Replica, Store, Validated, Write,
};
use clm_web_live::Item;
use clm_web_platform::access::{self, Access};
use clm_web_platform::install;
use clm_web_platform::platform::Auth;

use crate::Host;
use clm_web_platform::ledger;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::notify::{Gone, Permissions};
use crate::removal::{Storage, on_removal};
use crate::source::{self, Budget};

/// The batching delay: an applied event sets the alarm this many seconds ahead.
const BATCH: u64 = 60;
/// How long an installation token is kept: GitHub's last an hour, and this is five minutes
/// short of that.
const TOKEN_KEEP: u64 = 3300;
/// How long a read keeps the repo's pages open.
const OPEN: u64 = 300;
/// The seconds between resyncs while the pages are open.
const RESYNC: u64 = 600;
/// The store key of what the object keeps beside the replica.
const META: &str = "meta";
/// A read rewrites its time in the store only when it has moved this far.
const READ_WRITE: u64 = 60;
/// The version of the object that writes its record: 1 hashes binary assets. Storage whose
/// record names another, or none, is dropped on waking and built again
/// (`fetches-binary-assets`).
const VERSION: u32 = 1;

/// The object's alarm, in Unix seconds.
pub trait Alarm {
    fn get(&self) -> Option<u64>;
    fn set(&mut self, at: u64);
    fn clear(&mut self);
}

/// What a route answers: a status and a JSON body, empty for none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub body: String,
}

impl Answer {
    fn json(status: u16, body: &Value) -> Answer {
        Answer {
            status,
            body: body.to_string(),
        }
    }

    /// An answer with no body.
    pub fn empty(status: u16) -> Answer {
        Answer {
            status,
            body: String::new(),
        }
    }
}

/// What the object keeps beside the replica: what the source tells it and what the replica
/// doesn't expose. A field it lacks, as one an older object wrote does, reads as its default.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Meta {
    /// The version of the object that wrote the record; none, read as 0, before versions.
    version: u32,
    installation: Option<u64>,
    default_branch: Option<String>,
    /// The default branch's commit the replica holds; none until the first build lists it.
    tip: Option<String>,
    /// The refs the replica holds, by name, with their tips.
    refs: BTreeMap<String, String>,
    /// The held refs whose change files or code key couldn't be read, to read again.
    unread: BTreeSet<String>,
    /// The ledger branches it holds, with their tips.
    ledgers: BTreeMap<String, String>,
    pulls: BTreeMap<u64, Pull>,
    /// The latest delivery applied since the last notice.
    delivery: Option<String>,
    last_read: u64,
    last_resync: u64,
    /// When the views next change without a change.
    next: Option<u64>,
    /// The backlog items, and the commit, of the last recompute a notice looked at.
    items: Option<Vec<(String, String, String)>>,
    items_sha: Option<String>,
    /// The subscriptions whose endpoint a send answered as gone (`push-waiting`).
    gone: Gone,
}

/// Deletes everything an object stores.
struct Wipe<'a, S>(&'a mut S);

impl<S: Store> Storage for Wipe<'_, S> {
    fn delete_all(&mut self) {
        self.0.delete_all();
    }
}

pub struct Object<S, A> {
    name: String,
    replica: Replica,
    meta: Meta,
    store: S,
    alarm: A,
    /// The tree at `meta.tip`, kept while the object is awake.
    listing: Option<BTreeMap<String, Entry>>,
    /// The read time the store holds.
    stored_read: u64,
    /// An uninstall deleted what the object held (`uninstall-deletes`).
    removed: bool,
    /// The installation, its token and the time the token is kept until, while the object is awake.
    token: Option<(u64, String, u64)>,
    /// Each login's permission, as GitHub answered it, while the object is awake.
    permissions: Permissions,
}

impl<S: Store, A: Alarm> Object<S, A> {
    /// The object named `name`, `<owner>/<repo>`, with what `store` holds.
    pub fn load(name: &str, mut store: S, alarm: A) -> Self {
        let stored: Option<Meta> = store
            .get(META)
            .and_then(|b| serde_json::from_slice(&b).ok());
        let meta = match stored {
            Some(meta) if meta.version >= VERSION => meta,
            // Written by an object that didn't hash binary assets, or no record that reads:
            // built again from nothing.
            _ => {
                Wipe(&mut store).delete_all();
                Meta::default()
            }
        };
        Object {
            name: name.into(),
            replica: Replica::load(&store),
            stored_read: meta.last_read,
            removed: false,
            meta,
            store,
            alarm,
            listing: None,
            token: None,
            permissions: Permissions::default(),
        }
    }

    pub fn alarm(&self) -> &A {
        &self.alarm
    }

    /// Whether an uninstall emptied the object's storage, so the runtime can free it.
    pub fn removed(&self) -> bool {
        self.removed
    }

    fn save(&mut self) {
        self.replica.save(&mut self.store);
        self.save_meta();
    }

    fn save_meta(&mut self) {
        self.meta.version = VERSION;
        self.stored_read = self.meta.last_read;
        if let Ok(bytes) = serde_json::to_vec(&self.meta) {
            self.store.put(META, bytes);
        }
    }

    /// The installation's token, found by asking GitHub which installation holds the repo
    /// when no event has named it.
    async fn auth<P: Host>(&mut self, p: &P, b: &Budget) -> Option<Auth> {
        let id = match self.meta.installation {
            Some(id) => id,
            None => {
                b.spend(1);
                let id = install::installation_id(p, &self.name).await?;
                self.meta.installation = Some(id);
                id
            }
        };
        let now = p.now();
        if let Some((cached, token, until)) = &self.token
            && *cached == id
            && now < *until
        {
            return Some(Auth::Installation(token.clone()));
        }
        b.spend(1);
        let auth = install::token_direct(p, id).await?;
        if let Auth::Installation(token) = &auth {
            self.token = Some((id, token.clone(), now + TOKEN_KEEP));
        }
        Some(auth)
    }

    fn open(&self, now: u64) -> bool {
        now.saturating_sub(self.meta.last_read) < OPEN
    }

    /// The alarm one batch ahead, unless one is set that fires sooner: an idle alarm for a
    /// resync, many minutes off, doesn't hold a recompute back.
    fn batch(&mut self, now: u64) {
        if self.alarm.get().is_none_or(|at| at > now + BATCH) {
            self.alarm.set(now + BATCH);
        }
    }

    // ---- POST /event

    /// `POST /event`: a verified webhook's event, its delivery and the installation. Applies
    /// it, sets the alarm and answers 202; it recomputes nothing (`alarm-batches`).
    pub async fn event<P: Host>(&mut self, p: &P, req: &Value) -> Answer {
        let now = p.now();
        let (event, body) = (req["event"].as_str().unwrap_or_default(), &req["body"]);
        if on_removal(&mut Wipe(&mut self.store), &self.name, event, body) {
            self.replica = Replica::default();
            self.meta = Meta::default();
            self.listing = None;
            self.alarm.clear();
            self.removed = true;
            return Answer::empty(202);
        }
        let b = Budget::default();
        if let Some(id) = req["installation"].as_u64() {
            self.meta.installation = Some(id);
        }
        if self.meta.tip.is_none() {
            // Nothing is held: the first build reads the repo as it is now, and nothing of
            // the event is applied, its delivery included.
            if self.alarm.get().is_none() {
                self.alarm.set(now);
            }
        } else {
            let applied = match event {
                "push" => self.push(p, &b, body).await,
                "pull_request" => self.pull_request(p, &b, body).await,
                _ => false,
            };
            // Only an event applied names the next notice's delivery and sets the alarm.
            if applied {
                self.meta.delivery = req["delivery"].as_str().map(String::from);
                self.batch(now);
            }
        }
        self.save();
        Answer::empty(202)
    }

    /// Applies a push; answers whether it applied anything, which a tag's push doesn't.
    async fn push<P: Host>(&mut self, p: &P, b: &Budget, body: &Value) -> bool {
        let Some(branch) = body["ref"]
            .as_str()
            .and_then(|r| r.strip_prefix("refs/heads/"))
        else {
            return false;
        };
        let now = p.now();
        let after = body["after"].as_str().unwrap_or_default();
        let deleted = body["deleted"] == true || after.bytes().all(|c| c == b'0');
        let actor = body["pusher"]["name"]
            .as_str()
            .or(body["sender"]["login"].as_str())
            .unwrap_or_default()
            .to_string();
        let at = commit_time(body).unwrap_or_else(|| {
            if let Some(ts) = body["head_commit"]["timestamp"].as_str() {
                p.log(&format!(
                    "replica {}: unreadable commit time {ts}",
                    self.name
                ));
            }
            now
        });
        if let Some(d) = body["repository"]["default_branch"].as_str() {
            self.meta.default_branch = Some(d.into());
        }
        self.replica.apply(Change::Event(Event::Push {
            branch: branch.into(),
            at,
            actor: actor.clone(),
        }));
        if body["created"] == true {
            self.replica.apply(Change::Event(Event::Create {
                branch: branch.into(),
                at,
                actor,
            }));
        }
        if self.meta.default_branch.as_deref() == Some(branch) {
            if !deleted {
                self.move_tip(p, b, after).await;
            }
        } else if branch.starts_with("clm/ledger/") {
            self.move_ledger(p, b, branch, (!deleted).then_some(after))
                .await;
        } else if clm_replica::reads_ref(branch) {
            self.move_ref(p, b, branch, (!deleted).then_some(after), at)
                .await;
        }
        true
    }

    /// The default branch is at `after`: fetches what the compare lists when it is a
    /// fast-forward of few files, else lists the tree.
    async fn move_tip<P: Host>(&mut self, p: &P, b: &Budget, after: &str) {
        let Some(held) = self.meta.tip.clone() else {
            return;
        };
        if held == after {
            return;
        }
        let Some(auth) = self.auth(p, b).await else {
            p.log(&format!("replica {}: no installation token", self.name));
            return;
        };
        let compare = source::compare(p, &auth, &self.name, &held, after, b).await;
        match compare {
            Some(compare) => self.apply_compare(p, b, &auth, after, compare).await,
            None => self.relist(p, b, &auth, after).await,
        }
    }

    async fn apply_compare<P: Host>(
        &mut self,
        p: &P,
        b: &Budget,
        auth: &Auth,
        after: &str,
        compare: source::Compare,
    ) {
        // A compare names no mode: `fetch` asks each file's parent tree for it.
        let entry = |c: &source::Changed| Entry {
            path: c.path.clone(),
            sha: c.sha.clone(),
            size: 0,
            mode: String::new(),
        };
        let wanted: Vec<Entry> = compare
            .changed
            .iter()
            .filter(|c| clm_replica::wanted(&c.path))
            .map(entry)
            .collect();
        let mut files = source::fetch(p, auth, &self.name, after, &wanted, b).await;
        // What isn't fetched stays to fetch: the replica lists it as pending.
        for c in &compare.changed {
            if !files.iter().any(|f| f.entry.path == c.path) {
                // Its mode is read when the tree is listed to fetch it.
                let mut unfetched = entry(c);
                unfetched.mode = "100644".into();
                files.push(File::unfetched(unfetched));
            }
        }
        self.replica.apply(Change::Tip {
            sha: after.into(),
            files,
            removed: compare.removed,
        });
        self.meta.tip = Some(after.into());
        self.listing = None;
    }

    /// Lists the tree at `after` and fetches the blobs whose SHAs the replica doesn't hold.
    async fn relist<P: Host>(&mut self, p: &P, b: &Budget, auth: &Auth, after: &str) {
        let Some(entries) = source::tree(p, auth, &self.name, after, b).await else {
            p.log(&format!("replica {}: the tree can't be listed", self.name));
            return;
        };
        self.listing = Some(
            entries
                .iter()
                .map(|e| (e.path.clone(), e.clone()))
                .collect(),
        );
        self.replica.apply(Change::Tree {
            sha: after.into(),
            entries,
        });
        self.meta.tip = Some(after.into());
        self.fetch_pending(p, b, auth, false).await;
    }

    /// Fetches the paths the replica still needs, the spec files first, as many as the
    /// budget allows; with `spec_only`, only the spec files. Answers how many it took in.
    async fn fetch_pending<P: Host>(
        &mut self,
        p: &P,
        b: &Budget,
        auth: &Auth,
        spec_only: bool,
    ) -> usize {
        let mut pending = self.replica.pending_fetch();
        if spec_only {
            pending.retain(|path| clm_replica::keeps_text(path));
        }
        let Some(tip) = self.meta.tip.clone().filter(|_| !pending.is_empty()) else {
            return 0;
        };
        if self.listing.is_none() {
            let Some(entries) = source::tree(p, auth, &self.name, &tip, b).await else {
                return 0;
            };
            self.listing = Some(entries.into_iter().map(|e| (e.path.clone(), e)).collect());
        }
        let listing = self.listing.as_ref();
        let entries: Vec<Entry> = pending
            .iter()
            .filter_map(|path| listing?.get(path).cloned())
            .collect();
        // The files whose text the replica keeps go in queries of their own, before any
        // other (`fetches-what-the-replica-wants`).
        let (kept, rest): (Vec<Entry>, Vec<Entry>) = entries
            .into_iter()
            .partition(|e| clm_replica::keeps_text(&e.path));
        // The rest takes what budget the kept text leaves: a kept file that can't be read
        // holds nothing else back.
        let mut files = source::fetch(p, auth, &self.name, &tip, &kept, b).await;
        files.extend(source::fetch(p, auth, &self.name, &tip, &rest, b).await);
        let taken = files.len();
        if taken > 0 {
            self.replica.apply(Change::Tip {
                sha: tip,
                files,
                removed: Vec::new(),
            });
        }
        taken
    }

    async fn move_ledger<P: Host>(&mut self, p: &P, b: &Budget, branch: &str, tip: Option<&str>) {
        let text = match tip {
            None => None,
            Some(sha) => {
                let Some(auth) = self.auth(p, b).await else {
                    return;
                };
                let one = [(branch.to_string(), sha.to_string())];
                match source::ledger_texts(p, &auth, &self.name, &one, b).await {
                    Some(mut texts) => texts.pop().flatten(),
                    None => return,
                }
            }
        };
        match tip {
            Some(sha) if text.is_some() => {
                self.meta.ledgers.insert(branch.into(), sha.into());
            }
            _ => {
                self.meta.ledgers.remove(branch);
            }
        }
        self.replica.apply(Change::Ledger {
            branch: branch.into(),
            text,
        });
    }

    /// Holds `name` at `sha` as applied to the replica, and, when its change files or code
    /// key weren't `read` (a `propose/*` ref has none to read), apart as still to read, so a
    /// resync reads it again without taking it for a new ref.
    fn hold_ref(&mut self, name: &str, sha: &str, read: bool) {
        self.meta.refs.insert(name.into(), sha.into());
        if read || name.starts_with("propose/") {
            self.meta.unread.remove(name);
        } else {
            self.meta.unread.insert(name.into());
        }
    }

    /// A ref the views read moved, was created or was deleted (`tip` is none).
    async fn move_ref<P: Host>(
        &mut self,
        p: &P,
        b: &Budget,
        name: &str,
        tip: Option<&str>,
        time: u64,
    ) {
        let Some(sha) = tip else {
            self.meta.refs.remove(name);
            self.meta.unread.remove(name);
            self.replica.apply(Change::Ref {
                name: name.into(),
                sha: None,
                time,
                change: None,
            });
            return;
        };
        let Some(auth) = self.auth(p, b).await else {
            return;
        };
        let change = self.ref_change(p, b, &auth, name, sha).await;
        self.hold_ref(name, sha, change.is_some());
        self.replica.apply(Change::Ref {
            name: name.into(),
            sha: Some(sha.into()),
            time,
            change,
        });
    }

    /// What the views read of a ref at `sha`: a spec ref's change files, an `impl/*` head's
    /// code key.
    async fn ref_change<P: Host>(
        &self,
        p: &P,
        b: &Budget,
        auth: &Auth,
        name: &str,
        sha: &str,
    ) -> Option<clm_replica::RefChange> {
        if name.starts_with("impl/") {
            source::code_key(p, auth, &self.name, sha, b).await
        } else if name.starts_with("spec/") || name.starts_with("autosync/spec/") {
            let one = [(name.to_string(), sha.to_string())];
            source::spec_changes(p, auth, &self.name, &one, b)
                .await?
                .pop()
        } else {
            None
        }
    }

    /// Applies a `pull_request` event and sends the review notice it calls for; answers
    /// whether it applied anything, which an event naming no pull request doesn't.
    async fn pull_request<P: Host>(&mut self, p: &P, b: &Budget, body: &Value) -> bool {
        let pr = &body["pull_request"];
        let mut applied = false;
        if let (Some(number), Some(head), Some(base)) = (
            pr["number"].as_u64(),
            pr["head"]["ref"].as_str(),
            pr["base"]["ref"].as_str(),
        ) {
            let closed = pr["state"] == "closed" || body["action"] == "closed";
            let pull = Pull {
                number,
                title: pr["title"].as_str().unwrap_or_default().into(),
                head: head.into(),
                base: base.into(),
                head_sha: pr["head"]["sha"].as_str().unwrap_or_default().into(),
                url: pr["html_url"].as_str().unwrap_or_default().into(),
                fork: pr["head"]["repo"]["full_name"] != pr["base"]["repo"]["full_name"],
                open: !closed,
            };
            if pull.open {
                self.meta.pulls.insert(number, pull.clone());
            } else {
                self.meta.pulls.remove(&number);
            }
            self.replica.apply(Change::Pull(pull));
            applied = true;
        }
        if clm_web_live::review_notice(&self.name, body).is_some()
            && p.secret("VAPID_PUBLIC_KEY").is_some()
            && let Some(auth) = self.auth(p, b).await
        {
            crate::notify::review(
                p,
                &auth,
                &mut self.permissions,
                &mut self.meta.gone,
                &self.name,
                body,
            )
            .await;
        }
        applied
    }

    // ---- GET /view/<name>

    /// `GET /view/<name>?fresh=<0|1>`: the last computed view with its `sha` and
    /// `computed_at`, or 503 `{"preparing": <share>}`. A read of an object that holds nothing
    /// starts the build. With `fresh` it first brings itself up to the tip and recomputes if
    /// anything was stale.
    pub async fn view<P: Host>(&mut self, p: &P, name: &str, fresh: bool) -> Answer {
        let now = p.now();
        self.meta.last_read = now;
        if self.meta.tip.is_none() {
            if self.alarm.get().is_none() {
                self.alarm.set(now);
            }
            self.save_meta();
            return Answer::json(503, &json!({ "preparing": 0 }));
        }
        if fresh {
            self.catch_up(p, &Budget::default()).await;
            if self.replica.stale() {
                self.meta.next = self.replica.recompute(now).next;
                self.save();
                self.batch(now);
            }
        }
        // Files still to fetch, as a replica a newer release loaded lists, bring the alarm
        // forward; the read answers the views held until it recomputes
        // (`refetches-what-a-release-forgot`).
        if self.alarm.get().is_none_or(|at| at > now + BATCH)
            && !self.replica.pending_fetch().is_empty()
        {
            self.alarm.set(now);
        }
        let answer = match self.replica.view(name) {
            None => Answer::empty(404),
            Some(Viewed::Preparing { done }) => {
                if self.alarm.get().is_none() {
                    self.alarm.set(now);
                }
                Answer::json(503, &json!({ "preparing": done }))
            }
            Some(Viewed::Ready {
                sha,
                computed_at,
                value,
            }) => Answer::json(
                200,
                &json!({ "sha": sha, "computed_at": computed_at, "view": value }),
            ),
        };
        if self.alarm.get().is_none() {
            self.arm_idle(now);
        }
        if self.meta.last_read.saturating_sub(self.stored_read) >= READ_WRITE {
            self.save_meta();
        }
        answer
    }

    /// Reads the default branch's ref and applies a push the object hasn't seen. Answers
    /// whether the object's tip is now that ref's commit; a tip that moved with files still
    /// to fetch has moved.
    async fn catch_up<P: Host>(&mut self, p: &P, b: &Budget) -> bool {
        let Some(auth) = self.auth(p, b).await else {
            return false;
        };
        let branch = match self.meta.default_branch.clone() {
            Some(branch) => branch,
            None => {
                let Some(branch) = source::default_branch(p, &auth, &self.name, b).await else {
                    return false;
                };
                self.meta.default_branch = Some(branch.clone());
                branch
            }
        };
        let Some(tip) = source::branch_tip(p, &auth, &self.name, &branch, b).await else {
            return false;
        };
        self.move_tip(p, b, &tip).await;
        self.meta.tip.as_deref() == Some(tip.as_str())
    }

    /// While the pages are open, the alarm at the next lapse or resync.
    fn arm_idle(&mut self, now: u64) {
        let resync = self.meta.last_resync + RESYNC;
        let at = self.meta.next.map_or(resync, |n| n.min(resync));
        self.alarm.set(at.max(now + 1));
    }

    // ---- POST /write

    /// `POST /write`: `{"login", "token", "kind": "decide" | "propose" | "withdraw" | "answer", ...}`.
    /// Brings itself up to the tip, validates, appends with the requester's token, applies the
    /// record and, for a decision, a proposal or an answer, recomputes before it answers.
    pub async fn write<P: Host>(&mut self, p: &P, req: &Value) -> Answer {
        let now = p.now();
        let done = match self.replica.view("snapshot") {
            Some(Viewed::Ready { .. }) if self.meta.tip.is_some() => None,
            Some(Viewed::Preparing { done }) if self.meta.tip.is_some() => Some(json!(done)),
            _ => Some(json!(0)),
        };
        if let Some(done) = done {
            if self.alarm.get().is_none() {
                self.alarm.set(now);
            }
            return Answer::json(503, &json!({ "preparing": done }));
        }
        let (Some(login), Some(token)) = (req["login"].as_str(), req["token"].as_str()) else {
            return Answer::empty(422);
        };
        // A kind that is no write's is refused (422).
        let Some(kind) = req["kind"]
            .as_str()
            .filter(|k| ["decide", "propose", "withdraw", "answer"].contains(k))
        else {
            return Answer::empty(422);
        };
        let Some((owner, repo)) = self.name.split_once('/') else {
            return Answer::empty(404);
        };
        // GitHub's 403 or 404 is a token that can't read the repo; any other failure,
        // a 401 included, is GitHub failing the check.
        let access = match access::checked_direct(p, token, owner, repo).await {
            Ok(Some(access)) => access,
            Ok(None) => return Answer::empty(404),
            Err(_) => return Answer::empty(502),
        };
        if !self.catch_up(p, &Budget::default()).await {
            self.save();
            return Answer::empty(502);
        }
        let answer = match kind {
            "decide" => self.decide(p, &access, login, req).await,
            "propose" => self.propose(p, &access, login, req).await,
            "withdraw" => self.withdraw(p, &access, login, req).await,
            _ => self.answer(p, &access, login, req).await,
        };
        self.batch(now);
        self.save();
        answer
    }

    async fn decide<P: Host>(
        &mut self,
        p: &P,
        access: &Access,
        login: &str,
        req: &Value,
    ) -> Answer {
        let (Some(id), Some(version)) = (req["id"].as_str(), req["version"].as_str()) else {
            return Answer::empty(422);
        };
        let (Some(value), Some(decision)) = (req["value"].as_str(), req["decision"].as_str())
        else {
            return Answer::empty(422);
        };
        // The command a signed refusal names is pasted into a shell: only the words the
        // ledger writes go into it.
        if !["high", "medium", "low"].contains(&value)
            || !["keep", "cut", "redesign", "accept"].contains(&decision)
        {
            return Answer::empty(422);
        }
        let write = Write::Decision {
            id: id.into(),
            version: version.into(),
        };
        let settles = match self.replica.validate(&write) {
            Ok(Validated::Decision { settles }) => settles,
            Ok(_) => return Answer::empty(422),
            Err(Refusal::Signed) => {
                let (layer, slug) = id.split_once('/').unwrap_or_default();
                let command = if layer == "architecture" {
                    "clm architecture decide"
                } else {
                    "clm capabilities decide"
                };
                let command = format!("{command} {slug} --value {} --decision {}", value, decision);
                return Answer::json(422, &json!({ "error": "signed", "command": command }));
            }
            Err(Refusal::Stale) => return Answer::empty(409),
            Err(_) => return Answer::empty(422),
        };
        // The record's body as the ledger writes it: the types are `ledger`'s, reached
        // through `web.worker.platform`'s `record`.
        let body = json!({
            "kind": "decided", "capability": id, "value": value, "decision": decision,
            "settles": settles,
        });
        let Ok(body) = serde_json::from_value(body) else {
            return Answer::empty(422);
        };
        let record = ledger::record(p, login, body);
        let appended = ledger::appended(p, access, &record).await;
        if !self.applied(&record.writer, appended) {
            return Answer::empty(502);
        }
        self.recompute(p.now());
        Answer {
            status: 200,
            body: serde_json::to_string(&record).unwrap_or_default(),
        }
    }

    async fn propose<P: Host>(
        &mut self,
        p: &P,
        access: &Access,
        login: &str,
        req: &Value,
    ) -> Answer {
        let (Some(title), Some(text)) = (req["title"].as_str(), req["body"].as_str()) else {
            return Answer::empty(422);
        };
        let nodes: Vec<String> = req["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| n.as_str().map(String::from))
            .collect();
        let write = Write::Proposal {
            title: title.into(),
            nodes: nodes.clone(),
        };
        let slug = match self.replica.validate(&write) {
            Ok(Validated::Proposal { slug }) => slug,
            Err(Refusal::Taken) => return Answer::json(409, &json!({ "error": "taken" })),
            Err(Refusal::Invalid) => return Answer::json(422, &json!({ "error": "nodes" })),
            _ => return Answer::empty(422),
        };
        let body = json!({
            "kind": "proposed", "slug": slug, "title": title, "body": text, "nodes": nodes,
        });
        let Ok(body) = serde_json::from_value(body) else {
            return Answer::empty(422);
        };
        let record = ledger::record(p, login, body);
        let appended = ledger::appended(p, access, &record).await;
        if !self.applied(&record.writer, appended) {
            return Answer::empty(502);
        }
        self.recompute(p.now());
        Answer::json(200, &json!({ "id": record.id }))
    }

    /// An answer to a backlog item, as the replica validates it (`replica/validate-answers`).
    async fn answer<P: Host>(
        &mut self,
        p: &P,
        access: &Access,
        login: &str,
        req: &Value,
    ) -> Answer {
        let (Some(item), Some(text)) = (req["item"].as_str(), req["answer"].as_str()) else {
            return Answer::empty(422);
        };
        let write = Write::Answer {
            item: item.into(),
            answer: text.into(),
        };
        match self.replica.validate(&write) {
            Ok(Validated::Answer) => {}
            Err(Refusal::Unknown) => return Answer::json(404, &json!({ "error": "unknown" })),
            Err(Refusal::Invalid) => return Answer::json(422, &json!({ "error": "answer" })),
            _ => return Answer::empty(422),
        }
        let Ok(body) = serde_json::from_value(json!({
            "kind": "answered", "item": item, "answer": text,
        })) else {
            return Answer::empty(422);
        };
        let record = ledger::record(p, login, body);
        let appended = ledger::appended(p, access, &record).await;
        if !self.applied(&record.writer, appended) {
            return Answer::empty(502);
        }
        self.recompute(p.now());
        Answer::json(200, &json!({ "id": record.id }))
    }

    /// Decides from the requester's own web ledger read at its tip, never from the replica.
    async fn withdraw<P: Host>(
        &mut self,
        p: &P,
        access: &Access,
        login: &str,
        req: &Value,
    ) -> Answer {
        let Some(id) = req["id"].as_str() else {
            return Answer::empty(422);
        };
        let Some(own) = ledger::own(p, access, login).await else {
            return Answer::empty(502);
        };
        if !own.proposals.contains_key(id) {
            return Answer::empty(404);
        }
        let Ok(body) = serde_json::from_value(json!({"kind": "withdrawn", "proposal": id})) else {
            return Answer::empty(422);
        };
        let record = ledger::record(p, login, body);
        let appended = ledger::appended(p, access, &record).await;
        if self.applied(&record.writer, appended) {
            Answer::empty(200)
        } else {
            Answer::empty(502)
        }
    }

    /// After an append of a record by `writer`: applies the ledger text the append committed,
    /// without reading the branch back. Answers whether the append went through.
    fn applied(&mut self, writer: &str, appended: Option<ledger::Appended>) -> bool {
        let Some(ledger::Appended { tip, text }) = appended else {
            return false;
        };
        let branch = format!("clm/ledger/{writer}");
        self.meta.ledgers.insert(branch.clone(), tip);
        self.replica.apply(Change::Ledger {
            branch,
            text: Some(text),
        });
        true
    }

    fn recompute(&mut self, now: u64) {
        self.meta.next = self.replica.recompute(now).next;
    }

    // ---- the alarm

    /// The alarm: builds or continues the first build, resyncs when it is due and the pages
    /// are open, recomputes once, saves, and then tells the open pages and sends the
    /// notifications the recompute calls for (`alarm-batches`, `first-build`, `resync`).
    pub async fn on_alarm<P: Host>(&mut self, p: &P) {
        let now = p.now();
        self.alarm.clear();
        let b = Budget::default();
        let mut taken = 0;
        let mut auth: Option<Auth> = None;
        let building = self.meta.tip.is_none();
        let pending = !self.replica.pending_fetch().is_empty();
        if building || pending {
            auth = self.auth(p, &b).await;
            match &auth {
                None => {
                    p.log(&format!("replica {}: no installation token", self.name));
                    self.alarm.set(now + BATCH);
                    return;
                }
                Some(a) if building && !self.start(p, &b, a).await => {
                    self.alarm.set(now + BATCH);
                    return;
                }
                _ => {}
            }
        }
        // A resync due goes before the files still to fetch: only the fetch stops at the
        // budget, so it takes what the resync leaves.
        if self.open(now) && now >= self.meta.last_resync + RESYNC {
            if auth.is_none() {
                auth = self.auth(p, &b).await;
            }
            if let Some(a) = &auth {
                self.resync(p, &b, a, true).await;
            }
            self.meta.last_resync = now;
        }
        if (building || pending)
            && let Some(a) = &auth
        {
            taken = self.fetch_pending(p, &b, a, building).await;
        }
        let lapsed = self.meta.next.is_some_and(|t| t <= now);
        let mut recomputed = false;
        if self.replica.stale() || lapsed {
            self.recompute(now);
            recomputed = !self.replica.stale();
        }
        self.save();
        let notify = self.items_moved(p, &mut auth, &b).await;
        if recomputed || notify {
            self.notify(p).await;
        }
        self.save_meta();
        let pending = !self.replica.pending_fetch().is_empty();
        let mut at: Option<u64> = None;
        if pending {
            // The next slice at once while this one took files or the budget ran out before
            // the fetch could, as a resync can spend it; a minute on when GitHub gave nothing.
            let next = taken > 0 || building || b.left() < 2;
            at = Some(now + if next { 1 } else { BATCH });
        }
        if self.open(now) {
            let resync = self.meta.last_resync + RESYNC;
            let idle = self
                .meta
                .next
                .map_or(resync, |n| n.min(resync))
                .max(now + 1);
            at = Some(at.map_or(idle, |a| a.min(idle)));
        }
        if let Some(at) = at {
            self.alarm.set(at);
        }
    }

    /// The first build: the default branch's tip and tree, the ledgers, the refs, the open
    /// pull requests and the recent events, then the spec files are fetched.
    async fn start<P: Host>(&mut self, p: &P, b: &Budget, auth: &Auth) -> bool {
        let repo = self.name.clone();
        let branch = match self.meta.default_branch.clone() {
            Some(branch) => branch,
            None => match source::default_branch(p, auth, &repo, b).await {
                Some(branch) => branch,
                None => return false,
            },
        };
        self.meta.default_branch = Some(branch.clone());
        let Some(tip) = source::branch_tip(p, auth, &repo, &branch, b).await else {
            return false;
        };
        let Some(entries) = source::tree(p, auth, &repo, &tip, b).await else {
            return false;
        };
        self.listing = Some(
            entries
                .iter()
                .map(|e| (e.path.clone(), e.clone()))
                .collect(),
        );
        self.replica.apply(Change::Tree {
            sha: tip.clone(),
            entries,
        });
        self.meta.tip = Some(tip);
        self.meta.last_resync = p.now();
        // Every ref listed now is one the object hadn't looked for, not one just created:
        // the creations are those GitHub's events listing gives.
        self.resync(p, b, auth, false).await;
        if let Some(events) = source::events(p, auth, &repo, b).await {
            for event in events.into_iter().rev() {
                self.replica.apply(Change::Event(event));
            }
        }
        true
    }

    /// Lists the refs and the open pull requests and applies any difference from what the
    /// object holds: refs created, moved or deleted, ledger branches, the default branch's
    /// tip and the pull requests. With `creations`, a ref it didn't hold reaches the activity
    /// feed as created, its webhook having been lost.
    async fn resync<P: Host>(&mut self, p: &P, b: &Budget, auth: &Auth, creations: bool) {
        let repo = self.name.clone();
        if let Some(branch) = self.meta.default_branch.clone()
            && let Some(tip) = source::branch_tip(p, auth, &repo, &branch, b).await
        {
            self.move_tip(p, b, &tip).await;
        }
        for prefix in clm_replica::REF_PREFIXES {
            let Some(tips) = source::tips(p, auth, &repo, prefix, b).await else {
                continue;
            };
            let listed: BTreeMap<String, &source::Tip> = tips
                .iter()
                .map(|t| (format!("{prefix}{}", t.name), t))
                .collect();
            let gone: Vec<String> = self
                .meta
                .refs
                .keys()
                .filter(|n| {
                    n.starts_with(prefix) && clm_replica::reads_ref(n) && !listed.contains_key(*n)
                })
                .cloned()
                .collect();
            for name in gone {
                self.move_ref(p, b, &name, None, 0).await;
            }
            let moved: Vec<(String, &source::Tip)> = listed
                .into_iter()
                .filter(|(n, t)| {
                    self.meta.refs.get(n) != Some(&t.sha) || self.meta.unread.contains(n)
                })
                .collect();
            let spec: Vec<(String, String)> = moved
                .iter()
                .filter(|(n, _)| n.starts_with("spec/") || n.starts_with("autosync/spec/"))
                .map(|(n, t)| (n.clone(), t.sha.clone()))
                .collect();
            let mut changes = source::spec_changes(p, auth, &repo, &spec, b)
                .await
                .unwrap_or_default()
                .into_iter();
            for (name, tip) in moved {
                let change = if spec.iter().any(|(n, _)| *n == name) {
                    changes.next()
                } else if prefix == "impl/" && b.left() > 8 {
                    source::code_key(p, auth, &repo, &tip.sha, b).await
                } else {
                    None
                };
                let held = self.meta.refs.get(&name);
                if held == Some(&tip.sha) && change.is_none() {
                    // Still unread at the tip it was held at: nothing to apply again.
                    continue;
                }
                if creations && held.is_none() {
                    self.replica.apply(Change::Event(Event::Create {
                        branch: name.clone(),
                        at: tip.time,
                        actor: String::new(),
                    }));
                }
                self.hold_ref(&name, &tip.sha, change.is_some());
                self.replica.apply(Change::Ref {
                    name,
                    sha: Some(tip.sha.clone()),
                    time: tip.time,
                    change,
                });
            }
        }
        self.resync_ledgers(p, b, auth).await;
        // A listing not read to its end, as one of more than 300 is, changes no pull
        // request: one it leaves out may still be open.
        if let Some((pulls, true)) = source::pulls(p, auth, &repo, b).await {
            let gone: Vec<Pull> = self
                .meta
                .pulls
                .values()
                .filter(|held| !pulls.iter().any(|p| p.number == held.number))
                .cloned()
                .collect();
            for mut pull in gone {
                pull.open = false;
                self.meta.pulls.remove(&pull.number);
                self.replica.apply(Change::Pull(pull));
            }
            for pull in pulls {
                if self.meta.pulls.get(&pull.number) != Some(&pull) {
                    self.meta.pulls.insert(pull.number, pull.clone());
                    self.replica.apply(Change::Pull(pull));
                }
            }
        }
    }

    async fn resync_ledgers<P: Host>(&mut self, p: &P, b: &Budget, auth: &Auth) {
        let Some(tips) = source::ledger_tips(p, auth, &self.name, b).await else {
            return;
        };
        let gone: Vec<String> = self
            .meta
            .ledgers
            .keys()
            .filter(|n| !tips.iter().any(|(t, _)| t == *n))
            .cloned()
            .collect();
        for branch in gone {
            self.meta.ledgers.remove(&branch);
            self.replica.apply(Change::Ledger { branch, text: None });
        }
        let moved: Vec<(String, String)> = tips
            .into_iter()
            .filter(|(n, sha)| self.meta.ledgers.get(n) != Some(sha))
            .collect();
        let Some(texts) = source::ledger_texts(p, auth, &self.name, &moved, b).await else {
            return;
        };
        for ((branch, sha), text) in moved.into_iter().zip(texts) {
            if text.is_some() {
                self.meta.ledgers.insert(branch.clone(), sha);
            }
            self.replica.apply(Change::Ledger { branch, text });
        }
    }

    /// Whether the backlog's items are those of a different commit than the last notice
    /// looked at: sends `waiting_notice`'s payload for the items before and after.
    async fn items_moved<P: Host>(&mut self, p: &P, auth: &mut Option<Auth>, b: &Budget) -> bool {
        let Some(Viewed::Ready { sha, value, .. }) = self.replica.view("snapshot") else {
            return false;
        };
        if self.meta.items_sha.as_deref() == Some(sha.as_str()) {
            return false;
        }
        let new: Vec<(String, String, String)> = value["backlog"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|i| {
                Some((
                    i["slug"].as_str()?.into(),
                    i["kind"].as_str()?.into(),
                    i["title"].as_str()?.into(),
                ))
            })
            .collect();
        let previous = self.meta.items.replace(new.clone());
        self.meta.items_sha = Some(sha);
        let item = |(slug, kind, title): &(String, String, String)| Item {
            slug: slug.clone(),
            kind: kind.clone(),
            title: title.clone(),
        };
        if let Some(previous) = previous
            && p.secret("VAPID_PUBLIC_KEY").is_some()
        {
            let (previous, new): (Vec<Item>, Vec<Item>) = (
                previous.iter().map(item).collect(),
                new.iter().map(item).collect(),
            );
            if clm_web_live::waiting_notice(&self.name, &previous, &new).is_some() {
                if auth.is_none() {
                    *auth = self.auth(p, b).await;
                }
                if let Some(a) = auth {
                    crate::notify::waiting(
                        p,
                        a,
                        &mut self.permissions,
                        &mut self.meta.gone,
                        &self.name,
                        Some(&previous),
                        &new,
                    )
                    .await;
                }
            }
        }
        true
    }

    /// Posts the latest delivery applied to the repo's `Live` object's `/notify`, so open
    /// pages refresh; a recompute no event caused names a fresh one.
    async fn notify<P: Host>(&mut self, p: &P) {
        let delivery = match self.meta.delivery.take() {
            Some(delivery) => delivery,
            None => {
                let mut bytes = [0u8; 16];
                p.random(&mut bytes);
                bytes.iter().map(|b| format!("{b:02x}")).collect()
            }
        };
        let body = json!({ "delivery": delivery }).to_string();
        if p.live_post(&self.name, "/notify", &body).await.is_none() {
            p.log(&format!("replica {}: the pages weren't told", self.name));
        }
    }
}

/// A push's head commit time: the instant its RFC 3339 timestamp names, offset counted.
fn commit_time(body: &Value) -> Option<u64> {
    unix_time(body["head_commit"]["timestamp"].as_str()?)
}
