//! Escalating a run: a `question` item for the human, filed without a subagent
//! (`repeated-stops-escalate`).

use std::ffi::OsStr;

use clm_git::{Failure, Options, git, run};

use crate::Options as LoopOptions;
use crate::cmd::{clm, clm_in};
use crate::state::{Kind, Run};

/// The item's slug, for a run that stopped for `cause`.
fn slug(run: &Run, cause: &str) -> String {
    let what = run
        .name
        .split_once(':')
        .map_or(run.name.as_str(), |(_, w)| w);
    format!("{what}-stops-on-{cause}")
}

/// The components a run's work is on: the component of a build, those of a draft's items.
fn components(options: &LoopOptions, run: &Run) -> Result<Vec<String>, Failure> {
    let name = |prefix: &str| run.name.strip_prefix(prefix).map(str::to_string);
    let mut out: Vec<String> = match run.kind {
        Some(Kind::Build) => name("build:").into_iter().collect(),
        Some(Kind::Draft) => {
            let mut all = Vec::new();
            for item in &run.items {
                let spec = format!("origin/main:backlog/{item}.md");
                let Ok(text) = git(&options.root, &["show", &spec]) else {
                    continue;
                };
                if let Ok(parsed) = clm_model::parse_backlog_item(item, &text) {
                    all.extend(parsed.frontmatter.nodes);
                }
            }
            all
        }
        _ => Vec::new(),
    };
    out.sort();
    out.dedup();
    Ok(out)
}

/// The pending requirements of a build's component, as `clm pending` lists them.
fn pending(options: &LoopOptions, component: &str) -> Result<Vec<String>, Failure> {
    let ran = clm(options, &["pending", component])?;
    if ran.code != 0 {
        return Ok(Vec::new());
    }
    Ok(ran
        .stdout
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect())
}

/// The slug and the text of the question item for a run that stopped for `cause`.
pub(crate) fn item(
    options: &LoopOptions,
    run: &Run,
    cause: &str,
) -> Result<(String, String), Failure> {
    let slug = slug(run, cause);
    let components = components(options, run)?;
    let blocks = match (run.kind, components.first()) {
        (Some(Kind::Build), Some(component)) => pending(options, component)?,
        _ => Vec::new(),
    };
    let last = run.log.last().map_or("", String::as_str);
    let mut text = format!(
        "---\ntitle: \"{} stops on {cause}: {}\"\nkind: question\ncomponents: [{}]\n",
        run.name,
        last.split_once(": ")
            .map_or(last, |(_, r)| r)
            .replace('"', "'"),
        components.join(", ")
    );
    if !blocks.is_empty() {
        text.push_str(&format!("blocks: [{}]\n", blocks.join(", ")));
    }
    text.push_str(&format!(
        "---\n\nThe build loop stopped launching `{}`: it stopped {} times, and the last two \
         stops, for the same cause, came after a rebuild. Its stops, oldest first:\n\n",
        run.name,
        run.log.len()
    ));
    for stop in &run.log {
        text.push_str(&format!("- {stop}\n"));
    }
    text.push_str(
        "\nWhat is needed: look at the run's branch and its last result, and say what to change, \
         so the work can go on. Delete this item once it is answered.\n",
    );
    Ok((slug, text))
}

/// Lands the item as `backlog/<slug>.md` on `origin/main`: commits a tree that adds it to
/// `origin/main`'s with `git commit-tree`, pushes the commit as `loop/<slug>`, lands that
/// branch with `clm land` from the main worktree and deletes the branch.
pub(crate) fn land(options: &LoopOptions, slug: &str, text: &str) -> Result<(), Failure> {
    let root = &options.root;
    let path = format!("backlog/{slug}.md");
    // Already on `main`: an earlier attempt landed and its answer was lost.
    if git(root, &["cat-file", "-e", &format!("origin/main:{path}")]).is_ok() {
        return Ok(());
    }
    let branch = format!("loop/{slug}");
    let index = std::env::temp_dir().join(format!("clm-loop-index-{}-{slug}", std::process::id()));
    let result = (|| {
        let index_env: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", index.as_os_str())];
        let in_index = Options {
            stdin: None,
            env: &index_env,
        };
        let blob = run(
            root,
            "git",
            &["hash-object", "-w", "--stdin"],
            Options {
                stdin: Some(text.as_bytes()),
                env: &[],
            },
        )?;
        let blob = String::from_utf8_lossy(&blob).trim().to_string();
        run(root, "git", &["read-tree", "origin/main"], in_index)?;
        let in_index = Options {
            stdin: None,
            env: &index_env,
        };
        let cacheinfo = format!("100644,{blob},{path}");
        run(
            root,
            "git",
            &["update-index", "--add", "--cacheinfo", &cacheinfo],
            in_index,
        )?;
        let in_index = Options {
            stdin: None,
            env: &index_env,
        };
        let tree = run(root, "git", &["write-tree"], in_index)?;
        let tree = String::from_utf8_lossy(&tree).trim().to_string();
        let message = format!("Escalate {slug}: the build loop asks the human\n");
        let commit = git(
            root,
            &["commit-tree", &tree, "-p", "origin/main", "-m", &message],
        )?;
        // A branch an earlier, failed attempt left is replaced.
        let _ = git(root, &["push", "origin", "--delete", &branch]);
        git(
            root,
            &["push", "origin", &format!("{commit}:refs/heads/{branch}")],
        )?;
        let main = clm_git::worktrees(root)?
            .into_iter()
            .next()
            .map_or_else(|| root.clone(), |w| w.path);
        let ran = clm_in(options, &main, &["land", &branch])?;
        if ran.code == 0 {
            Ok(())
        } else {
            Err(Failure::Refused(format!(
                "clm land {branch}: {}",
                ran.reason()
            )))
        }
    })();
    let _ = std::fs::remove_file(&index);
    // `clm land` deletes the branch it lands; a failed land leaves it, and the retry makes a
    // new one.
    let _ = git(root, &["push", "origin", "--delete", &branch]);
    let _ = git(
        root,
        &["update-ref", "-d", &format!("refs/remotes/origin/{branch}")],
    );
    result
}
