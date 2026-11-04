//! `clm runner`: unattended headless builds on a machine the human owns
//! (specs/orchestrator/runner/SPEC.md).

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod guard;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod looks;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod session;

/// What `clm runner` was given.
pub struct Options {
    /// The program run as `clm`: `crates/clm` passes its own executable.
    pub clm: PathBuf,
    /// The program run as `claude`, resolved on `PATH`.
    pub claude: OsString,
    /// The CLM repository the runner builds.
    pub root: PathBuf,
    /// `--permission-mode`, passed to `claude` unchecked.
    pub permission_mode: Option<String>,
    /// How long to wait between looks for work when nothing finishes.
    pub interval: Duration,
    /// Exit once a look starts nothing and no session is running.
    pub once: bool,
    /// Hours after which a claim lapses, passed to `clm next` and `clm run`.
    pub expiry: u64,
}

/// The paragraph every session's prompt starts with, before the launcher `clm run` printed.
pub const UNATTENDED: &str = "Nobody watches this session: `clm runner` started it headless, \
so Claude Code denies, rather than asks about, any tool call that needs confirmation, such as \
a command the settings don't allow, a write to a protected path such as \
`.claude/settings.json` or `.npmrc`, or the guard's `ask` decisions. This overrides the prompt \
file's note that Claude Code may ask the human to confirm a call. Treat such a denial as a \
step only the human can take: never write the file or run the command another way. Leave the \
requirement that needs it unbuilt, add a backlog item of kind `human-task` whose `blocks` names \
that requirement, build the rest, and land the branch with `clm land` even when the item is \
all it holds, so the item reaches `main` and the component settles. When `clm defect` is denied, \
list the defect under `## CLM defects` in the pull request body instead.";

/// Runs the runner until it stops, returning its exit code.
pub fn runner(options: &Options) -> u8 {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        looks::run(options)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = options;
        eprintln!("clm runner: runs on Linux and macOS");
        1
    }
}
