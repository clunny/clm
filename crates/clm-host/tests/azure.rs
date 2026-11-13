//! The `Azure` adapter (host/azure-pull-requests, host/azure-other-operations), against a
//! fake `az` and a fake `gh`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use clm_host::{Azure, Error, Host};
use tempfile::TempDir;

/// The tests put fakes first on the process's `PATH`, one test at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

const WEB: &str = "https://dev.azure.com/o/p/_git/r";

/// Writes a fake `program` in `dir`: it records each call's arguments on a line of
/// `<program>.args`, and answers with the text of the first of `answers` whose needle the
/// arguments hold. An answer starting `FAIL:` is printed on stderr and the fake exits 1.
fn fake(dir: &Path, program: &str, answers: &[(&str, &str)]) {
    let mut script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}/{program}.args\nall=\"$*\"\n",
        dir.display()
    );
    for (needle, out) in answers {
        script += &match out.strip_prefix("FAIL:") {
            Some(why) => {
                format!("case \"$all\" in *'{needle}'*) echo '{why}' >&2; exit 1;; esac\n")
            }
            None => {
                format!("case \"$all\" in *'{needle}'*) cat <<'EOT'\n{out}\nEOT\nexit 0;; esac\n")
            }
        };
    }
    script += "echo 'unexpected call' >&2\nexit 1\n";
    let path = dir.join(program);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs `body` with fake `az` and `gh` first on `PATH`, in a git repository on `branch`
/// whose `body.md` holds `body_text`. `body` gets the repository and the fakes' directory.
fn with_fakes<T>(
    az: &[(&str, &str)],
    branch: &str,
    body_text: &str,
    body: impl FnOnce(&Path, &Path) -> T,
) -> T {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    fake(dir.path(), "az", az);
    fake(dir.path(), "gh", &[]);
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", dir.path().display())) };
    let repo = TempDir::new().unwrap();
    let init = Command::new("git")
        .args(["init", "-q", "-b", branch])
        .current_dir(repo.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(init.success());
    fs::write(repo.path().join("body.md"), body_text).unwrap();
    let out = body(repo.path(), dir.path());
    unsafe { std::env::set_var("PATH", old) };
    out
}

fn calls(dir: &Path, program: &str) -> String {
    fs::read_to_string(dir.join(format!("{program}.args"))).unwrap_or_default()
}

fn pull_request(id: u32) -> String {
    format!(r#"{{"pullRequestId":{id},"repository":{{"webUrl":"{WEB}"}}}}"#)
}

fn url(id: u32) -> String {
    format!("{WEB}/pullrequest/{id}")
}

#[test]
fn opened() {
    // req:host/azure-pull-requests@f4a4b73c
    let created = pull_request(12);
    let answers = [("pr list", "[]"), ("pr create", created.as_str())];
    with_fakes(&answers, "spec/x", "the body", |root, dir| {
        let got = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        assert_eq!(got, Some(url(12)));
        let az = calls(dir, "az");
        let create = az.lines().find(|l| l.contains("repos pr create")).unwrap();
        for arg in [
            "--source-branch spec/x",
            "--target-branch main",
            "--title x",
            "--detect true",
            "--output json",
        ] {
            assert!(create.contains(arg), "{arg} missing from {create}");
        }
        let body = format!("--description @{}", root.join("body.md").display());
        assert!(create.contains(&body), "{body} missing from {create}");
        let list = az.lines().find(|l| l.contains("repos pr list")).unwrap();
        for arg in [
            "--source-branch spec/x",
            "--target-branch main",
            "--status active",
            "--detect true",
            "--output json",
        ] {
            assert!(list.contains(arg), "{arg} missing from {list}");
        }
        assert_eq!(calls(dir, "gh"), "");
    });
}

#[test]
fn a_relative_body_file_is_made_absolute() {
    // req:host/azure-pull-requests@f4a4b73c
    let created = pull_request(12);
    let answers = [("pr list", "[]"), ("pr create", created.as_str())];
    with_fakes(&answers, "spec/x", "the body", |root, dir| {
        // `az` runs in the repository's root, but the caller's file is relative to the
        // caller's directory, so the argument names it absolutely.
        let cwd = std::env::current_dir().unwrap();
        let name = "azure-test-relative-body.md";
        fs::write(cwd.join(name), "the body").unwrap();
        let out = Azure.page_open(root, "spec/x", "x", Path::new(name));
        fs::remove_file(cwd.join(name)).unwrap();
        out.unwrap();
        let want = format!("--description @{}", cwd.join(name).display());
        assert!(calls(dir, "az").contains(&want), "{}", calls(dir, "az"));
    });
}

#[test]
fn already_open() {
    // req:host/azure-pull-requests@f4a4b73c
    let listed = format!("[{}]", pull_request(12));
    with_fakes(
        &[("pr list", listed.as_str())],
        "spec/x",
        "b",
        |root, dir| {
            let got = Azure
                .page_open(root, "spec/x", "x", &root.join("body.md"))
                .unwrap();
            assert_eq!(got, Some(url(12)));
            assert!(!calls(dir, "az").contains("pr create"));
        },
    );
}

#[test]
fn body_updated() {
    // req:host/azure-pull-requests@f4a4b73c
    let listed = format!("[{}]", pull_request(12));
    let answers = [("pr list", listed.as_str()), ("pr update", "{}")];
    with_fakes(&answers, "spec/x", "new body", |root, dir| {
        let said = Azure.page_body(root, &root.join("body.md")).unwrap();
        assert_eq!(said.as_deref(), Some("updated #12"));
        let az = calls(dir, "az");
        let update = az.lines().find(|l| l.contains("repos pr update")).unwrap();
        let body = format!("--description @{}", root.join("body.md").display());
        assert!(
            update.contains("--id 12") && update.contains(&body),
            "{update}"
        );
        // Nothing else that changes the pull request: the whole call is exactly this.
        assert_eq!(
            update,
            format!("repos pr update --id 12 {body} --detect true --output json")
        );
    });
}

#[test]
fn a_blank_or_missing_body_calls_nothing() {
    // req:host/azure-pull-requests@f4a4b73c
    with_fakes(&[], "spec/x", "  \n", |root, dir| {
        assert!(Azure.page_body(root, &root.join("body.md")).is_err());
        assert!(Azure.page_body(root, &root.join("missing.md")).is_err());
        assert_eq!(calls(dir, "az"), "");
    });
}

#[test]
fn no_active_pull_request_updates_nothing() {
    // req:host/azure-pull-requests@f4a4b73c
    with_fakes(&[("pr list", "[]")], "spec/x", "b", |root, dir| {
        let err = Azure.page_body(root, &root.join("body.md")).unwrap_err();
        assert!(matches!(err, Error::Refused(_)), "{err:?}");
        assert!(!calls(dir, "az").contains("pr update"));
    });
}

#[test]
fn az_not_signed_in() {
    // req:host/azure-pull-requests@f4a4b73c
    let answers = [("repos", "FAIL:Please run 'az login'")];
    with_fakes(&answers, "spec/x", "b", |root, _| {
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        assert!(format!("{err:?}").contains("az login"), "{err:?}");
        let err = Azure.page_body(root, &root.join("body.md")).unwrap_err();
        assert!(format!("{err:?}").contains("az login"), "{err:?}");
    });
}

#[test]
fn issue_has_nowhere_to_go() {
    // req:host/azure-other-operations@1a35e857
    with_fakes(&[], "main", "the report", |root, dir| {
        let err = Azure
            .issue(root, "x", Path::new("body.md"), "clunny/clm")
            .unwrap_err();
        match err {
            Error::Nowhere(report) => assert_eq!(report, "the report"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            (calls(dir, "az"), calls(dir, "gh")),
            (String::new(), String::new())
        );
    });
}

#[test]
fn contributions_and_alerts_are_not_there() {
    // req:host/azure-other-operations@1a35e857
    with_fakes(&[], "main", "b", |root, dir| {
        let body = root.join("body.md");
        assert_eq!(Azure.parent(root).unwrap(), None);
        assert!(Azure.list(root).unwrap().is_empty());
        assert!(Azure.alerts(root).unwrap().is_empty());
        let no_host = |r: Result<(), Error>| match r {
            Err(Error::Refused(why)) => assert!(why.contains("no host"), "{why}"),
            other => panic!("{other:?}"),
        };
        no_host(Azure.show(root, 7).map(drop));
        no_host(Azure.sync(root));
        no_host(Azure.open_on_parent(root, "spec/x", "x", &body).map(drop));
        no_host(Azure.fetch(root, 7, "pr-7"));
        no_host(Azure.comment(root, 7, &body, true));
        no_host(Azure.label(root, 7, "x"));
        assert_eq!(
            (calls(dir, "az"), calls(dir, "gh")),
            (String::new(), String::new())
        );
    });
}

/// An `az` whose first `pr list` lists nothing and whose later ones print `later`, and whose
/// `pr create` prints `created`.
fn fake_az_listing_late(dir: &Path, later: &str, created: &str) {
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {d}/az.args\n\
         case \"$*\" in\n\
         *'pr list'*) if [ -e {d}/listed ]; then cat <<'EOT'\n{later}\nEOT\n\
         else touch {d}/listed; echo '[]'; fi;;\n\
         *'pr create'*) cat <<'EOT'\n{created}\nEOT\n;;\nesac\n",
        d = dir.display(),
    );
    let az = dir.join("az");
    fs::write(&az, script).unwrap();
    fs::set_permissions(&az, fs::Permissions::from_mode(0o755)).unwrap();
}

const UNREAD: &str = r#"{"status": "active"}"#;
const NO_FIELDS: &str = "az repos pr create: no pullRequestId or repository.webUrl in the answer: ";

#[test]
fn opened_but_unread() {
    // req:host/azure-unreadable-answer@d62c6d57
    let listed = format!("[{}]", pull_request(12));
    with_fakes(&[], "spec/x", "b", |root, dir| {
        fake_az_listing_late(dir, &listed, UNREAD);
        let got = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        assert_eq!(got, Some(url(12)));
        let az = calls(dir, "az");
        let lists: Vec<&str> = az.lines().filter(|l| l.contains("repos pr list")).collect();
        assert_eq!(lists.len(), 2, "{az}");
        for arg in [
            "--source-branch spec/x",
            "--target-branch main",
            "--status active",
        ] {
            assert!(lists[1].contains(arg), "{arg} missing from {}", lists[1]);
        }
    });
}

#[test]
fn nothing_to_read() {
    // req:host/azure-unreadable-answer@d62c6d57
    let answers = [
        ("pr list", "[]"),
        ("pr create", "{\"status\":\n   \"active\"}"),
    ];
    with_fakes(&answers, "spec/x", "b", |root, _| {
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        let Error::Git(why) = err else {
            panic!("{err:?}")
        };
        assert_eq!(why, format!("{NO_FIELDS}{UNREAD}"));
    });
}

#[test]
fn a_listing_without_fields_fails_too() {
    // req:host/azure-unreadable-answer@d62c6d57
    with_fakes(&[], "spec/x", "b", |root, dir| {
        fake_az_listing_late(dir, &format!("[{UNREAD}]"), UNREAD);
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        assert!(
            format!("{err:?}").contains(&format!("{NO_FIELDS}{}", UNREAD.replace('"', "\\\""))),
            "{err:?}"
        );
    });
}

#[test]
fn the_answer_is_cut_to_500_characters() {
    // req:host/azure-unreadable-answer@d62c6d57
    let long = format!("{{\"x\": \"{}\"}}", "a".repeat(900));
    let answers = [("pr list", "[]"), ("pr create", long.as_str())];
    with_fakes(&answers, "spec/x", "b", |root, _| {
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        let Error::Git(why) = err else {
            panic!("{err:?}")
        };
        let answer = why.strip_prefix(NO_FIELDS).unwrap();
        assert_eq!(answer.chars().count(), 500);
        assert!(long.starts_with(answer));
    });
}

#[test]
fn an_answer_that_is_not_json_names_the_parse_error() {
    // req:host/azure-unreadable-answer@d62c6d57
    let answers = [("pr list", "[]"), ("pr create", "not json")];
    with_fakes(&answers, "spec/x", "b", |root, _| {
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        assert!(format!("{err:?}").contains("bad JSON"), "{err:?}");
    });
}

/// An `az` that lists `listed` for `pr list`, answers `pr create` with pull request 12 and
/// `pr update` with `{}`, and copies the file each `--description @<file>` names to
/// `description.<n>` in `dir`, `n` counting the calls that carry one.
fn fake_az_keeping_descriptions(dir: &Path, listed: &str) {
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {d}/az.args\n\
         for a in \"$@\"; do case \"$a\" in @*) cp \"${{a#@}}\" {d}/description;; esac; done\n\
         case \"$*\" in\n*'pr list'*) echo '{listed}';;\n\
         *'pr create'*) echo '{created}';;\n*'pr update'*) echo '{{}}';;\nesac\n",
        d = dir.display(),
        created = pull_request(12),
    );
    let az = dir.join("az");
    fs::write(&az, script).unwrap();
    fs::set_permissions(&az, fs::Permissions::from_mode(0o755)).unwrap();
}

const NOTE_START: &str =
    "_Cut to fit Azure DevOps's 4000-character limit; the whole body is kept at ";

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

/// 300 lines of 20 characters, each ended by a line break.
fn long_body() -> String {
    (0..300)
        .map(|n| format!("line {n:03} {:-<11}\n", ""))
        .collect()
}

/// The clone's common git directory, as `git` prints it.
fn common_dir(root: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(root)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn kept_path(root: &Path) -> String {
    format!("{}/clm/pr-bodies/spec-x.md", common_dir(root))
}

#[test]
fn a_long_body_is_cut() {
    // req:host/azure-body-limit@70dcec44
    let body = long_body();
    assert_eq!(body.lines().next().unwrap().chars().count(), 20);
    with_fakes(&[], "spec/x", &body, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        let file = root.join("body.md");
        let got = Azure.page_open(root, "spec/x", "x", &file).unwrap();
        assert_eq!(got, Some(url(12)));
        let sent = fs::read_to_string(dir.join("description")).unwrap();
        let kept = kept_path(root);
        assert!(units(&sent) <= 4000, "{}", units(&sent));
        assert!(sent.starts_with("line 000 -----------\n"), "{sent}");
        let note = format!("{NOTE_START}{kept}._");
        assert!(sent.ends_with(&format!("\n\n{note}")), "{sent}");
        // The cut part ends at a line break: every line before the note is whole.
        let cut = sent.strip_suffix(&format!("\n\n{note}")).unwrap();
        assert!(body.starts_with(cut) && body[cut.len()..].starts_with('\n'));
        assert_eq!(fs::read_to_string(&kept).unwrap(), body);
        assert_eq!(fs::read_to_string(&file).unwrap(), body);
        // The longest part that leaves room: one more line would not fit.
        let next = body[cut.len() + 1..].lines().next().unwrap();
        assert!(units(cut) + 1 + units(next) + 2 + units(&note) > 4000);
        // The temporary file the description came from is gone.
        let used = calls(dir, "az");
        let temp = used
            .split("--description @")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .unwrap();
        assert!(!Path::new(temp).exists(), "{temp}");
    });
}

#[test]
fn a_short_body_is_passed_as_it_is() {
    // req:host/azure-body-limit@70dcec44
    let body: String = long_body()
        .lines()
        .take(10)
        .map(|l| format!("{l}\n"))
        .collect();
    with_fakes(&[], "spec/x", &body, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        let file = root.join("body.md");
        Azure.page_open(root, "spec/x", "x", &file).unwrap();
        assert_eq!(fs::read_to_string(dir.join("description")).unwrap(), body);
        assert!(calls(dir, "az").contains(&format!("--description @{}", file.display())));
        assert!(!Path::new(&kept_path(root)).exists());
    });
}

#[test]
fn four_thousand_units_fit_and_one_more_does_not() {
    // req:host/azure-body-limit@70dcec44
    // A surrogate pair counts as two units.
    let fits = format!("{}\u{1F600}", "a".repeat(3998));
    assert_eq!(units(&fits), 4000);
    with_fakes(&[], "spec/x", &fits, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        assert_eq!(fs::read_to_string(dir.join("description")).unwrap(), fits);
    });
    let over = format!("{fits}a");
    with_fakes(&[], "spec/x", &over, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        let sent = fs::read_to_string(dir.join("description")).unwrap();
        assert!(sent.contains(NOTE_START), "{sent}");
    });
}

#[test]
fn a_body_without_a_line_break_is_cut_at_a_character() {
    // req:host/azure-body-limit@70dcec44
    // Emoji are two units each; the cut never splits one.
    let body = "\u{1F600}".repeat(3000);
    with_fakes(&[], "spec/x", &body, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        let sent = fs::read_to_string(dir.join("description")).unwrap();
        let note = format!("{NOTE_START}{}._", kept_path(root));
        let cut = sent.strip_suffix(&format!("\n\n{note}")).unwrap();
        assert!(cut.chars().all(|c| c == '\u{1F600}'));
        assert!(
            units(&sent) <= 4000 && units(&sent) + 2 > 4000,
            "{}",
            units(&sent)
        );
        assert_eq!(fs::read_to_string(kept_path(root)).unwrap(), body);
    });
}

#[test]
fn the_cut_trims_trailing_line_breaks() {
    // req:host/azure-body-limit@70dcec44
    let body = format!("{}\n\n\n{}", "a".repeat(100), "b".repeat(5000));
    with_fakes(&[], "spec/x", &body, |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        let sent = fs::read_to_string(dir.join("description")).unwrap();
        assert!(
            sent.starts_with(&format!("{}\n\n{NOTE_START}", "a".repeat(100))),
            "{sent}"
        );
    });
}

#[test]
fn an_update_is_cut_too() {
    // req:host/azure-body-limit@70dcec44
    let body = long_body();
    let listed = format!("[{}]", pull_request(12));
    with_fakes(&[], "spec/x", &body, |root, dir| {
        fake_az_keeping_descriptions(dir, &listed);
        let said = Azure.page_body(root, &root.join("body.md")).unwrap();
        assert_eq!(said.as_deref(), Some("updated #12"));
        let sent = fs::read_to_string(dir.join("description")).unwrap();
        assert!(units(&sent) <= 4000);
        assert!(
            sent.ends_with(&format!("{NOTE_START}{}._", kept_path(root))),
            "{sent}"
        );
        assert_eq!(fs::read_to_string(kept_path(root)).unwrap(), body);
        assert!(calls(dir, "az").contains("repos pr update --id 12"));
    });
}

/// The test binary runs itself on this one with `CLM_CUT_CHILD` set, to read what the cut
/// writes to standard error.
#[test]
fn the_cut_child() {
    if std::env::var_os("CLM_CUT_CHILD").is_none() {
        return;
    }
    with_fakes(&[], "spec/x", &long_body(), |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        let got = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        println!("URL {}", got.unwrap());
        println!("KEPT {}", kept_path(root));
    });
}

#[test]
fn the_cut_is_said_on_standard_error_only() {
    // req:host/azure-body-limit@70dcec44
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "the_cut_child",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("CLM_CUT_CHILD", "1")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let kept = stdout
        .lines()
        .find_map(|l| l.strip_prefix("KEPT "))
        .unwrap();
    let said = format!("cut the body to 4000 characters; the whole is kept at {kept}\n");
    assert!(stderr.contains(&said), "{stderr}");
    assert!(!stdout.contains("cut the body"), "{stdout}");
    assert!(stdout.contains(&format!("URL {}", url(12))));
}
