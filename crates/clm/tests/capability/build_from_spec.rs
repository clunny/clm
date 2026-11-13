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
fn build(
    repo: &Repo,
    dir: &Path,
    node: &str,
    tests_dir: &str,
    requirements: &[(&str, String)],
) -> String {
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
    landed(&session.clm(&["land", &format!("impl/{node}")]))
}

#[test]
fn drain_the_queue() {
    // req:capability/build-from-spec@4bf37ced
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

    // `/clm:build`: both nodes are ready, with no `Needs` between them.
    let mut nodes = ready(&repo);
    nodes.sort();
    assert_eq!(nodes, ["a", "b"]);

    // The loop starts, and prints a launch for each build before either lands, each holding
    // a place of its own.
    let build_loop = |args: &[&str]| {
        let out = repo.clm_env(&[&["build"], args].concat(), "", &[("CLAUDECODE", "1")]);
        assert!(out.status.success(), "{}", printed(&out));
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let actions = build_loop(&["start", "--session-model", "opus"]);
    let launches: Vec<&String> = actions
        .iter()
        .filter(|l| l.starts_with("launch "))
        .collect();
    assert_eq!(launches.len(), 2, "{actions:?}");
    for node in ["a", "b"] {
        let prefix = format!("launch build:{node} build sonnet ");
        let launch = launches.iter().find(|l| l.starts_with(&prefix)).unwrap();
        let prompt = fs::read_to_string(launch.strip_prefix(&prefix).unwrap()).unwrap();
        assert!(
            prompt.starts_with(&format!("loop run build:{node}\n")),
            "{prompt}"
        );
        assert!(repo.root.join(format!(".clm/prompts/{node}.md")).exists());
    }
    repo.git(&["fetch", "-q", "origin"]);
    assert!(ready(&repo).is_empty(), "both are claimed");

    // The session launches each build in a subagent and reports its task.
    for (i, node) in ["a", "b"].iter().enumerate() {
        build_loop(&["launched", &format!("build:{node}"), &format!("t-{i}")]);
    }

    // Each build session works in its own worktree, and lands.
    let worktrees = repo.root.join(".claude/worktrees");
    let sha_a = build(&repo, &worktrees.join("build-a"), "a", "tests", &a);
    let sha_b = build(&repo, &worktrees.join("build-b"), "b", "lib/tests", &b);
    for (node, sha) in [("a", sha_a), ("b", sha_b)] {
        let actions = build_loop(&[
            "returned",
            &format!("build:{node}"),
            "--result",
            &format!("landed {sha}"),
        ]);
        assert!(
            !actions.iter().any(|l| l.starts_with("launch ")),
            "{actions:?}"
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
