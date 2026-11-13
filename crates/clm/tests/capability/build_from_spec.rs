//! `build-from-spec`: every pending requirement claimed, built in parallel isolated sessions,
//! and landed.

use std::fs;
use std::path::Path;

use crate::{Repo, landed, printed};

/// Node `b`, owning `lib/**`, with requirement `z`.
const NODE_B: &str = "---\ntitle: B\nstatus: active\nowners: []\npaths: [\"lib/**\"]\nserves: [capability/c]\n---\n\n## Contract\nB does z.\n\n## Requirements\n\n### Requirement: z\nB SHALL z.\n\n#### Scenario: z\n- GIVEN g\n- WHEN w\n- THEN t\n";

/// The nodes `clm next` lists as ready, in its order.
fn ready(repo: &Repo) -> Vec<String> {
    let out = repo.clm(&["next"]);
    assert!(out.status.success(), "{}", printed(&out));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

/// What a build session does in its own worktree `dir`: switch to the node's branch, write
/// a test citing each of `requirements` (`(slug, version)`) under `tests_dir`, push and land.
fn build(repo: &Repo, dir: &Path, node: &str, tests_dir: &str, requirements: &[(&str, String)]) {
    let d = dir.to_str().unwrap();
    repo.git(&["worktree", "add", "-q", "--detach", d, "origin/main"]);
    let git = |args: &[&str]| {
        let mut all = vec!["-C", d];
        all.extend_from_slice(args);
        repo.git(&all)
    };
    git(&["switch", "-q", &format!("impl/{node}")]);
    for (slug, version) in requirements {
        let marker = format!("{}{node}/{slug}@{version}", concat!("req", ":"));
        let file = dir.join(tests_dir).join(format!("t_{slug}.sh"));
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, format!("# {marker}\ntrue\n")).unwrap();
    }
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", &format!("build {node}")]);
    git(&["push", "-q", "origin", &format!("impl/{node}")]);
    let session = repo.at(dir);
    landed(&session.clm(&["land", &format!("impl/{node}")]));
}

/// Runs `clm <args>` as a session under Claude Code would, and returns the actions it printed.
fn loop_command(repo: &Repo, args: &[&str]) -> Vec<String> {
    let out = repo.clm_env(args, "", &[("CLAUDECODE", "1")]);
    assert!(out.status.success(), "{}", printed(&out));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// A node's requirement slugs with the versions a test cites them at.
type Versions = Vec<(&'static str, String)>;

/// A repository with nodes `a` (requirements `x`, `y`) and `b` (`z`) pending and nothing else,
/// and the versions each build cites.
fn queue() -> (Repo, Versions, Versions) {
    let repo = Repo::init();
    repo.write("specs/b/SPEC.md", NODE_B);
    // The fixture's capability is already proven, so only `a` and `b` are pending.
    let capability = &repo.hash("capability", "capability:c")[..8];
    repo.write(
        "tests/c.sh",
        &format!("# {}{capability}\ntrue\n", concat!("req", ":capability/c@")),
    );
    repo.commit("node b, and c proven");
    repo.land_on_origin("true");
    let versions = |node: &str, slugs: &[&'static str]| -> Vec<(&'static str, String)> {
        slugs
            .iter()
            .map(|s| {
                let version = &repo.hash(node, &format!("requirement:{s}"))[..8];
                (*s, version.to_string())
            })
            .collect()
    };
    let a = versions("a", &["x", "y"]);
    let b = versions("b", &["z"]);
    (repo, a, b)
}

/// The prompt file a `launch build:<node> ...` line names, read.
fn launch_prompt(actions: &[String], node: &str) -> Option<String> {
    let line = actions
        .iter()
        .find(|l| l.starts_with(&format!("launch build:{node} build ")))?;
    Some(fs::read_to_string(line.rsplit(' ').next().unwrap()).unwrap())
}

#[test]
fn drain_the_queue() {
    // req:capability/build-from-spec@54a7f627
    let (repo, a, b) = queue();

    // `/clm:build`: both nodes are ready, with no `Needs` between them.
    let mut nodes = ready(&repo);
    nodes.sort();
    assert_eq!(nodes, ["a", "b"]);

    // The loop launches both builds before either lands, each holding a place of its own, and
    // the session reports each subagent's task.
    let started = loop_command(&repo, &["build", "start", "--session-model", "opus"]);
    for node in ["a", "b"] {
        let launch = started
            .iter()
            .find(|l| l.starts_with(&format!("launch build:{node} build ")))
            .unwrap_or_else(|| panic!("no launch for {node}: {started:?}"));
        let prompt = fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
        assert!(
            prompt.starts_with(&format!("loop run build:{node}\n")),
            "{prompt}"
        );
        assert!(repo.root.join(format!(".clm/prompts/{node}.md")).exists());
        loop_command(
            &repo,
            &["build", "launched", &format!("build:{node}"), "t-1"],
        );
    }
    repo.git(&["fetch", "-q", "origin"]);
    assert!(ready(&repo).is_empty(), "both are claimed");

    // Each build session works in its own worktree, and lands; the session reports its return.
    let worktrees = repo.root.join(".claude/worktrees");
    build(&repo, &worktrees.join("build-a"), "a", "tests", &a);
    build(&repo, &worktrees.join("build-b"), "b", "lib/tests", &b);
    for node in ["a", "b"] {
        loop_command(
            &repo,
            &[
                "build",
                "returned",
                &format!("build:{node}"),
                "--result",
                "landed 1a2b3c4",
            ],
        );
    }

    // Nothing is left to build.
    repo.git(&["pull", "-q", "--ff-only", "origin", "main"]);
    let out = repo.clm(&["pending"]);
    let text = printed(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "", "{text}");
    assert!(ready(&repo).is_empty());
}

#[test]
fn resume_in_a_new_session() {
    // req:capability/build-from-spec@54a7f627
    let (repo, a, b) = queue();
    let worktrees = repo.root.join(".claude/worktrees");

    // The first session launches both builds; `b` lands, and the session ends while `a` runs.
    loop_command(&repo, &["build", "start", "--session-model", "opus"]);
    for node in ["a", "b"] {
        loop_command(
            &repo,
            &["build", "launched", &format!("build:{node}"), "t-1"],
        );
    }
    build(&repo, &worktrees.join("build-b"), "b", "lib/tests", &b);
    loop_command(
        &repo,
        &["build", "returned", "build:b", "--result", "landed 1a2b3c4"],
    );
    repo.git(&["fetch", "-q", "origin"]);
    let landed_before = repo.git(&["rev-parse", "origin/main"]);

    // A session whose lease is live is refused; once the lease lapses, a new one adopts the loop.
    let live = repo.clm_env(
        &["build", "start", "--session-model", "opus"],
        "",
        &[("CLAUDECODE", "1")],
    );
    assert!(!live.status.success(), "{}", printed(&live));
    assert!(
        printed(&live).contains("clm view queue"),
        "{}",
        printed(&live)
    );
    let common = repo.git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]);
    let file = Path::new(common.trim()).join("clm/loop");
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    state["lease"] = serde_json::json!(state["lease"].as_u64().unwrap() - 20 * 60);
    fs::write(&file, serde_json::to_vec(&state).unwrap()).unwrap();

    let started = loop_command(&repo, &["build", "start", "--session-model", "opus"]);
    let prompt = launch_prompt(&started, "a")
        .unwrap_or_else(|| panic!("`a` is not relaunched: {started:?}"));
    assert!(prompt.contains("impl/a"), "{prompt}");
    assert!(
        launch_prompt(&started, "b").is_none(),
        "`b` had landed: {started:?}"
    );
    loop_command(&repo, &["build", "launched", "build:a", "t-2"]);

    // The relaunched build lands, with nothing redone: `main` gained only `a`'s land.
    build(&repo, &worktrees.join("build-a"), "a", "tests", &a);
    loop_command(
        &repo,
        &["build", "returned", "build:a", "--result", "landed 5d6e7f8"],
    );
    repo.git(&["pull", "-q", "--ff-only", "origin", "main"]);
    let log = repo.git(&[
        "log",
        "--first-parent",
        "--format=%s",
        &format!("{}..HEAD", landed_before.trim()),
    ]);
    assert_eq!(log.lines().count(), 1, "{log}");
    let out = repo.clm(&["pending"]);
    assert!(out.status.success(), "{}", printed(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "");
}
