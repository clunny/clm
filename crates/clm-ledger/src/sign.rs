//! Signed records (`signed-ledger-records`, `ledger/record-signing`,
//! `ledger/signature-check`): `sign`, `signed` and `signed_by`. `ssh-keygen` runs through
//! `clm-git`'s `run`, as any program the crate runs does.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::{Body, Error, Record, signed_message};

/// Where a signed repository lists its signers, at the top of the tree.
const SIGNERS: &str = "docs/allowed_signers";

/// A record with its signature, as `sign` returns it and `append` writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub record: Record,
    /// The armored `-----BEGIN SSH SIGNATURE-----` block over the record's signed message.
    pub sig: String,
}

impl Signed {
    /// The record's line as `append` writes it: its JSON object, `sig` included.
    pub fn line(&self) -> Map<String, Value> {
        let mut line = unsigned_line(&self.record);
        line.insert("sig".into(), Value::String(self.sig.clone()));
        line
    }
}

fn unsigned_line(record: &Record) -> Map<String, Value> {
    match serde_json::to_value(record).expect("a record serializes") {
        Value::Object(object) => object,
        _ => unreachable!("a record is an object"),
    }
}

/// `body` as a record with a fresh `id`, the writer and the time, as `append` would fill
/// them, signed under `namespace` with git's `user.signingkey` when `gpg.format` is `ssh`.
/// `env` is the environment `config` reads; its `HOME`, `XDG_CONFIG_HOME` and
/// `GIT_CONFIG_GLOBAL` are the ones git reads its config with.
pub fn sign(
    root: &Path,
    body: Body,
    namespace: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Signed, Error> {
    let key = signing_key(root, env)?;
    let record = crate::append::fresh(root, body, env)?;
    let message = signed_message(&unsigned_line(&record));
    let key_arg = key.path.to_string_lossy().into_owned();
    let out = clm_git::run(
        root,
        "ssh-keygen",
        &["-Y", "sign", "-f", &key_arg, "-n", namespace],
        clm_git::Options {
            stdin: Some(message.as_bytes()),
            env: &[],
        },
    )
    .map_err(|e| Error::Signing(format!("can't sign the record: {e}")))?;
    let sig = String::from_utf8_lossy(&out).trim().to_string();
    Ok(Signed { record, sig })
}

/// The key `ssh-keygen` signs with: a path, or a literal public key written to a temporary
/// file, so that `ssh-keygen` signs through `ssh-agent`.
struct Key {
    path: PathBuf,
    _literal: Option<tempfile::TempDir>,
}

fn signing_key(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<Key, Error> {
    let missing = || {
        Error::Signing(
            "no SSH signing key: set git's user.signingkey to your SSH key, and gpg.format \
             to ssh (git config gpg.format ssh)"
                .into(),
        )
    };
    let format = git_config(root, env, &["--get", "gpg.format"])?;
    if format.as_deref() != Some("ssh") {
        return Err(missing());
    }
    let Some(value) = git_config(root, env, &["--get", "--type=path", "user.signingkey"])?
        .filter(|v| !v.is_empty())
    else {
        return Err(missing());
    };
    if let Some(public) = value.strip_prefix("key::") {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("key.pub");
        std::fs::write(&path, format!("{public}\n"))?;
        return Ok(Key {
            path,
            _literal: Some(dir),
        });
    }
    // A relative path is taken from the repository root.
    Ok(Key {
        path: top(root)?.join(value),
        _literal: None,
    })
}

/// `git config <args>`, `None` when the key is unset, with git's config read as `env` says.
fn git_config(
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    args: &[&str],
) -> Result<Option<String>, Error> {
    let vars: Vec<(&str, OsString)> = ["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL"]
        .into_iter()
        .filter_map(|name| env(name).map(|v| (name, OsString::from(v))))
        .collect();
    let vars: Vec<(&str, &std::ffi::OsStr)> =
        vars.iter().map(|(k, v)| (*k, v.as_os_str())).collect();
    let mut full = vec!["config"];
    full.extend_from_slice(args);
    match clm_git::run(
        root,
        "git",
        &full,
        clm_git::Options {
            stdin: None,
            env: &vars,
        },
    ) {
        Ok(out) => Ok(Some(String::from_utf8_lossy(&out).trim().to_string())),
        // Exit 1: unset.
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// The top of the git repository `root` is in.
fn top(root: &Path) -> Result<PathBuf, Error> {
    Ok(PathBuf::from(clm_git::git(
        root,
        &["rev-parse", "--show-toplevel"],
    )?))
}

/// Whether `rev`'s tree holds `docs/allowed_signers`: the repository is signed there.
pub fn signed(root: &Path, rev: &str) -> Result<bool, Error> {
    holds(root, rev, SIGNERS)
}

/// Whether `rev`'s tree holds `path`, relative to its top.
fn holds(root: &Path, rev: &str, path: &str) -> Result<bool, Error> {
    let args = ["ls-tree", "--full-tree", "--name-only", rev, "--", path];
    let listed = clm_git::git(root, &args)?;
    Ok(listed.lines().any(|line| line == path))
}

/// The file at `path` in `rev`'s tree, relative to its top, or `None` when it has none.
pub(crate) fn file_at(root: &Path, rev: &str, path: &str) -> Result<Option<Vec<u8>>, Error> {
    if !holds(root, rev, path)? {
        return Ok(None);
    }
    let blob = format!("{rev}:{path}");
    let options = clm_git::Options::default();
    let args = ["cat-file", "blob", blob.as_str()];
    Ok(Some(clm_git::run(root, "git", &args, options)?))
}

/// Whether `line`, a record's JSON object as read, has a `sig` that `ssh-keygen -Y verify`
/// accepts under `namespace`, against `rev`'s `docs/allowed_signers` for the person part of
/// its `writer`. False for a line without `sig` or a `rev` without the file; an error when
/// `ssh-keygen` can't be run, so a missing tool is never taken for a forged record.
pub fn signed_by(
    root: &Path,
    rev: &str,
    line: &Map<String, Value>,
    namespace: &str,
) -> Result<bool, Error> {
    let (Some(sig), Some(writer)) = (
        line.get("sig").and_then(Value::as_str),
        line.get("writer").and_then(Value::as_str),
    ) else {
        return Ok(false);
    };
    let person = crate::person(writer);
    let signers = match file_at(root, rev, SIGNERS)? {
        Some(signers) if !person.is_empty() => signers,
        _ => return Ok(false),
    };
    let dir = tempfile::tempdir()?;
    let (signers_file, sig_file) = (dir.path().join("allowed_signers"), dir.path().join("sig"));
    std::fs::write(&signers_file, signers)?;
    std::fs::write(&sig_file, format!("{}\n", sig.trim_end()))?;
    let (signers_arg, sig_arg) = (
        signers_file.to_string_lossy().into_owned(),
        sig_file.to_string_lossy().into_owned(),
    );
    let message = signed_message(line);
    let args = [
        "-Y",
        "verify",
        "-f",
        &signers_arg,
        "-I",
        person,
        "-n",
        namespace,
        "-s",
        &sig_arg,
    ];
    let options = clm_git::Options {
        stdin: Some(message.as_bytes()),
        env: &[],
    };
    match clm_git::run(root, "ssh-keygen", &args, options) {
        Ok(_) => Ok(true),
        Err(clm_git::Error::Failed { .. }) => Ok(false),
        Err(e) => Err(Error::Signing(format!("can't check a signature: {e}"))),
    }
}
