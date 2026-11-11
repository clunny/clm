//! `clm defect`: reports a defect in CLM itself as the `defects` setting says
//! (`defect-reports`).

use std::fs;
use std::io::Write;
use std::path::Path;

use clm_config::{Defects, Host};

use crate::{clm_root, settings};

/// Files the report found from `cwd`: what to print on success, or why it failed.
pub fn defect(cwd: &Path, title: &str, nodes: &[String], report: &str) -> Result<String, String> {
    let settings = settings(Some(cwd)).map_err(|e| format!("settings: {e}"))?;
    match settings.defects.value {
        Defects::Backlog => to_backlog(cwd, title, nodes, report),
        Defects::Ask => to_issue(cwd, title, nodes, report),
        Defects::Off => Ok(format!(
            "CLM defect reporting is off (`defects = off`, from {}); nothing filed.",
            settings.defects.layer
        )),
    }
}

/// Writes `backlog/<slug>.md` in the CLM repository holding `cwd` and returns its path.
fn to_backlog(cwd: &Path, title: &str, nodes: &[String], report: &str) -> Result<String, String> {
    let root = clm_root(cwd).ok_or("no CLM repository (`specs/SPEC.md`) here; nothing filed")?;
    let (graph, _) = clm_model::load(root);
    // A node a live change creates may be named too (`model/backlog-nodes`).
    if let Some(node) = nodes.iter().find(|n| !graph.backlog_node_known(n)) {
        return Err(format!(
            "no component `{node}` in this repository's specs or live changes; nothing filed"
        ));
    }
    let mut item = format!(
        "---\ntitle: {}\nkind: bug\ncomponents: [{}]\n",
        serde_json::to_string(title).expect("a string serializes"),
        nodes.join(", ")
    );
    // A change the repository doesn't have live (not yet written, or archived away) isn't
    // recorded: `clm check` would reject it.
    let live = |c: &String| {
        graph.changes.get(c).is_some_and(|c| {
            !c.archived && c.frontmatter.status != clm_model::ChangeStatus::Abandoned
        })
    };
    if let Some(change) = origin(root).filter(live) {
        item.push_str(&format!("origin: {change}\n"));
    }
    item.push_str(&format!("---\n\n{}\n", report.trim_end_matches('\n')));
    let unfiled = |rel: &str, e: std::io::Error| {
        format!(
            "{rel}: {e}\n{}\nclm defect: the report above wasn't filed",
            report.trim_end_matches('\n')
        )
    };
    fs::create_dir_all(root.join("backlog")).map_err(|e| unfiled("backlog", e))?;
    match create_unique(&root.join("backlog"), &slug(title), &item) {
        Ok(name) => Ok(format!("backlog/{name}")),
        Err((name, e)) => Err(unfiled(&format!("backlog/{name}"), e)),
    }
}

/// Writes `content` to `<dir>/<base>.md`, or `<base>-2.md`, `<base>-3.md`... when that exists,
/// and returns the file name. `create_new`, so two reports with one title at once can't
/// overwrite each other. On failure, the name it tried and why.
fn create_unique(
    dir: &Path,
    base: &str,
    content: &str,
) -> Result<String, (String, std::io::Error)> {
    for n in 1.. {
        let name = match n {
            1 => format!("{base}.md"),
            n => format!("{base}-{n}.md"),
        };
        match fs::File::options()
            .write(true)
            .create_new(true)
            .open(dir.join(&name))
        {
            Ok(mut file) => {
                return file
                    .write_all(content.as_bytes())
                    .map(|()| name.clone())
                    .map_err(|e| (name, e));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err((name, e)),
        }
    }
    unreachable!("an unused name")
}

/// The longest slug, short of the `-<n>` a taken one gets, so the file name stays well
/// within what file systems allow.
const SLUG_MAX: usize = 64;

/// The title in kebab case, cut at a word to at most `SLUG_MAX` bytes, or `defect` when it
/// has no letters or digits.
fn slug(title: &str) -> String {
    let mut slug = String::new();
    let lower = title.to_lowercase();
    for word in lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let sep = usize::from(!slug.is_empty());
        if slug.len() + sep + word.len() > SLUG_MAX {
            if slug.is_empty() {
                slug.push_str(&word[..SLUG_MAX]);
            }
            break;
        }
        if sep == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    if slug.is_empty() {
        "defect".to_string()
    } else {
        slug
    }
}

/// The change of the current `spec/<change>` branch. No other branch names one: an
/// `impl/*` branch builds a node, not a change.
fn origin(root: &Path) -> Option<String> {
    let branch = clm_git::current_branch(root).ok()??;
    let change = branch.strip_prefix("spec/")?;
    (!change.is_empty() && !change.contains('/')).then(|| change.to_string())
}

/// Files an issue on `clunny/clm` through the `github` adapter, whatever the repository's
/// `host` (`defects-file-on-github`), run in `cwd`, and returns its URL. A report it can't file
/// is kept under `clm/defects/` in the git common directory of `cwd`'s repository.
fn to_issue(cwd: &Path, title: &str, nodes: &[String], report: &str) -> Result<String, String> {
    let mut body = format!("## What happened\n\n{}\n\n", report.trim_end_matches('\n'));
    if !nodes.is_empty() {
        body.push_str(&format!("- Components: {}\n", nodes.join(", ")));
    }
    body.push_str(&format!(
        "- clm: {}\n- OS: {} {}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    let report = report.trim_end_matches('\n');
    let why = match file_issue(cwd, title, &body) {
        Ok(url) => return Ok(url),
        Err(why) => why,
    };
    let kept = match clm_git::common_dir(cwd) {
        Err(_) => "it isn't in a git repository, so the report wasn't kept".to_string(),
        Ok(common) => {
            let dir = common.join("clm").join("defects");
            let content = format!("# {title}\n\n{body}");
            match fs::create_dir_all(&dir)
                .map_err(|e| (dir.display().to_string(), e))
                .and_then(|()| create_unique(&dir, &slug(title), &content))
            {
                Ok(name) => format!(
                    "kept in {}; file it by hand on clunny/clm",
                    dir.join(name).display()
                ),
                Err((name, e)) => format!("the report wasn't kept: {name}: {e}"),
            }
        }
    };
    Err(format!(
        "{report}\nclm defect: the report above wasn't filed: {why}\nclm defect: {kept}"
    ))
}

/// Opens the issue through the `github` adapter and returns its URL, or why it couldn't.
fn file_issue(cwd: &Path, title: &str, body: &str) -> Result<String, String> {
    let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    file.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    match clm_host::adapter(Host::Github).issue(cwd, title, file.path(), "clunny/clm") {
        Ok(url) if url.trim().is_empty() => Err("the host printed no issue URL".into()),
        Ok(url) => Ok(url),
        Err(
            clm_host::Error::Nowhere(why)
            | clm_host::Error::Refused(why)
            | clm_host::Error::Git(why),
        ) => Err(why),
    }
}
