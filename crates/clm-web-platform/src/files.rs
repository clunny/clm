//! Reading a commit's files from GitHub without a tarball: the tree listing, then the
//! text of many files per GraphQL query (`batched-fetches`), and a commit's spec files
//! (`spec-files`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clm_model::Files;
use serde_json::{Value, json};

use crate::{gh, keys};

use crate::platform::{Auth, GitHub, Platform};

/// At most this many paths per GraphQL query.
pub const PER_QUERY: usize = 100;
/// How long a commit's spec files stay in the Cache API. A commit never changes.
const FILES_TTL: u64 = 86_400;
/// Blob API calls one `spec_files` makes at most, within the subrequest budget.
const MAX_BLOB_FETCHES: usize = 20;

/// A file in a commit's tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: String,
    /// The blob's SHA.
    pub sha: String,
}

/// The Trees API's recursive listing of a commit's tree, with its `truncated` flag; none
/// for any answer but a 200.
pub async fn listing<P: GitHub>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
) -> Option<keys::Listing> {
    let v = gh::get_ok(
        p,
        auth,
        &format!("/repos/{repo}/git/trees/{sha}?recursive=1"),
    )
    .await?;
    keys::listing_of(&v)
}

/// The blobs of a listing.
pub fn blobs(listing: &keys::Listing) -> Vec<TreeEntry> {
    listing
        .entries
        .iter()
        .filter(|e| e.kind == "blob")
        .filter_map(|e| {
            Some(TreeEntry {
                path: String::from_utf8(e.path.clone()).ok()?,
                sha: e.id.clone(),
            })
        })
        .collect()
}

/// Every blob in the commit's tree.
async fn tree<P: GitHub>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<TreeEntry>> {
    Some(blobs(&listing(p, auth, repo, sha).await?))
}

/// Each path's text at `sha`, by path, none for a binary file, batched by GraphQL query.
async fn texts<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    paths: &[String],
) -> Option<BTreeMap<String, Option<String>>> {
    let expressions: Vec<String> = paths.iter().map(|path| format!("{sha}:{path}")).collect();
    let texts = objects(p, auth, repo, "Blobs", &expressions).await?;
    Some(paths.iter().cloned().zip(texts).collect())
}

/// The text of each `<rev>:<path>` expression, in order, `None` for a binary file or one
/// that doesn't exist, batched in GraphQL queries named `query`; a text GraphQL cuts short
/// is read whole from the blob API, and that blob kept a day.
pub async fn objects<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    query: &str,
    expressions: &[String],
) -> Option<Vec<Option<String>>> {
    fetch(p, auth, repo, query, expressions, async |sha: &str| {
        blob(p, auth, repo, sha).await
    })
    .await
}

/// [`objects`] with nothing kept: a truncated blob is read from the blob API each time
/// (`object-reads`).
pub async fn objects_direct<P: GitHub>(
    p: &P,
    auth: &Auth,
    repo: &str,
    query: &str,
    expressions: &[String],
) -> Option<Vec<Option<String>>> {
    fetch(p, auth, repo, query, expressions, async |sha: &str| {
        blob_direct(p, auth, repo, sha).await
    })
    .await
}

/// Each `<rev>:<path>` expression's text, in order, batched in GraphQL queries named
/// `query`, at most `PER_QUERY` a query; none at all when a query fails or GitHub answers
/// a null `repository`.
async fn fetch<P: GitHub>(
    p: &P,
    auth: &Auth,
    repo: &str,
    query: &str,
    expressions: &[String],
    blob: impl AsyncFn(&str) -> Option<Vec<u8>>,
) -> Option<Vec<Option<String>>> {
    let (owner, name) = repo.split_once('/')?;
    let mut out = Vec::new();
    for chunk in expressions.chunks(PER_QUERY) {
        let mut vars = serde_json::Map::new();
        vars.insert("owner".into(), json!(owner));
        vars.insert("name".into(), json!(name));
        let mut params = vec!["$owner: String!".to_string(), "$name: String!".to_string()];
        let mut fields = String::new();
        for (i, expression) in chunk.iter().enumerate() {
            vars.insert(format!("e{i}"), json!(expression));
            params.push(format!("$e{i}: String!"));
            fields.push_str(&format!(
                "f{i}: object(expression: $e{i}) {{ ... on Blob {{ text isBinary isTruncated oid }} }}\n"
            ));
        }
        let query = format!(
            "query {query}({}) {{ repository(owner: $owner, name: $name) {{\n{fields}}} }}",
            params.join(", ")
        );
        let data = gh::graphql(p, auth, &query, Value::Object(vars)).await?;
        let repository = &data["repository"];
        if repository.is_null() {
            return None;
        }
        for i in 0..chunk.len() {
            let object = &repository[format!("f{i}")];
            let text = match (object["text"].as_str(), object["isTruncated"] == true) {
                // GraphQL cuts long text short; the blob API has all of it.
                (Some(_), true) => {
                    let sha = object["oid"].as_str()?;
                    String::from_utf8(blob(sha).await?).ok()
                }
                (text, _) => text.map(String::from),
            };
            out.push(text);
        }
    }
    Some(out)
}

/// The Cache API key of a blob's bytes.
fn blob_key(repo: &str, sha: &str) -> String {
    format!("blob:{repo}:{sha}")
}

/// The bytes of a blob kept in the Cache API.
async fn kept_blob<P: Platform>(p: &P, repo: &str, sha: &str) -> Option<Vec<u8>> {
    let kept = p.cache_get(&blob_key(repo, sha)).await?;
    STANDARD.decode(kept).ok()
}

/// The bytes of a blob, from the Cache API or the blob API, kept a day: a blob never
/// changes.
async fn blob<P: Platform>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = kept_blob(p, repo, sha).await {
        return Some(bytes);
    }
    fetch_blob(p, auth, repo, sha).await
}

/// The bytes of a blob from the blob API, kept a day.
async fn fetch_blob<P: Platform>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<u8>> {
    let content = blob_content(p, auth, repo, sha).await?;
    p.cache_put(&blob_key(repo, sha), &content, FILES_TTL).await;
    STANDARD.decode(content).ok()
}

/// The bytes of a blob from the blob API, each time.
async fn blob_direct<P: GitHub>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<u8>> {
    STANDARD
        .decode(blob_content(p, auth, repo, sha).await?)
        .ok()
}

/// A blob's base64 content without its line breaks.
async fn blob_content<P: GitHub>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<String> {
    let v = gh::get_ok(p, auth, &format!("/repos/{repo}/git/blobs/{sha}")).await?;
    Some(v["content"].as_str()?.split_whitespace().collect())
}

/// Whether `spec_files` fetches a binary file whole: one in an `assets/` directory under
/// `specs/` or `changes/`, at any depth.
fn is_asset(path: &str) -> bool {
    (path.starts_with("specs/") || path.starts_with("changes/")) && path.contains("/assets/")
}

/// Whether `spec_files` gives a file: one whose text the replica keeps or the gate reads
/// as a spec (`replica/files-it-fetches`, `gate/classify`).
fn is_spec_file(path: &str) -> bool {
    clm_replica::keeps_text(path) || clm_gate::is_spec_path(path)
}

/// The spec files of commit `sha`, from the Cache API or from GitHub: each text file with
/// its text, and each binary asset with its bytes from the blob API, at most
/// `MAX_BLOB_FETCHES` fetched per call and each kept a day, so a later call goes on from
/// where this one stopped. None while the commit needs more, or when a fetch fails. Binary
/// files that aren't assets are left out.
pub async fn spec_files<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    tree: Option<&[TreeEntry]>,
) -> Option<Files> {
    let key = format!("files3:{repo}@{sha}");
    if let Some(files) = p.cache_get(&key).await.and_then(|v| decode(&v)) {
        return Some(files);
    }
    let fetched;
    let tree = match tree {
        Some(t) => t,
        None => {
            fetched = self::tree(p, auth, repo, sha).await?;
            &fetched
        }
    };
    let entries: Vec<&TreeEntry> = tree.iter().filter(|e| is_spec_file(&e.path)).collect();
    let paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let texts = texts(p, auth, repo, sha, &paths).await?;
    let mut files = Files::new();
    let mut fetches = 0;
    let mut complete = true;
    for entry in entries {
        let bytes = match texts.get(&entry.path).and_then(Option::as_ref) {
            Some(text) => text.clone().into_bytes(),
            None if !is_asset(&entry.path) => continue,
            None => match kept_blob(p, repo, &entry.sha).await {
                Some(bytes) => bytes,
                None if fetches < MAX_BLOB_FETCHES => {
                    fetches += 1;
                    fetch_blob(p, auth, repo, &entry.sha).await?
                }
                None => {
                    complete = false;
                    continue;
                }
            },
        };
        files.insert(PathBuf::from(&entry.path), bytes);
    }
    if !complete {
        return None;
    }
    p.cache_put(&key, &encode(&files), FILES_TTL).await;
    Some(files)
}

/// Each path to `{"t": text}`, or `{"b": base64}` when it isn't UTF-8.
fn encode(files: &Files) -> String {
    let map: serde_json::Map<String, Value> = files
        .iter()
        .map(|(path, bytes)| {
            let value = match std::str::from_utf8(bytes) {
                Ok(text) => json!({"t": text}),
                Err(_) => json!({"b": STANDARD.encode(bytes)}),
            };
            (path.to_string_lossy().into_owned(), value)
        })
        .collect();
    Value::Object(map).to_string()
}

fn decode(text: &str) -> Option<Files> {
    let v: serde_json::Map<String, Value> = serde_json::from_str(text).ok()?;
    v.into_iter()
        .map(|(path, value)| {
            let bytes = match (value["t"].as_str(), value["b"].as_str()) {
                (Some(t), _) => t.as_bytes().to_vec(),
                (None, Some(b)) => STANDARD.decode(b).ok()?,
                _ => return None,
            };
            Some((PathBuf::from(path), bytes))
        })
        .collect()
}
