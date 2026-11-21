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
    // req:ledger/record-signing@13840103
    // req:ledger/append@bd683cb9
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
    // req:ledger/record-signing@13840103
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
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
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
    // req:ledger/record-signing@13840103
    // req:ledger/append@bd683cb9
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
    let signer = signed.record.writer.clone();
    let err = clm_ledger::append(&clone, signed, &env(home, &as_ann))
        .unwrap_err()
        .to_string();
    assert!(signer.starts_with("dana/"), "{signer}");
    let ann = clm_ledger::writer(&clone, &env(home, &as_ann)).unwrap();
    assert_eq!(
        err,
        format!("the record is signed as writer {signer}, but this clone writes as {ann}")
    );
    assert_eq!(sh(&clone, &["branch", "--list", "clm/*"]), "");
}

#[test]
fn no_key() {
    // req:ledger/record-signing@13840103
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
    // req:ledger/signature-check@5f3c7856
    let dir = tempfile::tempdir().unwrap();
    let s = signers(dir.path());
    assert!(clm_ledger::signed(dir.path(), &s.with).unwrap());
    assert!(!clm_ledger::signed(dir.path(), &s.without).unwrap());
}

#[test]
fn listed_signer() {
    // req:ledger/signature-check@5f3c7856
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
    // req:ledger/signature-check@5f3c7856
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let line = signed_line(root, "dana", &s.christopher, "clm-approve", approved("f"));
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}

#[test]
fn edited_after_signing() {
    // req:ledger/signature-check@5f3c7856
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let mut line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    line.insert("fingerprint".into(), "g".into());
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}

#[test]
fn wrong_namespace_or_unsigned() {
    // req:ledger/signature-check@5f3c7856
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let mut line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-land").unwrap());
    line.remove("sig");
    assert!(!clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
}

/// Runs `run` with `PATH` holding only `git`, so no `ssh-keygen` is found, and `name` set to
/// `value` on the process: the tools `clm-ledger` starts inherit both. Tests run one per
/// process (`ci/nextest-runner`), so no other test sees them; the old values come back when
/// `run` returns or panics.
fn on_the_process<R>(vars: &[(&str, &std::ffi::OsStr)], run: impl FnOnce() -> R) -> R {
    struct Restore(Vec<(String, Option<std::ffi::OsString>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (name, old) in self.0.drain(..) {
                match old {
                    // SAFETY: one test per process; nothing else reads the environment meanwhile.
                    Some(old) => unsafe { std::env::set_var(&name, old) },
                    // SAFETY: as above.
                    None => unsafe { std::env::remove_var(&name) },
                }
            }
        }
    }
    let _restore = Restore(
        vars.iter()
            .map(|(name, _)| (name.to_string(), std::env::var_os(name)))
            .collect(),
    );
    for (name, value) in vars {
        // SAFETY: as above.
        unsafe { std::env::set_var(name, value) };
    }
    run()
}

/// A directory holding only a link to the `git` on `PATH`.
fn git_alone() -> tempfile::TempDir {
    let path = std::env::var_os("PATH").unwrap();
    let git = std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|git| git.is_file())
        .expect("git on PATH");
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(git, dir.path().join("git")).unwrap();
    dir
}

/// An `ssh-agent` serving a socket of its own until dropped.
struct Agent {
    child: std::process::Child,
    socket: PathBuf,
    _dir: tempfile::TempDir,
}

impl Agent {
    fn start() -> Agent {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("agent");
        let child = Command::new("ssh-agent")
            .arg("-D")
            .arg("-a")
            .arg(&socket)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        while !socket.exists() {
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "ssh-agent made no socket"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Agent {
            child,
            socket,
            _dir: dir,
        }
    }

    fn add(&self, key: &Path) {
        let ok = Command::new("ssh-add")
            .arg("-q")
            .arg(key)
            .env("SSH_AUTH_SOCK", &self.socket)
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(ok, "ssh-add");
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn signing_refused() {
    // req:ledger/record-signing@13840103
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let clone = clone_with_origin(home, "a", "Ann");
    let owned = vars(home);
    let vars: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let env = env(home, &vars);
    // A key, but no `gpg.format`.
    let (key, public) = keygen(home, "ann");
    sh(
        &clone,
        &["config", "user.signingkey", key.to_str().unwrap()],
    );
    match clm_ledger::sign(&clone, approved("f"), "clm-approve", &env) {
        Err(clm_ledger::Error::Signing(m)) => assert_eq!(
            m,
            "no SSH signing key: set git's user.signingkey to your SSH key, and gpg.format to \
             ssh (git config gpg.format ssh)"
        ),
        other => panic!("{other:?}"),
    }
    // `key::` and the public half of a key an agent holds.
    let agent = Agent::start();
    agent.add(&key);
    sh(&clone, &["config", "gpg.format", "ssh"]);
    sh(
        &clone,
        &["config", "user.signingkey", &format!("key::{public}")],
    );
    let socket = [("SSH_AUTH_SOCK", agent.socket.as_os_str())];
    let signed = on_the_process(&socket, || {
        clm_ledger::sign(&clone, approved("f"), "clm-approve", &env)
    })
    .unwrap();
    let msg = clm_ledger::signed_message(&signed.line());
    let signers = format!("ann {public}\n");
    let keys = tempfile::tempdir().unwrap();
    assert!(verifies(
        keys.path(),
        &signers,
        "ann",
        "clm-approve",
        &signed.sig,
        &msg
    ));
    // No `ssh-keygen` on `PATH`.
    sh(&clone, &["config", "user.signingkey", "../ann"]);
    let git = git_alone();
    let path = [("PATH", git.path().as_os_str())];
    match on_the_process(&path, || {
        clm_ledger::sign(&clone, approved("f"), "clm-approve", &env)
    }) {
        Err(clm_ledger::Error::Signing(m)) => assert!(
            m.starts_with("can't sign the record: ssh-keygen -Y sign -f "),
            "{m}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_tool() {
    // req:ledger/signature-check@5f3c7856
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let line = signed_line(root, "dana", &s.dana, "clm-approve", approved("f"));
    let mut no_writer = line.clone();
    no_writer.remove("writer");
    let git = git_alone();
    let path = [("PATH", git.path().as_os_str())];
    let (first, second) = on_the_process(&path, || {
        (
            clm_ledger::signed_by(root, &s.with, &line, "clm-approve"),
            clm_ledger::signed_by(root, &s.with, &no_writer, "clm-approve"),
        )
    });
    match first {
        Err(clm_ledger::Error::Signing(m)) => {
            assert!(m.starts_with("can't check a signature: "), "{m}")
        }
        other => panic!("{other:?}"),
    }
    assert!(!second.unwrap());
}

#[test]
fn one_person_part() {
    // req:ledger/request-cleared@3e4c06c9
    // An approval by the writer `dana`, with no `/`, signed with dana's key, clears dana/web's
    // earlier request, and `signed_by` checks it for `dana`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signers(root);
    let mut line: Map<String, Value> = serde_json::from_str(
        r#"{"id":"a1","kind":"approved","writer":"dana","at":"2026-01-02T00:00:00.000Z","fingerprint":"f","branch":"spec/x","change":null}"#,
    )
    .unwrap();
    let message = clm_ledger::signed_message(&line);
    let mut child = Command::new("ssh-keygen")
        .args(["-Y", "sign", "-n", "clm-approve", "-f"])
        .arg(&s.dana)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(message.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "ssh-keygen -Y sign");
    let sig = String::from_utf8(out.stdout).unwrap();
    line.insert("sig".into(), sig.trim().into());
    assert!(clm_ledger::signed_by(root, &s.with, &line, "clm-approve").unwrap());
    let request = r#"{"id":"r1","kind":"changes_requested","writer":"dana/web","at":"2026-01-01T00:00:00.000Z","branch":"spec/x","fingerprint":"f","text":"t"}"#;
    let text = format!("{request}\n{}\n", Value::Object(line));
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/dana/web", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert!(fold.cleared(&fold.change_requests["spec/x"][0]));
}
