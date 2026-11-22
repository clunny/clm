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
    // A commit on `branch` that `origin/main` holds too, so the gate finds nothing cut.
    for args in [
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "base",
        ][..],
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    ] {
        let ok = Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(ok.success(), "git {args:?}");
    }
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
    // req:host/azure-pull-requests@0c59d49d
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
    // req:host/azure-pull-requests@0c59d49d
    let created = pull_request(12);
    let answers = [("pr list", "[]"), ("pr create", created.as_str())];
    with_fakes(&answers, "spec/x", "the body", |root, dir| {
        // A relative body file is the root's, whatever directory the caller runs in, and
        // the argument `az` gets names it absolutely.
        Azure
            .page_open(root, "spec/x", "x", Path::new("body.md"))
            .unwrap();
        let want = format!("--description @{}", root.join("body.md").display());
        assert!(calls(dir, "az").contains(&want), "{}", calls(dir, "az"));
    });
}

#[test]
fn already_open() {
    // req:host/azure-pull-requests@0c59d49d
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
    // req:host/azure-pull-requests@0c59d49d
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
    // req:host/azure-pull-requests@0c59d49d
    with_fakes(&[], "spec/x", "  \n", |root, dir| {
        assert!(Azure.page_body(root, &root.join("body.md")).is_err());
        assert!(Azure.page_body(root, &root.join("missing.md")).is_err());
        assert_eq!(calls(dir, "az"), "");
    });
}

#[test]
fn no_active_pull_request_updates_nothing() {
    // req:host/azure-pull-requests@0c59d49d
    with_fakes(&[("pr list", "[]")], "spec/x", "b", |root, dir| {
        let err = Azure.page_body(root, &root.join("body.md")).unwrap_err();
        assert!(matches!(err, Error::Refused(_)), "{err:?}");
        assert!(!calls(dir, "az").contains("pr update"));
    });
}

#[test]
fn az_not_signed_in() {
    // req:host/azure-pull-requests@0c59d49d
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
    // req:host/azure-other-operations@1b336bb9
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
    // req:host/azure-other-operations@1b336bb9
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
    // req:host/azure-unreadable-answer@44460c25
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
    // req:host/azure-unreadable-answer@44460c25
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
    // req:host/azure-unreadable-answer@44460c25
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
fn scenario_listed_without_fields() {
    // req:host/azure-unreadable-answer@44460c25
    let listed = format!("[{UNREAD}]");
    let answers = [("pr list", listed.as_str()), ("", "{}")];
    with_fakes(&answers, "spec/x", "## Intent", |root, dir| {
        let file = root.join("body.md");
        let opened = Azure.page_open(root, "spec/x", "x", &file).unwrap_err();
        let updated = Azure.page_body(root, &file).unwrap_err();
        for err in [opened, updated] {
            let Error::Git(why) = &err else {
                panic!("{err:?}")
            };
            assert!(
                why.contains("no pullRequestId or repository.webUrl in the answer"),
                "{why}"
            );
        }
        let az = calls(dir, "az");
        assert!(
            !az.contains("pr create") && !az.contains("pr update"),
            "{az}"
        );
    });
}

#[test]
fn the_answer_is_cut_to_500_characters() {
    // req:host/azure-unreadable-answer@44460c25
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
    // req:host/azure-unreadable-answer@44460c25
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
    format!("{}/clm/pr-bodies/spec-x.markdown", common_dir(root))
}

#[test]
fn a_long_body_is_cut() {
    // req:host/azure-body-limit@905d72db
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
    // req:host/azure-body-limit@905d72db
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
    // req:host/azure-body-limit@905d72db
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
    // req:host/azure-body-limit@905d72db
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
    // req:host/azure-body-limit@905d72db
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
    // req:host/azure-body-limit@905d72db
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

/// Commits on the repository's branch a spec of component `a` holding `slugs`' requirements.
fn commit_spec(root: &Path, slugs: &[&str]) {
    let file = |rel: &str, text: &str| {
        fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
        fs::write(root.join(rel), text).unwrap();
    };
    file(
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n\n## Design\nr\n\n## Requirements\n",
    );
    let mut a = "---\ntitle: a\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n".to_string();
    for slug in slugs {
        a.push_str(&format!(
            "\n### Requirement: {slug}\n{slug} SHALL hold.\n\n#### Scenario: {slug}\n- GIVEN g\n- WHEN w\n- THEN t\n"
        ));
    }
    file("specs/a/SPEC.md", &a);
    for args in [
        &["add", "specs"][..],
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "spec",
        ],
    ] {
        let ok = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(ok.success(), "git {args:?}");
    }
}

/// Makes the repository's branch a spec branch that removes `a/x` against `origin/main`.
fn cut_a_x(root: &Path) {
    commit_spec(root, &["x", "y"]);
    let ok = Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", "HEAD"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(ok.success());
    commit_spec(root, &["y"]);
}

const CUTS: &str = "## Cuts\n- specs/a/SPEC.md: gate/cut: requirement a/x is gone\n\n";

#[test]
fn the_cuts_come_before_the_cut_to_fit() {
    // req:host/pr-open@26605e35
    // req:host/pr-body@08a50617
    let body = long_body();
    for listed in ["[]".to_string(), format!("[{}]", pull_request(12))] {
        with_fakes(&[], "spec/x", &body, |root, dir| {
            cut_a_x(root);
            fake_az_keeping_descriptions(dir, &listed);
            let file = root.join("body.md");
            if listed == "[]" {
                Azure.page_open(root, "spec/x", "x", &file).unwrap();
            } else {
                Azure.page_body(root, &file).unwrap();
            }
            let sent = fs::read_to_string(dir.join("description")).unwrap();
            assert!(units(&sent) <= 4000, "{}", units(&sent));
            assert!(sent.starts_with(&format!("{CUTS}line 000")), "{sent}");
            let kept = fs::read_to_string(kept_path(root)).unwrap();
            assert_eq!(kept, format!("{CUTS}{body}"));
            assert_eq!(fs::read_to_string(&file).unwrap(), body);
        });
    }
}

/// `clm pr <args>` through `clm-host`'s call in `root`, whose `clm.toml` sets `host = "azure"`:
/// status, output and error buffers.
fn pr_call(root: &Path, args: &[&str]) -> (u8, String, String) {
    fs::write(root.join("clm.toml"), "host = \"azure\"\n").unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_host::pr_command(args.iter().copied(), root, &|_| None, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn the_cut_is_said_on_standard_error_only() {
    // req:host/azure-body-limit@905d72db
    with_fakes(&[], "spec/x", &long_body(), |root, dir| {
        fake_az_keeping_descriptions(dir, "[]");
        let args = ["open", "spec/x", "--title", "x", "--body-file", "body.md"];
        let (code, out, err) = pr_call(root, &args);
        let said = format!(
            "cut the body to 4000 characters; the whole is kept at {}\n",
            kept_path(root)
        );
        assert_eq!(
            (code, out, err),
            (0, format!("{}\n", url(12)), said.clone())
        );
        // `pr body` says it the same way.
        fake_az_keeping_descriptions(dir, &format!("[{}]", pull_request(12)));
        let (code, out, err) = pr_call(root, &["body", "body.md"]);
        assert_eq!((code, out, err), (0, "updated #12\n".to_string(), said));
        // A body within the limit says nothing.
        fs::write(root.join("body.md"), "short").unwrap();
        let (code, _, err) = pr_call(root, &["body", "body.md"]);
        assert_eq!((code, err.as_str()), (0, ""));
    });
}

#[test]
fn a_listing_that_is_not_a_list_exits_2() {
    // req:host/host-tool-failure@5ca96db4
    let answers = [("pr list", r#"{"message": "x"}"#), ("pr create", "{}")];
    with_fakes(&answers, "spec/x", "b", |root, dir| {
        let args = ["open", "spec/x", "--title", "x", "--body-file", "body.md"];
        let (code, out, err) = pr_call(root, &args);
        assert_eq!((code, out.as_str()), (2, ""), "{err}");
        assert!(err.contains("not a list"), "{err}");
        assert!(!calls(dir, "az").contains("pr create"));
    });
}
