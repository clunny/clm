//! Reading a commit's files from GitHub without a tarball: the tree listing, then the
//! text of many files per GraphQL query.

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

/// The paths the model reads.
pub fn is_spec_path(path: &str) -> bool {
    path == "ARCHITECTURE.md"
        || path == "CAPABILITIES.md"
        || ["specs/", "changes/", "backlog/", "docs/decisions/"]
            .iter()
            .any(|d| path.starts_with(d))
}

/// The Trees API's recursive listing of a commit's tree.
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

/// A commit's tree as the snapshot reads it: its blobs, the size of each regular file, and
/// whether GitHub left entries out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listed {
    pub blobs: Vec<TreeEntry>,
    /// Each regular file's (mode `100644` or `100755`) size in bytes, as stored: symlinks
    /// and submodules never count (`snapshot-size`).
    pub sizes: BTreeMap<String, u64>,
    pub truncated: bool,
}

/// The size of each regular file in the Trees API's JSON, by path. `keys::Listing` holds
/// `clm_gate::Entry`, which has no size, so sizes and modes are read here.
fn sizes_of(v: &Value) -> BTreeMap<String, u64> {
    v["tree"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["type"] == "blob" && matches!(e["mode"].as_str(), Some("100644" | "100755")))
        .filter_map(|e| Some((e["path"].as_str()?.to_string(), e["size"].as_u64()?)))
        .collect()
}

/// The Trees API's recursive listing of a commit's tree, with each regular file's size.
pub async fn listed<P: GitHub>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Listed> {
    let v = gh::get_ok(
        p,
        auth,
        &format!("/repos/{repo}/git/trees/{sha}?recursive=1"),
    )
    .await?;
    let listing = keys::listing_of(&v)?;
    Some(Listed {
        blobs: blobs(&listing),
        sizes: sizes_of(&v),
        truncated: listing.truncated,
    })
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
pub async fn tree<P: GitHub>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<TreeEntry>> {
    Some(listed(p, auth, repo, sha).await?.blobs)
}

/// Bytes git reads from the start of a file to decide it is binary, as `clm size` does.
const BINARY_PROBE: usize = 8000;

/// A blob as GitHub gave it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fetched {
    /// Its text; `None` for a binary blob, one missing from the commit, or truncated text
    /// that isn't UTF-8.
    pub text: Option<String>,
    /// GitHub says `isBinary`, or its text has a NUL byte in its first 8000 bytes.
    pub binary: bool,
}

/// Each path at `sha` as GitHub gives it, batched by GraphQL query.
pub async fn texts<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    paths: &[String],
) -> Option<BTreeMap<String, Fetched>> {
    let expressions: Vec<String> = paths.iter().map(|path| format!("{sha}:{path}")).collect();
    let fetched = fetch(p, auth, repo, "Blobs", &expressions, async |sha: &str| {
        blob(p, auth, repo, sha).await
    })
    .await?;
    Some(paths.iter().cloned().zip(fetched).collect())
}

/// The text of each `<rev>:<path>` expression, in order, `None` for a binary file or one
/// that doesn't exist, batched in GraphQL queries named `query`.
pub async fn objects<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    query: &str,
    expressions: &[String],
) -> Option<Vec<Option<String>>> {
    let fetched = fetch(p, auth, repo, query, expressions, async |sha: &str| {
        blob(p, auth, repo, sha).await
    })
    .await?;
    Some(fetched.into_iter().map(|f| f.text).collect())
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
    let fetched = fetch(p, auth, repo, query, expressions, async |sha: &str| {
        blob_direct(p, auth, repo, sha).await
    })
    .await?;
    Some(fetched.into_iter().map(|f| f.text).collect())
}

/// Each `<rev>:<path>` expression's blob, in order, batched in GraphQL queries named
/// `query`.
async fn fetch<P: GitHub>(
    p: &P,
    auth: &Auth,
    repo: &str,
    query: &str,
    expressions: &[String],
    blob: impl AsyncFn(&str) -> Option<Vec<u8>>,
) -> Option<Vec<Fetched>> {
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
                    let bytes = blob(sha).await?;
                    String::from_utf8(bytes).ok()
                }
                (text, _) => text.map(String::from),
            };
            let nul = text
                .as_ref()
                .is_some_and(|t| t.as_bytes()[..t.len().min(BINARY_PROBE)].contains(&0));
            out.push(Fetched {
                binary: object["isBinary"] == true || nul,
                text,
            });
        }
    }
    Some(out)
}

/// The bytes of a blob, from the Cache API or the blob API. A blob never changes.
async fn blob<P: Platform>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<u8>> {
    let key = format!("blob:{repo}:{sha}");
    if let Some(bytes) = p
        .cache_get(&key)
        .await
        .and_then(|b| STANDARD.decode(b).ok())
    {
        return Some(bytes);
    }
    let content = blob_content(p, auth, repo, sha).await?;
    p.cache_put(&key, &content, FILES_TTL).await;
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

/// Whether the model reads a file's bytes, not just its text: node and change assets.
fn is_asset(path: &str) -> bool {
    (path.starts_with("specs/") || path.starts_with("changes/")) && path.contains("/assets/")
}

/// The files a snapshot reads besides the model's: `clm.toml`, for the size budgets, and
/// every `.gitattributes`, for the files they leave out of sizes (`snapshot-size`).
fn is_size_path(path: &str) -> bool {
    path == "clm.toml" || path == ".gitattributes" || path.ends_with("/.gitattributes")
}

/// The files the model reads at `sha`, with `clm.toml` and every `.gitattributes`: from
/// the Cache API, or from GitHub. Binary assets
/// come through the blob API so their hashes match a checkout's, at most
/// `MAX_BLOB_FETCHES` per request; each is cached, so a later request picks up where this
/// one stopped. Binary files the model never reads are left out.
pub async fn spec_files<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    tree: Option<&[TreeEntry]>,
) -> Option<Files> {
    let key = format!("files2:{repo}@{sha}");
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
    let entries: Vec<&TreeEntry> = tree
        .iter()
        .filter(|e| is_spec_path(&e.path) || is_size_path(&e.path))
        .collect();
    let paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let texts = texts(p, auth, repo, sha, &paths).await?;
    let mut files = Files::new();
    let mut fetches = 0;
    let mut complete = true;
    for entry in entries {
        let bytes = match texts.get(&entry.path).and_then(|f| f.text.as_ref()) {
            Some(text) => text.clone().into_bytes(),
            _ if !is_asset(&entry.path) => continue,
            _ if fetches < MAX_BLOB_FETCHES => {
                fetches += 1;
                blob(p, auth, repo, &entry.sha).await?
            }
            _ => {
                complete = false;
                continue;
            }
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
