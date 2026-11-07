//! `git-host`: CLM with GitHub, a bare git server or no host at all.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::{Repo, landed, printed};

#[test]
fn bare_server() {
    // req:capability/git-host@3a12ab25
    let repo = Repo::init();
    // `origin` is a bare git server.
    repo.land_on_origin("true");

    // A `gh` on `PATH` that notes any call, so a host call shows.
    let bin = repo.root.parent().unwrap().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let called = bin.join("gh-called");
    let gh = bin.join("gh");
    fs::write(
        &gh,
        format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", called.display()),
    )
    .unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |args: &[&str]| repo.clm_env(args, "", &[("PATH", &path)]);
    let clm = |args: &[&str]| {
        let out = run(args);
        assert!(out.status.success(), "clm {args:?}: {}", printed(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // Propose: a change that needs review, checked on its own branch, with its page.
    repo.git(&["switch", "-q", "-c", "spec/20261030-anywhere"]);
    let capabilities = fs::read_to_string(repo.root.join("CAPABILITIES.md")).unwrap();
    repo.write(
        "CAPABILITIES.md",
        &capabilities.replace("A person can c.", "A person can c, anywhere."),
    );
    repo.write(
        "changes/20261030-anywhere/change.md",
        "---\nstatus: applied\n---\n\n## Intent\n\nc, anywhere.\n",
    );
    repo.commit("c, anywhere");
    clm(&["check"]);
    let gate = run(&["gate", "--base", "origin/main"]);
    let first = String::from_utf8_lossy(&gate.stdout).into_owned();
    assert_eq!(
        first.lines().next(),
        Some("spec review"),
        "{}",
        printed(&gate)
    );
    repo.git(&["push", "-q", "-u", "origin", "spec/20261030-anywhere"]);
    repo.git(&["switch", "-q", "main"]);
    let body = repo.root.parent().unwrap().join("body.md");
    fs::write(&body, "## Intent\n\nc, anywhere.\n").unwrap();
    let page = clm(&[
        "pr",
        "open",
        "spec/20261030-anywhere",
        "--title",
        "c, anywhere",
        "--body-file",
        body.to_str().unwrap(),
    ]);
    assert_eq!(page.trim(), "none");

    // Approve, then land.
    clm(&["approve", "spec/20261030-anywhere"]);
    let commit = landed(&run(&["land", "spec/20261030-anywhere"]));
    assert_eq!(repo.origin_main(), commit);
    let landed = repo.git(&["show", &format!("{commit}:CAPABILITIES.md")]);
    assert!(landed.contains("A person can c, anywhere."), "{landed}");

    // Nothing needed a host.
    assert!(
        !called.exists(),
        "gh was called: {}",
        fs::read_to_string(&called).unwrap()
    );
}

#[test]
fn azure_devops() {
    // req:capability/git-host@3a12ab25
    let repo = Repo::init();
    // The repository is set up for Azure DevOps.
    repo.write("clm.toml", "host = \"azure\"\n");
    repo.commit("host azure");
    repo.git(&["push", "-q", "origin", "main"]);

    // A fake `az` that records its calls: no active pull request, and a created one.
    let bin = repo.root.parent().unwrap().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let calls = bin.join("az-calls");
    let az = bin.join("az");
    let created =
        r#"{"pullRequestId": 12, "repository": {"webUrl": "https://dev.azure.com/o/p/_git/r"}}"#;
    fs::write(
        &az,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n*'pr list'*) echo '[]';;\n*'pr create'*) echo '{created}';;\n*) echo 'unexpected call' >&2; exit 1;;\nesac\n",
            calls.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&az, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    // Propose a change on a branch, then open its pull request.
    repo.git(&["switch", "-q", "-c", "spec/x"]);
    repo.write("note.md", "x\n");
    repo.commit("x");
    repo.git(&["push", "-q", "-u", "origin", "spec/x"]);
    let body = repo.root.parent().unwrap().join("body.md");
    fs::write(&body, "## Intent\n\nx.\n").unwrap();
    let out = repo.clm_env(
        &[
            "pr",
            "open",
            "spec/x",
            "--title",
            "x",
            "--body-file",
            body.to_str().unwrap(),
        ],
        "",
        &[("PATH", &path)],
    );
    assert!(out.status.success(), "{}", printed(&out));
    // Its URL is printed.
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "https://dev.azure.com/o/p/_git/r/pullrequest/12"
    );
    // The pull request was opened through the Azure CLI.
    let recorded = fs::read_to_string(&calls).unwrap();
    let create = recorded
        .lines()
        .find(|l| l.contains("repos pr create"))
        .unwrap_or_else(|| panic!("no pr create: {recorded}"));
    for want in [
        "--source-branch spec/x",
        "--target-branch main",
        "--title x",
        "--description @",
    ] {
        assert!(create.contains(want), "{create}");
    }
}
