//! The ledgers' state (`per-writer-ledgers`): every `clm/ledger/*` branch's `records.jsonl`,
//! read in one GraphQL query and folded by `clm_ledger`, so the Worker sees the approvals
//! and checks `clm` sees. The branches' sources are kept with the live state for 60
//! seconds, so a person's web subscriptions can be folded from their own branch alone
//! (`web-subscriptions-by-branch`).

use clm_ledger::{Body, Fold, Record};
use serde_json::{Value, json};

use crate::access::{Access, valid_name};
use crate::platform::{Auth, Clock, GitHub, Platform, Secrets};
use crate::{files, gh, live};

/// Each ledger branch's name and its `records.jsonl`.
pub type Sources = Vec<(String, String)>;

pub async fn fold<P: Platform>(p: &P, access: &Access) -> Option<Fold> {
    Some(folded(&sources(p, access).await?))
}

/// Every ledger branch's source, kept with the live state.
pub async fn sources<P: Platform>(p: &P, access: &Access) -> Option<Sources> {
    live::cached(p, access, "ledger", async || {
        read(p, access.auth(), &access.full_name()).await
    })
    .await
}

/// The fold of every source.
pub fn folded(sources: &Sources) -> Fold {
    let sources: Vec<(&str, &str)> = sources
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    clm_ledger::fold_records(&sources)
}

/// The login a web ledger branch, `clm/ledger/<login>/web`, names, for a `<login>` the
/// owner-name rule accepts; `None` for any other branch.
fn web_login(branch: &str) -> Option<&str> {
    branch
        .strip_prefix("clm/ledger/")?
        .strip_suffix("/web")
        .filter(|login| valid_name(login))
}

/// Each web ledger branch's login with the fold of its records alone, whatever `writer`
/// they claim: that person's web subscriptions, which no other branch's records end
/// (`web-subscriptions-by-branch`).
pub fn web_folds(sources: &Sources) -> impl Iterator<Item = (&str, Fold)> {
    sources.iter().filter_map(|(name, text)| {
        let login = web_login(name)?;
        Some((login, clm_ledger::fold_records(&[(name, text)])))
    })
}

/// The fold of `login`'s own web ledger from `sources`, empty when it has none.
pub fn web_fold(sources: &Sources, login: &str) -> Fold {
    sources
        .iter()
        .find(|(name, _)| web_login(name) == Some(login))
        .map(|(name, text)| clm_ledger::fold_records(&[(name, text)]))
        .unwrap_or_default()
}

/// Every ledger branch of `repo` with its `records.jsonl`, read now, as `auth`.
pub async fn read<P: GitHub>(p: &P, auth: &Auth, repo: &str) -> Option<Sources> {
    let path = format!("/repos/{repo}/git/matching-refs/heads/clm/ledger/");
    let refs = gh::get_ok(p, auth, &path).await?;
    let branches: Vec<(String, String)> = refs
        .as_array()?
        .iter()
        .filter_map(|r| {
            let name = r["ref"].as_str()?.strip_prefix("refs/heads/")?;
            Some((name.to_string(), r["object"]["sha"].as_str()?.to_string()))
        })
        .collect();
    if branches.is_empty() {
        return Some(Vec::new());
    }
    let expressions: Vec<String> = branches
        .iter()
        .map(|(_, sha)| format!("{sha}:records.jsonl"))
        .collect();
    let texts = files::objects_direct(p, auth, repo, "Ledgers", &expressions).await?;
    Some(
        branches
            .into_iter()
            .zip(texts)
            .filter_map(|((name, _), text)| Some((name, text?)))
            .collect(),
    )
}

/// The fold of `clm/ledger/<login>/web` read at its tip, so what a write decides is what
/// the ledger holds now, not the minute-old fold: empty when GitHub says it isn't there,
/// `None` when it can't be read.
pub async fn own<P: GitHub>(p: &P, access: &Access, login: &str) -> Option<Fold> {
    let name = format!("clm/ledger/{}", web_writer(login));
    let path = format!("/repos/{}/git/ref/heads/{name}", access.full_name());
    let tip = match gh::get(p, access.auth(), &path).await {
        (404, _) => return Some(Fold::default()),
        (200, v) => v["object"]["sha"].as_str()?.to_string(),
        _ => return None,
    };
    let expression = format!("{tip}:records.jsonl");
    let repo = access.full_name();
    let text = files::objects_direct(p, access.auth(), &repo, "Ledgers", &[expression])
        .await?
        .pop()
        .flatten()?;
    Some(clm_ledger::fold_records(&[(&name, &text)]))
}

/// The ledger's one file, as `clm-ledger` writes it.
const FILE: &str = "records.jsonl";

/// The commit `branch` points at.
pub async fn tip<P: GitHub>(p: &P, access: &Access, branch: &str) -> Option<String> {
    let path = format!("/repos/{}/git/ref/heads/{branch}", access.full_name());
    let v = gh::get_ok(p, access.auth(), &path).await?;
    v["object"]["sha"].as_str().map(String::from)
}

/// The writer of `login`'s web ledger, `clm/ledger/<login>/web`.
pub fn web_writer(login: &str) -> String {
    format!("{login}/web")
}

/// A record of `body` by `<login>/web`, now, with a random ID.
pub fn record<P: Clock + Secrets>(p: &P, login: &str, body: Body) -> Record {
    let mut id = [0u8; 16];
    p.random(&mut id);
    Record {
        id: id.iter().map(|b| format!("{b:02x}")).collect(),
        writer: web_writer(login),
        at: crate::install::rfc3339(p.now()),
        body,
    }
}

/// The `sha` of a Git Data API create.
fn created(res: (u16, Value)) -> Option<String> {
    match res {
        (200 | 201, v) => v["sha"].as_str().map(String::from),
        _ => None,
    }
}

/// Commits `record` as the last line of its writer's `records.jsonl` and moves the ref,
/// only forward, so an append that raced another fails rather than dropping it.
pub async fn append<P: GitHub>(p: &P, access: &Access, record: &Record) -> Option<()> {
    let repo = access.full_name();
    let auth = access.auth();
    let name = format!("clm/ledger/{}", record.writer);
    let parent = tip(p, access, &name).await;
    let mut content = match &parent {
        Some(sha) => {
            let expression = format!("{sha}:{FILE}");
            files::objects_direct(p, auth, &repo, "Ledgers", &[expression])
                .await?
                .pop()
                // A ledger whose file can't be read is never rewritten without it.
                .flatten()?
        }
        None => String::new(),
    };
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&serde_json::to_string(record).ok()?);
    content.push('\n');
    let git = format!("{}/repos/{repo}/git", gh::API);
    let blob = json!({"content": content, "encoding": "utf-8"});
    let blob = created(gh::post(p, auth, &format!("{git}/blobs"), &blob).await)?;
    let tree = json!({"tree": [{"path": FILE, "mode": "100644", "type": "blob", "sha": blob}]});
    let tree = created(gh::post(p, auth, &format!("{git}/trees"), &tree).await)?;
    let commit = json!({
        "message": format!("ledger: {}", record.body.kind()),
        "tree": tree,
        "parents": parent.iter().collect::<Vec<_>>(),
    });
    let commit = created(gh::post(p, auth, &format!("{git}/commits"), &commit).await)?;
    let moved = match parent {
        Some(_) => {
            let url = format!("{git}/refs/heads/{name}");
            gh::patch(p, auth, &url, &json!({"sha": commit, "force": false})).await
        }
        None => {
            let body = json!({"ref": format!("refs/heads/{name}"), "sha": commit});
            gh::post(p, auth, &format!("{git}/refs"), &body).await
        }
    };
    matches!(moved.0, 200 | 201).then_some(())
}
