//! The remote a `host = "none"` clone keeps for itself, given before a command runs
//! (`none-remote-before-commands`).

use std::path::{Path, PathBuf};

use clm_config::Host;
use clm_git::{REMOTE, common_dir, git};

use crate::Command;
use crate::upgrade_gate::located;

/// Gives the clone `command` runs in its own remote when its `host` is `none` and it has
/// none, and returns the lines to print on stderr: what was done, or a warning. Nothing is
/// pushed. A command outside a CLM repository, or whose settings don't load, gets none of
/// this, and neither do `guide` and `hook`.
pub(crate) fn keep(command: &Command, cwd: &Path) -> Vec<String> {
    let Some(here) = located(command) else {
        return Vec::new();
    };
    let root = match here.root {
        Some(root) if root.join("specs/SPEC.md").is_file() => root,
        Some(_) => return Vec::new(),
        None => match clm_hooks::clm_root(cwd) {
            Some(root) => root,
            None => return Vec::new(),
        },
    };
    let Ok(settings) = clm_config::load(Some(root), &clm_config::process_env) else {
        return Vec::new();
    };
    if settings.host.value != Host::None {
        return Vec::new();
    }
    match clm_host::keep_remote(root, Host::None) {
        Ok(Some(path)) => notes(Ok(Some(path)), local_main(root), true),
        Ok(None) => notes(Ok(None), false, origin_is_own(root)),
        Err(
            clm_host::Error::Refused(why)
            | clm_host::Error::Nowhere(why)
            | clm_host::Error::Git(why),
        ) => notes(Err(why), false, true),
    }
}

/// The stderr lines for what `keep_remote` did: `made` is its answer, `has_main` whether the
/// clone has a local `main`, `own` whether `origin` is the clone's own remote.
fn notes(made: Result<Option<PathBuf>, String>, has_main: bool, own: bool) -> Vec<String> {
    match made {
        Ok(Some(path)) => {
            let mut lines = vec![format!(
                "origin: {} (the clone's own remote, host = \"none\")",
                path.display()
            )];
            if has_main {
                lines.push(
                    "main is only local: run `clm reconnect` to push main and the ledgers to it"
                        .to_string(),
                );
            }
            lines
        }
        Ok(None) if own => Vec::new(),
        Ok(None) => vec![
            "warning: host = \"none\" but origin is another repository: lands will push main \
             there; a repository with a remote should set host = \"git\" in clm.toml"
                .to_string(),
        ],
        Err(why) => vec![format!(
            "warning: couldn't give the clone its own remote: {why}"
        )],
    }
}

fn local_main(root: &Path) -> bool {
    git(
        root,
        &["rev-parse", "--verify", "--quiet", "refs/heads/main"],
    )
    .is_ok()
}

/// Whether `origin` is the bare repository `keep_remote` makes. An `origin` whose URL can't
/// be read counts as the clone's own, since there is nothing to name.
fn origin_is_own(root: &Path) -> bool {
    let (Ok(url), Ok(common)) = (git(root, &["remote", "get-url", REMOTE]), common_dir(root))
    else {
        return true;
    };
    let own = common.join("clm").join("remote.git");
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    same(Path::new(&url), &own)
}

#[cfg(test)]
mod tests {
    use super::*;

    // req:clm/none-remote-before-commands@2ab520dd
    #[test]
    fn a_made_remote_is_named_with_the_reconnect_hint_when_there_is_a_main() {
        let made = Ok(Some(PathBuf::from("/r/.git/clm/remote.git")));
        let lines = notes(made.clone(), true, true);
        assert_eq!(
            lines[0],
            "origin: /r/.git/clm/remote.git (the clone's own remote, host = \"none\")"
        );
        assert!(lines[1].contains("clm reconnect"), "{lines:?}");
        assert_eq!(notes(made, false, true).len(), 1);
    }

    // req:clm/none-remote-before-commands@2ab520dd
    #[test]
    fn another_origin_warns_and_the_clones_own_is_quiet() {
        assert!(notes(Ok(None), false, true).is_empty());
        let lines = notes(Ok(None), false, false);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("host = \"git\""), "{lines:?}");
    }

    // req:clm/none-remote-before-commands@2ab520dd
    #[test]
    fn a_failure_is_a_warning() {
        let lines = notes(Err("it broke".into()), false, true);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("warning:") && lines[0].contains("it broke"));
    }
}
