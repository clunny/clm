//! `clm build release <component>`: the human gives up a component's build (`release`).

use clm_git::{Failure, now};

use crate::cmd::{clm, clm_ok};
use crate::state::{self, RunState};
use crate::step::{next_actions, refresh_holds};
use crate::{Options, time};

/// Releases `component`'s build: the branch on `origin` goes, the local one is kept under
/// `released/<component>/<time>`, the place is freed and the run forgotten. Prints the `kept`
/// line, then the actions to take now.
pub fn release(options: &Options, component: &str) -> Result<Vec<String>, Failure> {
    let root = &options.root;
    let name = format!("build:{component}");
    clm_git::fetch(root)?;
    refresh_holds(options)?;
    if let Some(run) = state::read(root).and_then(|f| f.run(&name).cloned()) {
        let live = !run.orphan && matches!(run.state, RunState::Planned | RunState::Running);
        if live || (run.orphan && run.hold.is_some()) {
            return Err(Failure::Refused(format!(
                "{name} is still running or held by a session taken over; \
                 stop it first, or wait for its branch to stay still"
            )));
        }
    }
    let branch = format!("impl/{component}");
    let deleted = clm(options, &["branch", "delete", &branch])?;
    if deleted.code != 0 {
        let reason = deleted.reason();
        return Err(if deleted.code == 1 {
            Failure::Refused(reason)
        } else {
            Failure::Git(reason)
        });
    }
    let mut lines = Vec::new();
    let local = format!("refs/heads/{branch}");
    if clm_git::git(root, &["rev-parse", "--verify", "-q", &local]).is_ok() {
        let kept = format!("released/{component}/{}", time::compact(now()));
        clm_git::git(root, &["branch", "-m", &branch, &kept])?;
        lines.push(format!("kept {kept}"));
    }
    clm_ok(options, &["run", "--finish", component])?;
    state::update(root, |f| {
        f.lease = Some(now());
        f.runs.retain(|r| r.name != name);
        Ok(())
    })?;
    lines.extend(next_actions(options)?);
    Ok(lines)
}
