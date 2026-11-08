//! Signed records: `sign`, `append` of a signed record, `signed` and `signed_by`
//! (ledger/record-signing, ledger/signature-check).

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clm_ledger::Body;
use common::{clone_with_origin, commit, env, repo, sh};
use serde_json::{Map, Value};

fn approved(fingerprint: &str) -> Body {
    Body::Approved {
        fingerprint: fingerprint.into(),
        branch: "spec/x".into(),
        change: Some("x".into()),
    }
}

/// A fresh ed25519 key pair at `dir/<name>`: the private key's path and the public key.
fn keygen(dir: &Path, name: &str) -> (PathBuf, String) {
    let key = dir.join(name);
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen -t ed25519");
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    (key, public.trim().to_string())
}

/// Sign with `key` from now on.
fn use_key(repo: &Path, key: &Path) {
    sh(repo, &["config", "gpg.format", "ssh"]);
    sh(repo, &["config", "user.signingkey", key.to_str().unwrap()]);
}

/// The object without `sig`, keys sorted, no whitespace, built here independently.
fn message(object: &Map<String, Value>) -> String {
    let sorted: BTreeMap<&String, &Value> = object.iter().filter(|(k, _)| *k != "sig").collect();
    serde_json::to_string(&sorted).unwrap()
}

/// `ssh-keygen -Y verify` exits 0.
fn verifies(
    dir: &Path,
    signers: &str,
    person: &str,
    namespace: &str,
    sig: &str,
    msg: &str,
) -> bool {
    let (signers_file, sig_file) = (dir.join("signers"), dir.join("sig"));
    std::fs::write(&signers_file, signers).unwrap();
    std::fs::write(&sig_file, sig).unwrap();
    let mut child = Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(&signers_file)
        .args(["-I", person, "-n", namespace, "-s"])
        .arg(&sig_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(msg.as_bytes())
        .unwrap();
    child.wait().unwrap().success()
}

/// Isolated from the user's own git config.
fn vars(home: &Path) -> Vec<(&'static str, String)> {
    vec![(
        "GIT_CONFIG_GLOBAL",
        home.join("gitconfig").to_string_lossy().into_owned(),
    )]
}

#[test]
fn signed_and_appended() {
    // req:ledger/record-signing@12908fe7
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let clone = clone_with_origin(home, "a", "Ann");
    let keys = tempfile::tempdir().unwrap();
    let (key, public) = keygen(keys.path(), "ann");
    use_key(&clone, &key);
    let owned = vars(home);
    let vars: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let env = env(home, &vars);

    let signed = clm_ledger::sign(&clone, approved("f"), "clm-approve", &env).unwrap();
    assert!(signed.sig.starts_with("-----BEGIN SSH SIGNATURE-----"));
    assert!(signed.record.writer.starts_with("ann/"));
    let out = clm_ledger::append(&clone, signed.clone(), &env).unwrap();
    assert_eq!(out.record, signed.record, "id, writer and at kept");

    let branch = format!("clm/ledger/{}", signed.record.writer);
    let text = sh(&clone, &["show", &format!("{branch}:records.jsonl")]);
    let line: Map<String, Value> = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(line["sig"], signed.sig.as_str());
    assert_eq!(line["id"], signed.record.id.as_str());
    let signers = format!("ann {public}\n");
    let msg = message(&line);
    assert!(verifies(
        keys.path(),
        &signers,
        "ann",
        "clm-approve",
        &signed.sig,
        &msg
    ));
    // Not under another namespace.
    assert!(!verifies(
        keys.path(),
        &signers,
        "ann",
        "clm-land",
        &signed.sig,
        &msg
    ));

    // The fold reads it, `sig` and all, beside an unsigned one.
    clm_ledger::append(&clone, approved("g"), &env).unwrap();
    let fold = clm_ledger::fold(&clone).unwrap();
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.approvals["f"][0], signed.record);
    assert_eq!(fold.lines[&signed.record.id], line);
    let unsigned = &fold.approvals["g"][0];
    assert!(!fold.lines[&unsigned.id].contains_key("sig"));
}

#[test]
fn signed_message_keeps_the_line_as_written() {
    // req:ledger/record-signing@12908fe7
    // A land with no code key and no change: `code_key` stays out, `changes` stays `[]`.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let clone = clone_with_origin(home, "a", "Ann");
    std::fs::write(clone.join("clm.toml"), "host = \"none\"\n").unwrap();
    let keys = tempfile::tempdir().unwrap();
    let (key, public) = keygen(keys.path(), "ann");
    use_key(&clone, &key);
    let owned = vars(home);
    let vars: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let env = env(home, &vars);
    let body = Body::Landed {
        commit: "c".into(),
        branch: "impl/x".into(),
        kind: clm_ledger::LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: None,
        changes: vec![],
    };
    let signed = clm_ledger::sign(&clone, body, "clm-land", &env).unwrap();
    let line = signed.line();
    assert!(!line.contains_key("code_key"));
    assert_eq!(line["changes"], Value::Array(vec![]));
    assert_eq!(line["review"], Value::Bool(false));
    assert_eq!(clm_ledger::signed_message(&line), message(&line));
    let signers = format!("ann {public}\n");
    assert!(verifies(
        keys.path(),
        &signers,
        "ann",
        "clm-land",
        &signed.sig,
        &message(&line)
    ));
}

#[test]
fn relative_key_and_another_writers_record() {
    // req:ledger/record-signing@12908fe7
    // A relative `user.signingkey` is taken from the repository root; a record signed as
    // another writer isn't appended to their branch from here.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let clone = clone_with_origin(home, "a", "Ann");
    let (_, public) = keygen(home, "ann");
    sh(&clone, &["config", "gpg.format", "ssh"]);
    sh(&clone, &["config", "user.signingkey", "../ann"]);
    let global = home.join("gitconfig");
    let global = global.to_str().unwrap();
    let as_dana = [("GIT_CONFIG_GLOBAL", global), ("CLM_WRITER", "dana")];
    let signed =
        clm_ledger::sign(&clone, approved("f"), "clm-approve", &env(home, &as_dana)).unwrap();
    let msg = clm_ledger::signed_message(&signed.line());
    let signers = format!("dana {public}\n");
    assert!(verifies(
        home,
        &signers,
        "dana",
        "clm-approve",
        &signed.sig,
        &msg
    ));
    let as_ann = [("GIT_CONFIG_GLOBAL", global)];
    let err = clm_ledger::append(&clone, signed, &env(home, &as_ann))
        .unwrap_err()
        .to_string();
    assert!(err.contains("dana/"), "{err}");
    assert_eq!(sh(&clone, &["branch", "--list", "clm/*"]), "");
}

#[test]
fn no_key() {
    // req:ledger/record-signing@12908fe7
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let clone = clone_with_origin(home, "a", "Ann");
    let owned = vars(home);
    let vars: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let env = env(home, &vars);
    let err = clm_ledger::sign(&clone, approved("f"), "clm-approve", &env)
        .unwrap_err()
        .to_string();
    assert!(err.contains("user.signingkey"), "{err}");
    assert!(err.contains("gpg.format"), "{err}");
    assert_eq!(
        sh(&clone, &["branch", "--list", "clm/*"]),
        "",
        "nothing appended"
    );
    // A key without `gpg.format = ssh` isn't an SSH signing key.
    sh(&clone, &["config", "user.signingkey", "/nowhere"]);
    let err = clm_ledger::sign(&clone, approved("f"), "clm-approve", &env)
        .unwrap_err()
        .to_string();
    assert!(err.contains("user.signingkey"), "{err}");
}

/// A repository whose second commit adds `docs/allowed_signers` listing `dana`'s key, and
/// keys for `dana` and `christopher-lunny`: the commits without and with the file, and the keys.
struct Signers {
    _keys: tempfile::TempDir,
    without: String,
    with: String,
    dana: PathBuf,
    christopher: PathBuf,
}

fn signers(root: &Path) -> Signers {
    repo(root, "T");
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let without = commit(root, &[], "settings");
    let keys = tempfile::tempdir().unwrap();
    let (dana, dana_pub) = keygen(keys.path(), "dana");
    let (christopher, christopher_pub) = keygen(keys.path(), "christopher");
    let file = format!("dana {dana_pub}\nchristopher-lunny {christopher_pub}\n");
    let with = commit(root, &[("docs/allowed_signers", &file)], "signers");
    Signers {
        _keys: keys,
        without,
        with,
        dana,
        christopher,
    }
}

/// `body` signed under `namespace` by writer `person`, with `key`.
fn signed_line(
    root: &Path,
    person: &str,
    key: &Path,
    namespace: &str,
    body: Body,
) -> Map<String, Value> {
    use_key(root, key);
    let global = root.join("no-gitconfig");
    let global = global.to_str().unwrap();
    let vars = [("CLM_WRITER", person), ("GIT_CONFIG_GLOBAL", global)];
    let env = env(root, &vars);
    clm_ledger::sign(root, body, namespace, &env)
        .unwrap()
        .line()
}

#[test]
fn signed_is_the_file() {
    // req:ledger/signature-check@8b219b3b
    let dir = tempfile::tempdir().unwrap();
    let s = signers(dir.path());
    assert!(clm_ledger::signed(dir.path(), &s.with).unwrap());
    assert!(!clm_ledger::signed(dir.path(), &s.without).unwrap());
}

#[test]
fn listed_signer() {
    // req:ledger/signature-check@8b219b3b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    assert!(line["writer"].as_str().unwrap().starts_with("dana/"));
    assert!(clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
    // No `docs/allowed_signers` at the commit: no signer is listed.
    assert!(!clm_ledger::signed_by(root, &s.without, &line, "clm-approve").unwrap());
}

#[test]
fn another_persons_name() {
    // req:ledger/signature-check@8b219b3b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let line = signed_line(root, "dana", &s.christopher, "clm-approve", approved("f"));
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}

#[test]
fn edited_after_signing() {
    // req:ledger/signature-check@8b219b3b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let mut line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    line.insert("fingerprint".into(), "g".into());
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}

#[test]
fn wrong_namespace_or_unsigned() {
    // req:ledger/signature-check@8b219b3b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let mut line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-land").unwrap());
    line.remove("sig");
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}
