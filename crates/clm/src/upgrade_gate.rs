//! Which commands wait for the project's required migrations (`upgrade-blocks-commands`).

use std::path::{Path, PathBuf};

use crate::{Architecture, Branch, Capabilities, Command, Contribution, Effort, Ledger, Page};

/// Where a command works and whether a pending required migration stops it.
pub(crate) struct Located<'a> {
    /// The `--root` the command was given: `None` when it has none or was given none, and
    /// the repository holding the current directory is meant.
    pub(crate) root: Option<&'a Path>,
    /// Whether a pending required migration stops the command (`upgrade-blocks-commands`).
    pub(crate) blocked: bool,
}

/// Where `command` works, or `None` for the two commands that never look at the project:
/// `guide` and `hook`. Every variant is named, so a command added later doesn't compile until
/// someone decides which side of the list it is on.
pub(crate) fn located(command: &Command) -> Option<Located<'_>> {
    use Command as C;
    fn at(root: Option<&Path>, blocked: bool) -> Option<Located<'_>> {
        Some(Located { root, blocked })
    }
    fn given(root: &Option<PathBuf>) -> Option<Located<'_>> {
        at(root.as_deref(), true)
    }
    match command {
        C::Guide { .. } | C::Hook { .. } => None,
        // The commands that run whatever is pending.
        C::Status { root, .. } | C::Backlog { root, .. } | C::View { root, .. } => {
            at(root.as_deref(), false)
        }
        C::Defect { .. } | C::Config { .. } | C::Init { .. } | C::Upgrade { .. } => at(None, false),
        C::Check { root, .. }
        | C::Coverage { root }
        | C::Pending { root, .. }
        | C::Size { root, .. }
        | C::Apply { root, .. }
        | C::Rename { root, .. }
        | C::Archive { root, .. }
        | C::Context { root, .. }
        | C::Hashes { root, .. }
        | C::Gate { root, .. }
        | C::Next { root, .. }
        | C::Claim { root, .. }
        | C::Run { root, .. }
        | C::Setup { root }
        | C::Proposals { root, .. }
        | C::Triage { root, .. }
        | C::Intake { root, .. }
        | C::Draft { root, .. }
        | C::Runner { root, .. }
        | C::WrapUp { root }
        | C::Rebase { root }
        | C::Autosync { root }
        | C::Issue { root, .. }
        | C::Land { root, .. }
        | C::Verify { root, .. }
        | C::Approve { root, .. }
        | C::Reconnect { root }
        | C::Affected { root, .. }
        | C::Health { root, .. }
        | C::Workaround { root, .. }
        | C::Unreferenced { root, .. }
        | C::Survey { root, .. }
        | C::Fingerprint { root, .. } => given(root),
        C::Capabilities {
            command: Some(Capabilities::Decide { root: inner, .. }),
            root,
            ..
        }
        | C::Architecture {
            command: Some(Architecture::Decide { root: inner, .. }),
            root,
            ..
        } => at(inner.as_deref().or(root.as_deref()), true),
        C::Capabilities {
            command: None,
            root,
            ..
        }
        | C::Architecture {
            command: None,
            root,
            ..
        } => given(root),
        C::Pr {
            page: Page::Open { root, .. } | Page::Body { root, .. },
        } => given(root),
        C::Contribution { contribution } => match contribution {
            Contribution::Parent { root, .. }
            | Contribution::Sync { root, .. }
            | Contribution::Open { root, .. }
            | Contribution::Show { root, .. }
            | Contribution::Fetch { root, .. }
            | Contribution::Comment { root, .. }
            | Contribution::Label { root, .. } => given(root),
        },
        C::Branch {
            branch: Branch::Delete { root, .. } | Branch::Push { root, .. },
        } => given(root),
        C::Ledger { ledger } => match ledger {
            Ledger::Compact { root, .. }
            | Ledger::Verify { root, .. }
            | Ledger::Show { root, .. } => given(root),
        },
        C::Effort {
            effort: Effort::Record { root, .. },
        } => given(root),
        C::Superpowers { .. } => at(None, true),
    }
}

/// The error that stops `command`, or `None` when it may run. `require` is
/// `clm_upgrade::require_current`; `cwd` is where `clm` was started. Outside any CLM
/// repository, with no `--root`, `require` isn't called.
pub(crate) fn blocked(
    command: &Command,
    cwd: &Path,
    require: impl Fn(&Path) -> Result<(), String>,
) -> Option<String> {
    let here = located(command).filter(|here| here.blocked)?;
    let root = match here.root {
        Some(root) => root,
        None => clm_hooks::clm_root(cwd)?,
    };
    require(root).err()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;

    use clap::Parser;
    use tempfile::TempDir;

    use super::*;
    use crate::Cli;

    fn parse(args: &[&str]) -> Command {
        Cli::try_parse_from(std::iter::once("clm").chain(args.iter().copied()))
            .unwrap()
            .command
    }

    /// What `blocked` asked `require` about, and what it returned.
    fn run(args: &[&str], cwd: &Path) -> (Vec<PathBuf>, Option<String>) {
        let asked = RefCell::new(Vec::new());
        let result = blocked(&parse(args), cwd, |root| {
            asked.borrow_mut().push(root.to_path_buf());
            Err("pending".to_string())
        });
        (asked.into_inner(), result)
    }

    // req:clm/upgrade-blocks-commands@8787b1c6
    #[test]
    fn a_blocked_command_with_a_root_is_judged_for_that_root() {
        let cwd = TempDir::new().unwrap();
        for args in [
            &["check", "--root", "/r"][..],
            &["land", "--root", "/r"],
            &["ledger", "show", "--root", "/r"],
            &["capabilities", "--root", "/r"],
            &[
                "capabilities",
                "--root",
                "/r",
                "decide",
                "x",
                "--value",
                "high",
                "--decision",
                "keep",
            ],
            &[
                "architecture",
                "--root",
                "/r",
                "decide",
                "x",
                "--value",
                "high",
                "--decision",
                "keep",
            ],
            &["coverage", "--root", "/r"],
            &["pending", "--root", "/r"],
            &["rebase", "--root", "/r"],
        ] {
            let (asked, result) = run(args, cwd.path());
            assert_eq!(asked, [Path::new("/r")], "{args:?}");
            assert_eq!(result.as_deref(), Some("pending"), "{args:?}");
        }
    }

    // req:clm/upgrade-blocks-commands@8787b1c6
    #[test]
    fn the_listed_commands_run_whatever_is_pending() {
        let dir = TempDir::new().unwrap();
        let cwd = dir.path().canonicalize().unwrap();
        fs::create_dir_all(cwd.join("specs")).unwrap();
        fs::write(cwd.join("specs/SPEC.md"), "").unwrap();
        let cwd = &cwd;
        for args in [
            &["status", "--root", "/r"][..],
            &["guide"],
            &["defect", "--title", "t"],
            &["config", "get", "defects"],
            &["init"],
            &["backlog", "--root", "/r"],
            &["hook", "guard"],
            &["view", "spec", "--root", "/r"],
        ] {
            let (asked, result) = run(args, cwd);
            assert!(asked.is_empty() && result.is_none(), "{args:?}");
        }
    }

    // req:clm/upgrade-blocks-commands@8787b1c6
    #[test]
    fn without_a_root_the_repository_holding_the_current_directory_is_judged() {
        let dir = TempDir::new().unwrap();
        let top = dir.path().canonicalize().unwrap();
        fs::create_dir_all(top.join("specs")).unwrap();
        fs::write(top.join("specs/SPEC.md"), "").unwrap();
        fs::create_dir_all(top.join("sub/dir")).unwrap();
        let (asked, result) = run(&["check"], &top.join("sub/dir"));
        assert_eq!(asked, [top]);
        assert_eq!(result.as_deref(), Some("pending"));
    }

    // req:clm/upgrade-blocks-commands@8787b1c6
    #[test]
    fn outside_any_clm_repository_nothing_is_judged() {
        let dir = TempDir::new().unwrap();
        let top = dir.path().canonicalize().unwrap();
        fs::create_dir_all(top.join(".git")).unwrap();
        let (asked, result) = run(&["check"], &top);
        assert!(asked.is_empty() && result.is_none());
    }

    // req:clm/upgrade-blocks-commands@8787b1c6
    #[test]
    fn a_passing_judgement_lets_the_command_run() {
        let cwd = TempDir::new().unwrap();
        let result = blocked(&parse(&["check", "--root", "/r"]), cwd.path(), |_| Ok(()));
        assert_eq!(result, None);
    }
}
