//! Run (see specs/orchestrator/run/SPEC.md): `clm run <node>` claims a node, or picks up
//! its unfinished build, and writes the prompt for a session that builds its pending
//! requirements and lands them with `clm land` to `.clm/prompts/<node>.md`, printing a short
//! launcher that names it (`builds-in-subagents`, `build-prompt-file`,
//! `unattended-superpowers`, `land-locally`). It starts no session and makes no worktree:
//! `build` gives the launcher to a subagent in its own Claude Code worktree.
//! `clm superpowers <skill>` resolves the pinned copy builds follow.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use clm_coverage::{Marker, Pending, State as ReqState};
use clm_gate::{ListLine, list_lines};
use clm_git::{REMOTE, git, git_isolated};
use clm_model::{Files, Graph};
pub use clm_orchestrator::Error;
use clm_orchestrator::{PlaceKind, Placed, claim_local, free_place, place_slot, take_place};

mod command;
pub use command::{
    RunArgs, SetupArgs, SuperpowersArgs, run_command, setup_command, superpowers_command,
};

/// The environment a call reads, as a lookup of one variable's value.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The branch a node's claim and build live on is `impl/<node>`.
const BRANCH_PREFIX: &str = "impl/";

/// The superpowers repository builds follow.
const SUPERPOWERS_URL: &str = "https://github.com/obra/superpowers";

/// The pin, one line `<tag> <commit>` in a data file, moved by hand without editing Rust
/// (`cut-evals`).
const PIN: (&str, &str) = split_pin(include_str!("../superpowers.pin"));
/// The pinned release's tag.
pub const SUPERPOWERS_TAG: &str = PIN.0;
/// The pinned release's commit.
pub const SUPERPOWERS_COMMIT: &str = PIN.1;

/// `<tag> <commit>`, split at compile time. Trailing whitespace, such as the `\r\n` of a
/// checkout that converts line endings, is dropped.
const fn split_pin(pin: &'static str) -> (&'static str, &'static str) {
    let bytes = pin.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut space = 0;
    while space < end && bytes[space] != b' ' {
        space += 1;
    }
    assert!(
        space > 0 && space + 1 < end,
        "superpowers.pin is `<tag> <commit>`"
    );
    let (tag, rest) = bytes.split_at(space);
    let (commit, _) = rest.split_at(end - space);
    let (_, commit) = commit.split_at(1);
    match (std::str::from_utf8(tag), std::str::from_utf8(commit)) {
        (Ok(tag), Ok(commit)) => (tag, commit),
        _ => panic!("superpowers.pin isn't UTF-8"),
    }
}

/// The method a session follows (`design-brainstorming`). A constant like the pin, changed
/// as code.
pub const SKILLS: &[&str] = &[
    "superpowers:writing-plans",
    "superpowers:test-driven-development",
    "superpowers:verification-before-completion",
];

/// Skills that would hand the step plan to other sessions, worktrees or a human.
const EXECUTION_SKILLS: &[&str] = &[
    "superpowers:executing-plans",
    "superpowers:subagent-driven-development",
    "superpowers:using-git-worktrees",
    "superpowers:finishing-a-development-branch",
];

/// Why a build never waits on a background command by matching processes by name.
const BACKGROUND_WAIT: &str = ", never through a loop, watch or monitor that matches \
                               processes by name (such as `pgrep -f`): such a match also finds \
                               other builds' processes and outlives the command it waits for.";

/// A land the session started is waited for to its end (`one-report-per-run`).
const LAND_WAIT: &str = " Wait for a `clm land` you started{check_land} until its completion \
                          notice arrives, however long that takes, since lands queue behind \
                          other builds' lands and can take many minutes. Never write your final \
                          message, landed or stopped, while one of those is still running, and \
                          never stop one to end the build, since the orchestrating session reads \
                          the final line as the build's end and then frees your place and tidies \
                          your worktree. When your turn ends while a command you started in the \
                          background still runs, because waiting for its completion notice is all \
                          that is left, the message ending that turn is not your final message, \
                          since the orchestrating session may receive it as the build's result: \
                          it carries no closing line, neither `landed <sha>` nor `stopped: \
                          <reason>`, and ends with the line `waiting: <command>`, naming the \
                          command you wait for; your final message comes in a later turn, once \
                          the command has ended. You hand back one report, your final message \
                          (`one-report-per-run`): a tool that delivers your report to the \
                          orchestrating session and accepts one report per run, such as Claude \
                          Code's `SubagentHandback`, is never called for the message ending a \
                          turn that waits, which you end as plain text, and is called once, with \
                          the final message, since a report sent while waiting uses up the run's \
                          one report and the final message would then never reach the \
                          orchestrating session.";

/// The review (`advisory-code-review`): `/code-review` at this level, plus this subagent.
const REVIEW_LEVEL: &str = "medium";
const SPEC_AUDITOR: &str = "clm:spec-auditor";

/// How many requirements a run may stop after, with the tree green, when the pending list
/// is long (`node-reconciliation`).
pub const BUDGET: usize = 6;

const WORKDIR: &str = ".clm";
/// Where the session writes its pull request body (`own-pr-body`).
const PR_BODY: &str = ".clm/pr-body.md";
/// Where `clm run` writes each node's prompt, under the checkout it runs in.
const PROMPTS: &str = ".clm/prompts";

/// What a run tells its session about one pending requirement (`pending-prompt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// `<node>/<slug>`.
    pub id: String,
    /// Its current version; `None` when obsolete.
    pub version: Option<String>,
    /// `unbuilt`, `behind` or `obsolete`.
    pub state: String,
    /// Behind: for each distinct cited version, the diff from the text at that version to
    /// the current text, or `None` when the earlier text wasn't found.
    pub diffs: Vec<(String, Option<String>)>,
    /// Obsolete: the markers' locations. Behind: the locations of the markers that cite it
    /// at another version or without one (`stale-markers-keep-behind`), except those of
    /// `foreign` and those in a test of `delete`.
    pub markers: Vec<String>,
    /// Behind, once the root sets `wiring`: the stale markers in a file another component
    /// owns, each with that owner, which this build may not change (`pending-prompt`). An
    /// obsolete requirement has none: `pending` gives such a marker to its file's owner
    /// (`check.coverage/pending-report`).
    pub foreign: Vec<(String, String)>,
    /// Obsolete: the tests among `markers` that an approved `deleted-tests.txt` names
    /// (`land/deleted-tests-approved`), each `<file>::<name>`, which the build deletes.
    /// Behind: each test in a file the build may change that cites it at any version and
    /// that an approved list names, whose markers `markers` leaves out.
    pub delete: Vec<String>,
    /// Obsolete: every other test among `markers`, which `clm land` refuses the build to
    /// lose, each `<file>::<name>`; empty when `elsewhere` is set.
    pub keep: Vec<String>,
    /// Obsolete, when its ID names another component or one the spec graph doesn't hold:
    /// what this build does with its markers in its own files instead of `keep`.
    pub elsewhere: Option<Elsewhere>,
}

/// An obsolete requirement that was not the building component's, pending on it for the
/// markers in its files (`pending-prompt`, `check.coverage/pending-report`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Elsewhere {
    /// The changes that removed it, sorted.
    pub removed_by: Vec<String>,
    /// Each test, `<file>::<name>`, that no approved list names and whose requirement a
    /// move takes to a requirement of this component, with that requirement.
    pub recite: Vec<(String, String)>,
    /// Each marker, `<file>:<line>`, outside a test or on a test that also cites a
    /// requirement the spec holds.
    pub take_out: Vec<String>,
    /// Every other test, `<file>::<name>`, to leave.
    pub leave: Vec<String>,
    /// The component whose code it needed, when the spec graph holds that component, no
    /// file of it cites the requirement and it isn't a capability: that component's to
    /// remove, under a `deferred` item.
    pub owner_code: Option<String>,
}

/// A blocked requirement and what blocks it (`pending-prompt`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Blocked {
    /// `<node>/<slug>`.
    pub id: String,
    /// The `backlog/<slug>` items whose `blocks` name it, then the pending requirements it
    /// waits on.
    pub by: Vec<String>,
    /// For an obsolete requirement that waits on its own ID, the components its other lines
    /// are pending on, sorted: those whose files still cite it (`check.coverage/wait-lines`).
    pub cited_in: Vec<String>,
}

/// The job a run gives its session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Job {
    pub node: String,
    /// The node's `SPEC.md`, repo-relative.
    pub spec: String,
    /// Each skill of [`SKILLS`] with the path of its `SKILL.md` in the pinned copy, as
    /// `clm superpowers` prints it.
    pub skills: Vec<(String, String)>,
    /// The unblocked pending requirements, most urgent first.
    pub pending: Vec<Item>,
    /// The blocked requirements, each with what blocks it.
    pub blocked: Vec<Blocked>,
    /// The node's `clm context` bundle.
    pub bundle: Option<String>,
    /// Whether the run resumes `impl/<node>`, which then has commits `origin/main` lacks.
    pub resumed: bool,
    /// The repository's `[land] check`, the command `clm land` runs (`check-before-land`).
    pub check: Option<String>,
    /// The alias of the model the `spec-auditor` review runs on, passed in unchecked
    /// (`model-per-launch`); `None` for the session's own model.
    pub review_model: Option<String>,
    /// Whether the node builds the capability tests, as the model's
    /// `capability_tests_node` says (`prompt-serves`).
    pub capability_tests: bool,
    /// Whether the lists were read from `origin/main`'s tree because the branch conflicts
    /// with it (`pending-prompt`).
    pub read_from_main: bool,
    /// The build's slot, `place_slot` of `impl/<node>` once its place is taken
    /// (`prompt-slot`).
    pub slot: u64,
    /// The repository's `[build] setup`, the command `clm setup` runs (`prompt-setup`).
    pub setup: Option<String>,
    /// Whether the root sets `wiring`, so a build changes only its own component's files
    /// and its stale markers in another's are listed apart (`pending-prompt`).
    pub wiring: bool,
    /// Whether the repository's `[verify]` sets `mutate`, so the mutation pass after the land
    /// judges the tests and the prompt leaves out the hand-run spot-checks
    /// (`prompt-spot-checks`).
    pub mutate: bool,
}

/// The prompt for `job`.
pub fn prompt(job: &Job) -> String {
    let node = &job.node;
    let slot = job.slot;
    // Only a repository with a `[land] check` has `clm land --check-only` to name.
    let check_slot = if job.check.is_some() {
        ", while `clm land --check-only` sets `CLM_SLOT` itself and is still run as the checks \
         step says, with no variable"
    } else {
        ""
    };
    let branch = format!("{BRANCH_PREFIX}{node}");
    let skills = {
        let each: Vec<String> = job
            .skills
            .iter()
            .map(|(skill, path)| format!("`{skill}`, at `{path}`"))
            .collect();
        listing_owned(&each, "and", false)
    };
    let sync = "run `clm rebase` and resolve what it reports (conflicted files, stale deltas, \
                check failures), committing each fix, until its last line is `rebase ok`. Go by \
                that last line: `rebase incomplete` means something is left for you, `rebase \
                failed` means git itself failed, so run it once more and, if it fails again, \
                stop and say why";
    let spec_changed = format!(
        "When `clm rebase` reports `spec changed {node}`, the spec moved under you: read the \
         `changed` and `requirement` lines that follow, run `clm pending {node}`, and either \
         adapt your work to the new text or discard and rebuild the part it affects, \
         whichever fits. Record what changed and which way you went under `## Sync` in the \
         pull request body. Nothing is re-planned: the pending list comes from the markers each time. \
         A change `clm rebase` archived on the branch is committed with the rest."
    );
    // Where the session passes the checks, it runs `clm land --check-only`, never the
    // configured command by its path (`check-before-land`, `land-check-command`).
    // The background rule names the check command, and says running it in the background
    // keeps it one plain command, only when the repository sets one
    // (`prompt-check-command`).
    let (rerun, again, background) = match &job.check {
        Some(check) => (
            format!(
                "re-run the checks: run `clm land --check-only`, which runs `{check}`, the \
                 command `clm.toml` names, and then `clm check`, as `clm land` does. Run it \
                 from the worktree's top level as one plain command, exactly as written, \
                 adding no `cd`, absolute path, variable, pipe or redirection. A pipe, such as \
                 one to `tail`, reports the last command's result instead of the check's and \
                 cuts the check's output, so a failing check can read as passing; the \
                 command's last line, `checks pass` or the name of the command that failed, \
                 is its verdict, and Claude Code keeps a long command's output whole, so \
                 there is nothing to gain by cutting it. When Claude \
                 Code refuses or denies that command, don't run a part of it in its place and \
                 call that the check: report the refusal with `clm defect`, list it under \
                 `## CLM defects` in the pull request body, and land with `clm land`, which runs the \
                 whole command on the merged tree and refuses the land when it fails. You may \
                 run parts of the check while you work, to fail faster, but never report them \
                 as the check. While `clm land --check-only` runs, in the foreground or the \
                 background, change nothing in your worktree: no commit, no edit to a tracked \
                 file, no `clm rebase` and no `git` command that moves `HEAD`. The command \
                 judges the working tree as it stands, so a change made meanwhile can fail a \
                 check that reads `HEAD` and leave a verdict on a tree you no longer have. \
                 Work outside git, such as writing `.clm/pr-body.md`, may go on meanwhile. \
                 After a change you made once the check started, run `clm land --check-only` \
                 again once the check has ended."
            ),
            format!(
                "pass the repo's checks again with `clm land --check-only`, which runs \
                 `{check}` and then `clm check`, as `clm land` does,"
            ),
            "Wait for a command you run in the background, such as `clm land --check-only`, \
             through that command's own completion notice"
                .to_string()
                + BACKGROUND_WAIT
                + " Running a command in the background is a setting of the tool call, not a \
                   change to the command, so `clm land --check-only` run that way is still one \
                   plain command."
                + &LAND_WAIT.replace(
                    "{check_land}",
                    ", and a `clm land --check-only` you started,",
                ),
        ),
        None => (
            "re-run the checks.".to_string(),
            "pass the repo's checks again,".to_string(),
            "Wait for a command you run in the background, such as a long test run, through \
             that command's own completion notice"
                .to_string()
                + BACKGROUND_WAIT
                + " Running a command in the background is a setting of the tool call, not a \
                   change to the command."
                + &LAND_WAIT.replace("{check_land}", ""),
        ),
    };
    // `clm run` has no alias for the session's own model, so it names it in words.
    let auditor_model = match &job.review_model {
        Some(model) => format!("with `model: \"{model}\"` in the Agent tool's call"),
        None => "with the Agent tool's `model` set to the alias of your own model".to_string(),
    };
    // Only the component that builds the capability tests is told what its capabilities are to
    // it (`prompt-serves`).
    // Only while the root names no `wiring` may a build edit another component's file.
    let recite = if job.wiring {
        "Re-cite a marker of your own requirement in your own files; one in another \
         component's file is listed apart under its requirement \
         (`architecture/path-ownership`)."
    } else {
        "Re-citing a marker of your own requirement, wherever its file sits, is allowed \
         (`architecture/path-ownership`)."
    };
    // Your component's files, what the land refuses once the root names `wiring`, and, while
    // it names none, the instruction that is then all that holds a build to them.
    let own = "your component's files are those your component's `tests` match, wherever they \
               sit, and those its `paths` match that no other component's `tests` match \
               (`path-ownership`, `components-own-their-tests`)";
    let own_files = if job.wiring {
        format!(
            "`clm land` refuses a build that changes a file `gate/own-files` gives as not its \
             own, naming the file and its owner, before it runs any check \
             (`land/build-holds-own-files`): {own}, beside the root's `wiring` files, which \
             every build may change, and a capability test your component's rules serve, \
             which `capability-tests-follow-rules` below says when to change."
        )
    } else {
        format!(
            "This repository's root names no `wiring`, so no program refuses a build's change \
             to another component's file, and your build keeps to its own: {own}, apart from \
             the exception `capability-tests-follow-rules` states below and the re-citing \
             above, wherever its file sits."
        )
    };
    let capability_tests = if job.capability_tests {
        " This component builds the capability tests: each capability under that heading is one \
         its tests prove at the project's public boundary, whether or not a rule serves it; \
         the capability component has no rules to serve them."
    } else {
        ""
    };
    // The marker's name is built so it never stands whole in this file, or `clm affected`
    // would take the file for a reader (`doc-readers-marked`).
    let reads = concat!("clm:", "reads");
    let doc_readers = format!(
        "A test you write or change that reads a file of the repository's own tree (not a \
         fixture's) at `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or a file \
         under `specs/`, `changes/`, `backlog/` or `docs/`, whether it opens the file or runs a \
         command that reads it, or that reads the repository's own `clm.toml`, holds a `{reads}\
         \u{20}<glob>` marker, usually a comment, whose glob matches each such path it reads: a \
         land whose code was already checked runs only `clm check` and the tests whose markers \
         match a file the branch changes (`land-checks-stay-fast`), so an unmarked reader lets a \
         spec branch that breaks it land and leaves `main` failing every later code land's \
         check. "
    );
    // Only a repository with a `[land] check` has a check to record (`worktree-check-counts`).
    let recorded = if job.check.is_some() {
        "A passing `clm land --check-only` on a committed `HEAD` that contains `origin/main`, \
         with no uncommitted change and no untracked file git doesn't ignore, is recorded, so \
         that `clm land` runs no check of its own while `main` hasn't moved since. That is why \
         you run that check after committing everything and after `clm rebase` prints `rebase \
         ok`, and why between the check and `clm land` you commit nothing, edit no tracked \
         file and run no `clm rebase`. When the check prints `not recorded:`, the land runs the \
         check itself, which costs time and nothing else. Before that check, run by hand the \
         formatter and the linter the check command runs, whatever the project's are, and the \
         tests of the code you changed, so a slip shows in seconds rather than after a whole \
         check. "
    } else {
        ""
    };
    // The component that builds the capability tests leaves a capability pending while a
    // rule it exercises is `behind` (`capability-waits-for-changed-rule`).
    let waits = if job.capability_tests {
        " When a capability's test can't pass because a rule it exercises is still `behind`, \
         its rebuild not yet landed, stop at that capability and leave it pending: file no \
         backlog item and re-cite no test whose checks still prove the entry's old text \
         (`proven-by-tests`), since `clm pending` holds a capability back while such a rule is \
         blocked and, when nothing blocks the rule, the rule's build is ready and the \
         capability's next build finds it current (`serving-rules-order-capabilities`)."
    } else {
        ""
    };
    // Every command the session runs itself ends within a limit
    // (`bounded-test-runs`); `clm land --check-only` is run as written, so it is named only
    // where the repository sets a check.
    let as_written = if job.check.is_some() {
        " and `clm land --check-only`, which you run as written,"
    } else {
        ""
    };
    let bounded = format!(
        "Every command you run yourself, other than `clm land`{as_written}, ends within a time \
         limit (`bounded-test-runs`): run it in the foreground, where the tool call's own \
         timeout ends it, or, when you run it in the background, as `timeout <limit> \
         <command>`, with a limit well past the time the run should take, whatever the command \
         is, a test or check run or any other, and for each command of a chain you start in \
         the background. No command you run reads standard input that nothing feeds it, such \
         as `cat` with no file to read or a `cat >> <file>` with no input given, since nothing \
         ever types into a command you run: write a file with your editing tools instead; a \
         command given its input by a heredoc or a pipe, such as `clm defect` or `clm \
         workaround` with the report on stdin, is fine, since its input ends. A command that \
         never ends by itself, such as a test that hangs on a polling wait with no deadline of \
         its own or a command waiting on standard input, never ends a run started in the \
         background, so its completion notice never comes, and nothing wakes you, nor the \
         orchestrating session, which counts a waiting build as still running, so the build \
         sits idle with no one told. A test or check run your limit ends counts as a failing \
         run, whose hanging test you find as for any failure: fix the test when it is in your \
         component's files, and otherwise record it as a backlog item and name that item in \
         your pull request body."
    );
    // The build spot-checks its tests by mutation by hand only where no mutation pass after
    // the land judges them (`prompt-spot-checks`, `mutation-after-land`).
    let (spot_checks, mutants) = if job.mutate {
        ("", "")
    } else {
        (
            "Once the requirements you built pass their tests, and before the review, spot-check \
             them by mutation: for each key rule you built, break the rule in the code on purpose \
             and confirm that a test citing the requirement fails, adding or fixing a test when \
             none does. Run them safely: commit the work first, so that restoring a file from \
             that commit loses nothing; count a mutant only when the code still compiles with \
             it, since a build error fails every test and proves none; count it as killed only \
             when a named test fails, read from the run's list of failures rather than its exit \
             status; restore each mutated file from the commit and `touch` it, so a build tool \
             that judges freshness by file times rebuilds it rather than keeping the mutant in \
             the binary; run a script that loops over mutants with `sh`, not `zsh`, which \
             doesn't split an unquoted list into words; mutate nothing while the repository's \
             checks run; and end with `git status` showing no change to a tracked file. List \
             each mutant, with the test that failed or the test you added, under `## Review` in \
             the pull request body.\n\n",
            " a spot-check's runs of a mutant included,",
        )
    };
    // Before any test run, a build prepares its fresh worktree (`prompt-setup`).
    let setup = if job.setup.is_some() {
        format!(
            "Right after you have switched to `{branch}` and checked the branch, and before you \
             run any test, run `clm setup` from the worktree's top level as one plain command. \
             It runs the project's `[build] setup`, so that this build's tests run against this \
             worktree's own code and dependencies, never the main checkout's \
             (`tests-run-in-their-own-tree`). Run `clm setup` again after your own edit to the \
             project's dependency declarations and after a `clm rebase` that brought such an \
             edit in from `origin/main`. A `clm setup` that fails because of the branch's own \
             edits is fixed and run again. One that fails otherwise, or that Claude Code \
             refuses or denies, is reported (a refusal or denial with `clm defect`) and ends \
             the build with `stopped: setup failed`, since no test result is then to be \
             trusted."
        )
    } else {
        "This repository declares no setup step, so nothing prepares your worktree before its \
         tests run. When a test run resolves code or dependencies from outside the worktree, \
         such as from a parent directory or an environment the checkouts share, that is a \
         defect of the project (`tests-run-in-their-own-tree`): record it as a `bug` backlog \
         item whose body names `tests-run-in-their-own-tree`, and install what your tests \
         need into your own worktree by hand before you trust a test's result."
            .to_string()
    };
    let ship = format!(
        "Commit everything, then {sync}. Then {again} confirm \
         `clm gate --base origin/main` prints `code`, or `spec` for a branch that changes no \
         code file, such as one whose only change deletes a resolved backlog item or is \
         `clm rebase`'s archive move, which lands on checks (`three-routes-onto-main`), and push with \
         `clm branch push`. Write the whole pull request body with `Write` to \
         `{PR_BODY}` (outside git, like the logs) and open the branch's pull request with \
         `clm pr open {branch} --title <title> --body-file {PR_BODY}`. To correct the body, \
         rewrite that file and run `clm pr body {PR_BODY}`. A host may cut a long body, so the \
         body puts what a reviewer needs first and stays short. When `clm pr open` fails, you \
         still land with `clm land`, since nothing waits on a pull request (`land-locally`), \
         and you do not retry `clm pr open` by another route. Whenever `clm pr open` or \
         `clm pr body` says the body is kept at a path, your final message names that path, \
         since your worktree, and `{PR_BODY}` with it, is removed once it lands. The body has `## Plan`, \
         `## Review`, `## Sync`, `## Workarounds`, `## Fixes in other components`, `## Left pending` and \
         `## CLM defects`. \
         Summarise your step plan under `## Plan`, under `## Review` list every finding and \
         what you did about it, and under `## Workarounds` in the pull request body list your workaround flags.\n\n\
         Then land with `clm land` and go by what it prints. {recorded}A line `landed <sha> ...` means \
         the branch is on `main` and the build is done, whatever lines follow it (such as \
         `awaiting review: ...` or a warning). Otherwise go by its last line. \
         `gate/up-to-date`, or a conflict with `main`, means {sync}, then push with \
         `clm branch push` and run `clm land` again. A failing check means fix \
         it, commit, push and run `clm land` again. When the same failure comes back after \
         a fix, or for anything else, stop and say what it printed."
    );
    let mut p = format!(
        "Run `git switch {branch}` and check that `git branch --show-current` prints \
         `{branch}` before doing anything else.\n\n\
         You are running unattended: no one will answer questions or approve anything. When a \
         choice comes up, decide it by this repository's rules (`CLAUDE.md`, \
         `ARCHITECTURE.md`, `docs/decisions/`) and keep going. Never run `clm approve`, \
         and never push `main` yourself: only `clm land` does. Claude Code may \
         ask the human to confirm a tool call: that is the session's permission mode at work, \
         not a question for you to wait on. Keep the scratch files you write while working \
         (context excerpts, notes, intermediate output) under this worktree's `{WORKDIR}/`, \
         beside your plan and pull request body, which git ignores, and never in a scratchpad or \
         temporary directory your system prompt names or in any other directory shared with \
         other sessions: the builds and drafts launched beside you are given the same \
         directory and would overwrite one another's files. End your final message with one line: \
         `landed <sha>`, with the commit `clm land` landed, or `stopped: <reason>` when \
         nothing landed. {background} Before you write your final message, landed or stopped, stop every \
         other background task you started that is still running (background shells, monitors, \
         watches) and wait for every subagent you launched to return, so that no task of a \
         finished build wakes the orchestrating session later. {bounded}\n\n\
         {setup}\n\n\
         When Claude Code refuses a write, don't write it another way, such as through Bash: \
         treat it as a step only a human can take, as below.\n\n\
         Use the skills {skills}. For each, read and follow that file, the pinned copy every \
         build follows, and never invoke an installed copy of the skill.\n\n\
         Name the model of every subagent you launch yourself with the Agent tool: \
         `model: \"sonnet\"` for each, except the review's auditor below, whose model the \
         review step names, and never launch a subagent without naming its model.\n\n\
         On branch `{branch}`, which claims component `{node}`, push your commits as you go to \
         keep the claim alive. Every land goes to `main`.\n\n\
         Never install or replace the `clm` binary on your `PATH`, with `cargo install` or \
         otherwise: the orchestrating session and the other builds run that binary, and \
         replacing it would change the commands they rely on mid-session. Running the `clm` \
         this branch builds without installing it, such as with `cargo run`, is fine, and so \
         is installing other tools. When that binary is missing or lacks a command the work \
         needs, stop and say why rather than installing one. A `clm` that runs but only lacks \
         `clm workaround` is not such a case: carry on as the workaround paragraph below says.\n\n\
         Bring component `{node}`'s code up to its spec. First write a step plan with \
         `superpowers:writing-plans` to `{WORKDIR}/plans/{node}.md` in this worktree. It stays \
         outside git (`{WORKDIR}/` is excluded), so never `git add` it. Its `Spec:` line cites \
         `{spec}`. Then execute the plan yourself, in this session: ignore the plan header's \
         required sub-skill, never invoke {execution}, and don't wait for anyone to review the \
         plan.\n\n\
         Build each pending requirement below test-first, most urgent first. Each test cites \
         its requirement at the current version, `req:<id>@<version>`, which is what makes it \
         current; cite the versions listed here. The component's rules serve the capabilities \
         under `# Capabilities served` in your context bundle, when it has one; build them to \
         that purpose.{capability_tests} {doc_readers}An architectural constraint is proven by the rules its \
         `#### Enforced by` names and is never cited by a test (`human-layer-proof`); one \
         that says only `#### Untested` needs no proof. \
         For an `obsolete` requirement, remove the \
         code its markers mark that no other requirement needs (`no-dead-code`), unless its \
         entry below says it was not your component's, and delete \
         or keep each of its tests as its entry below says (`removal-needs-a-yes`). {recite} \
         {own_files} Put each new test in a file your \
         component's `tests` match, or its `paths` when it sets no `tests`, and never add a test \
         to a file another component owns. When a requirement you build can't be built within \
         those files (it needs a file no glob of your component matches, an import its \
         `depends_on` doesn't allow, or an edit to another component's code \
         or tests, as when what it removes from or adds to its Contract breaks a component that \
         still uses it), keep every component building as it stands, leave the requirement \
         unbuilt, and add a `question` backlog item whose `components` name the components \
         concerned, with `blocks` naming the requirement and saying what spec change it waits \
         on: a glob, a dependency, or another component's requirement that moves that component \
         first (`coupled-changes-sequence`); never move an import, hide a file, edit a spec path \
         or ask for a history rewrite to get past a check, and never add or widen an exemption, \
         which only a change a human reviews does (`temporary-exemptions`). A file of your \
         component that an exemption names still holds every other rule, and you add nothing to \
         such a file that the exemption would excuse. For an `obsolete` requirement whose \
         code another component still uses, remove its markers and tests as usual, but keep that \
         code, with no `clm:remove-by` marker, and add a `deferred` item without `blocks`, whose \
         `components` name both components and whose body names the kept code, the component \
         that uses it, and that the code goes once that component no longer uses it \
         (`contract-removal-needs`, `deferred-work-resolves-in-code`). Files the root's Design \
         gives every build to edit, such as the CLI's dispatch, the workspace manifest and its \
         lockfile, aren't another component's code for this rule. \
         When a requirement you build changes what a test in the `capability` component's `paths` \
         observes at the product's boundary (a screen, a path, a message or an output the test \
         expects), update that test in your own branch, since your land runs the test and the \
         `capability` component holds no requirement that could carry the move \
         (`path-ownership`, `capability-tests-follow-rules`): change only the test's \
         expectations, and the steps that reach them, where the requirement changed them, to \
         what the requirement states; keep the test's citation as it is and drop no check the \
         capability's scenarios, as `CAPABILITIES.md` now states them, state, so when the change \
         edited the entry the test follows the entry's current text while its citation stays at \
         the old version, leaving the capability `behind` for its own build to re-cite \
         (`proven-by-tests`); and name each such fix under `## Fixes in other components` in \
         your pull request body. A requirement under which a capability entry, as \
         `CAPABILITIES.md` now states it, no longer holds is not such a fix: leave the \
         requirement unbuilt, keeping the behaviour the test checks, and add a `question` \
         backlog item whose `components` name both components, with `blocks` naming the \
         requirement, and which names the entry, so a spec change edits it.{waits} Keep the \
         code lean and true to the component's Contract and Design, folding in the cleanups the spec asks for. When the list \
         is long, you may open your pull request and land once the tree is green with at most \
         {BUDGET} requirements built, leaving the rest pending for the next run; list what you \
         left under `## Left pending` in the pull request body. A landed branch is progress whatever \
         it leaves.\n\n\
         {spot_checks}\
         Your build's slot is {slot}. The project's tests run beside other builds' tests on this \
         host (`tests-run-concurrently`), so every test command you run by hand,{mutants} runs \
         with `CLM_SLOT={slot}` set before the \
         command{check_slot}. A test you write binds no fixed port, writes no \
         fixed path outside its checkout and the system's temporary directory, and starts no \
         server another run could share; a port or path it must know in advance is keyed on \
         `CLM_SLOT`. A test of the project that fails because another run holds a port or a \
         path is a defect of the project's tests: don't retry it, and when the fix lies outside \
         your component, report it as a `bug` backlog item whose body names \
         `tests-run-concurrently`.\n\n\
         After each step of your step plan, commit, {sync}, then push with \
         `clm branch push`, which also keeps your claim alive. When the remote \
         rejects a push as `remote rejected` with `(failure)`, or with `remote: fatal error`, \
         other builds are usually pushing at the same moment: wait a few seconds and push \
         again, at most twice more, before you stop and report git's message. That holds for \
         every push you make. Never retry a hook's decline (`pre-receive hook declined`) or a \
         lease rejection (`stale info`) that way: the first is a decision, and the second \
         means the branch moved, which `clm rebase` handles. {spec_changed}\n\n\
         When the work needs a fact only a human knows (such as a domain or an account name) \
         or a step only a human can take (such as creating an account or setting a secret), \
         don't guess. Add `backlog/<slug>.md` with kind `question` or `human-task`, `components` \
         `[{node}]`, and `blocks` listing the requirements that can't be built without it, \
         saying exactly what is needed and why. Build the rest, and land what you have. When \
         only something outside the component depends on the answer, such as a deploy, add the \
         item without `blocks` and carry on. When a short answer settles a `question` item, \
         list the answers that settle it under `choices` (two to five, each a short line), so \
         the human can answer it with a tap in the web app; list none for a question that \
         needs explaining.\n\n\
         Your branch may add backlog items, add requirements to an existing item's `blocks`, \
         and delete items a live change lists in `resolves`. Any other backlog edit, such as \
         switching an item to kind `human-task`, makes the branch mixed: don't make it, and list \
         it under `## Backlog` in the pull request body for a spec branch to make; a requirement you \
         ask a spec branch for there names the entry it serves. When your work \
         fixes an item that a live change lists in `resolves`, delete its \
         `backlog/<slug>.md` on your own branch, this one, so the requirements it blocked \
         are released when that branch lands. When your branch does the work of a `deferred` \
         item whose `components` name your component, such as removing code a build kept for another \
         component that no longer uses it, delete its `backlog/<slug>.md` on your own branch in a \
         commit whose trailer block holds `Resolves: backlog/<slug>`. A `bug` item whose \
         `components` name your component and whose `unproven` lists a requirement says the \
         mutation pass found that the requirement's tests notice no change to its code \
         (`verify/files-unproven`): mend each such test so it fails when the code it proves \
         breaks, or say in the resolving commit why the test is sound, and delete the item's \
         `backlog/<slug>.md` in a commit whose trailer block holds `Resolves: backlog/<slug>`. \
         A `question`, `human-task` \
         or `idea` item you can't delete that way goes under `## Backlog` in the pull request body, \
         as any other backlog edit you can't make.\n\n\
         Flag a workaround against a requirement when one of these happens while you build it: \
         you needed a special case or workaround to satisfy a scenario; you could only write \
         a test by faking something the requirement doesn't mention; you needed to change the \
         Design of another component to make the requirement buildable; or you had to stop to \
         raise a spec change, or the spec-auditor found a scenario the code can't observe. \
         Then run `clm workaround {node}/<slug> --title <title>`, the title naming the trigger, \
         with the situation on stdin, and commit the file with your work. A flag changes \
         nothing about the work: you still build the requirement, and a flag never blocks the \
         build or the land, with one exception. When `clm workaround` exits 3, also when you \
         re-record a flag, the capability the requirement serves has too many unsettled \
         workarounds (`redesign-before-workaround`): that isn't a failure, so keep the flag, \
         stop working on that requirement, file a `question` item that names the \
         capability and blocks the requirement, and build the rest. When the `clm` on your \
         `PATH` lacks `clm workaround`, list the \
         flag under `## Workarounds` alone, write no flag file by hand, and carry on. When a \
         requirement you flagged on this branch moves under you (`clm rebase` reports \
         `spec changed {node}`, and `clm check` then rejects your flag at its old version, \
         the one way a flag can hold up the land), re-record it at the new version: delete \
         your flag file and run `clm workaround` again; or delete it when the new text removes \
         the workaround. Flags that are already on `main` aren't yours to edit.\n\n\
         Then review the diff: run `/code-review {REVIEW_LEVEL} origin/main...HEAD`, and \
         launch the `{SPEC_AUDITOR}` subagent for component `{node}`, {auditor_model}. Both \
         reviewers run before every land, whatever the diff's size or kind (only markers, \
         only tests, a few lines): a small diff is never a reason to skip either. Name both \
         reviewers under `## Review` in the pull request body, each with its findings and \
         what you did about them, or `no findings`. Fix the findings you agree with and \
         {rerun} {ship}\n",
        spec = job.spec,
        execution = listing(EXECUTION_SKILLS, "or"),
    );
    if job.resumed {
        p.push_str(&format!(
            "\nThe branch `{branch}` already exists, from an earlier session, with commits \
             `origin/main` lacks. Start with `clm rebase` and handle what it reports as above, \
             then land the same branch as above; don't start another. If an earlier session \
             opened its pull request, `clm pr open` prints that pull request's URL instead of opening \
             another and leaves the old body, so then also run `clm pr body {PR_BODY}`.\n"
        ));
    }
    p.push_str("\n## Pending requirements, most urgent first\n");
    if job.read_from_main {
        p.push_str(&format!(
            "\nThe lists below were read from `origin/main` because the branch conflicts with \
             it, so they lack the markers your branch's own commits hold and show requirements \
             it built as pending: `clm pending {node}` after the first `clm rebase` is the \
             list to build from.\n"
        ));
    }
    if job.pending.is_empty() {
        p.push_str("\nNothing is pending.\n");
    }
    for item in &job.pending {
        p.push_str(&format!(
            "\n- `{}` {} `{}`\n",
            item.id,
            item.state,
            item.version.as_deref().unwrap_or("-")
        ));
        for (cited, diff) in &item.diffs {
            match diff {
                Some(diff) => p.push_str(&format!(
                    "  The text moved from the version `{cited}` some marker cites to the \
                     current text:\n```diff\n{diff}```\n"
                )),
                None => p.push_str(&format!(
                    "  The earlier text at version `{cited}` wasn't found in the history of \
                     `{}`; compare against your tests.\n",
                    job.spec
                )),
            }
        }
        if item.state == "obsolete" {
            if let Some(e) = &item.elsewhere {
                p.push_str(&format!(
                    "  A live change removed it, and it was not `{node}`'s: {} removed it. Its \
                     markers in your files are at {}; remove no code for it.",
                    listing_owned(&e.removed_by, "and", true),
                    item.markers.join(", ")
                ));
            } else if item.markers.is_empty() {
                p.push_str("  A live change removed it.");
            } else {
                p.push_str(&format!(
                    "  A live change removed it; its markers are at {}. Remove the code they \
                     mark that no other requirement needs.",
                    item.markers.join(", ")
                ));
            }
            if !item.delete.is_empty() {
                p.push_str(&format!(
                    " Delete the tests an approved list names (`land/deleted-tests-approved`): \
                     {}.",
                    listing_owned(&item.delete, "and", true)
                ));
            }
            if !item.keep.is_empty() {
                p.push_str(&format!(
                    " `clm land` refuses this build to lose every other test that cites it: {}; \
                     re-cite each to the requirement that now states what it proves and, where \
                     none does, leave it and the code it runs and add a `question` item whose \
                     `blocks` names `{}` (`model/parse-backlog`), so its line waits for the \
                     human's answer: a spec change that lists the test in a \
                     `deleted-tests.txt` or states what it proves.",
                    listing_owned(&item.keep, "and", true),
                    item.id
                ));
            }
            if let Some(e) = &item.elsewhere {
                p.push_str(&elsewhere(node, &item.id, e));
            }
            p.push('\n');
        } else if item.state == "behind" && !item.markers.is_empty() {
            // Only while the root sets no `wiring` may a build edit another component's file.
            let wherever = if job.wiring {
                ""
            } else {
                ", wherever its file sits, another component's `paths` included,"
            };
            p.push_str(&format!(
                "  Re-cite each marker that cites it at another version or without one{wherever} \
                 after checking that its test proves the current text (`path-ownership`): {}. When a test proves what \
                 the current text no longer states, because the diff above moved its scenario \
                 to another requirement or removed it, take its marker off instead of re-citing \
                 it; the requirement that now states it is cited by its own build.\n",
                item.markers.join(", ")
            ));
        }
        if item.state == "behind" && !item.delete.is_empty() {
            p.push_str(&format!(
                "  Delete, and never re-cite, each test of a file you may change that cites it \
                 and that an approved list names (`land/deleted-tests-approved`), since a change \
                 lists a test to delete though its requirement stays, such as a duplicate or the \
                 test of a sentence the requirement lost (`cuts-are-what-a-program-lists`): {}. \
                 When no other test cites it once they are gone, write one that proves its \
                 current text.\n",
                listing_owned(&item.delete, "and", true)
            ));
        }
        if item.state == "behind" && !item.foreign.is_empty() {
            let each: Vec<String> = item
                .foreign
                .iter()
                .map(|(at, owner)| format!("`{at}` in `{owner}`'s file"))
                .collect();
            p.push_str(&format!(
                "  Leave each marker that cites it at another version or without one in a file \
                 another component owns: {}. `clm land` refuses this build's edit to such a \
                 file (`land/build-holds-own-files`), and `clm check` reports the marker unless \
                 an exemption covers its file (`check/tests-cite-their-owner`); add a \
                 `question` item whose `blocks` names `{}`.\n",
                listing_owned(&each, "and", false),
                item.id
            ));
        }
    }
    if !job.blocked.is_empty() {
        p.push_str("\nBlocked, left out of the list above; build nothing that relies on them:\n");
        for b in &job.blocked {
            let by = (!b.by.is_empty())
                .then(|| format!("blocked by {}", listing_owned(&b.by, "and", true)));
            let cited_in = (!b.cited_in.is_empty()).then(|| {
                format!(
                    "waiting on {}, whose files still cite it",
                    listing_owned(&b.cited_in, "and", true)
                )
            });
            let what: Vec<String> = by.into_iter().chain(cited_in).collect();
            p.push_str(&format!("- `{}`, {}\n", b.id, what.join(", and ")));
        }
    }
    if let Some(bundle) = &job.bundle {
        p.push_str(&format!("\nThe context bundle for `{node}`:\n\n{bundle}"));
    }
    p
}

/// What the build of `node` does with obsolete `id`'s markers when `id` was another
/// component's, or a component's the spec graph doesn't hold (`pending-prompt`).
fn elsewhere(node: &str, id: &str, e: &Elsewhere) -> String {
    let mut p = String::new();
    if !e.recite.is_empty() {
        let each: Vec<String> = e
            .recite
            .iter()
            .map(|(test, to)| format!("`{test}` to `{to}`"))
            .collect();
        p.push_str(&format!(
            " Re-cite each test a move took to a requirement of `{node}`: {}.",
            listing_owned(&each, "and", false)
        ));
    }
    if !e.take_out.is_empty() {
        p.push_str(&format!(
            " Take out its markers at {}, each outside a test or on a test that also cites a \
             requirement the spec holds, and leave the code and the test.",
            e.take_out.join(", ")
        ));
    }
    if !e.leave.is_empty() {
        p.push_str(&format!(
            " Leave each other test that cites it, never re-citing it, since your bundle \
             doesn't hold another component's requirements: {}; add a `question` item whose \
             `blocks` names `{id}` (`model/parse-backlog`), as `check/obsolete-tests-elsewhere` \
             makes the removing change list such a test.",
            listing_owned(&e.leave, "and", true)
        ));
    }
    if let Some(owner) = &e.owner_code {
        p.push_str(&format!(
            " The code `{id}` needed is `{owner}`'s to remove: add a `deferred` item naming \
             `{owner}` and `{id}` unless the backlog holds one."
        ));
    }
    p
}

/// `items` in backticks, as `A, B and C` (or `or`).
fn listing(items: &[&str], conjunction: &str) -> String {
    let owned: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    listing_owned(&owned, conjunction, true)
}

/// `items`, in backticks when `quote`, as `A, B and C` (or `or`).
fn listing_owned(items: &[String], conjunction: &str, quote: bool) -> String {
    let items: Vec<String> = items
        .iter()
        .map(|s| if quote { format!("`{s}`") } else { s.clone() })
        .collect();
    match items.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {conjunction} {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The path of `skill`'s `SKILL.md` in the pinned superpowers copy builds follow, with the
/// environment `env` names and a relative `$CLM_SUPERPOWERS_DIR` read from `cwd`.
fn superpowers_skill(skill: &str, cwd: &Path, env: Env) -> Result<String, Error> {
    // One plain name, so it can't escape `skills/`; checked before anything is fetched.
    let mut parts = Path::new(skill).components();
    if !matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    ) {
        return Err(Error::Refused(format!(
            "`{skill}` isn't a superpowers skill name"
        )));
    }
    let path = superpowers_dir(cwd, env)?
        .join("skills")
        .join(skill)
        .join("SKILL.md");
    if !path.is_file() {
        return Err(Error::Refused(format!(
            "no superpowers skill `{skill}` at {}",
            path.display()
        )));
    }
    // A relative `$CLM_SUPERPOWERS_DIR` still prints an absolute path.
    let path = std::path::absolute(&path)
        .map_err(|e| Error::Refused(format!("{}: {e}", path.display())))?;
    Ok(path.display().to_string())
}

/// `clm setup` (`setup-command`): runs the repository's `[build] setup` in its root, through
/// `sh -c` with standard input closed, and returns what to print on success. A command that
/// fails is a refusal naming the command and its status; none set prints that and runs nothing.
/// The settings' `[env]` variables are set for the command, over the caller's own. What the
/// command prints is passed through to `out` and `err` as it comes; when either can't be
/// written, the result is [`Error::Git`] once the command has ended, which the call exits 2
/// with (`commands-in-crate`).
pub fn setup(
    root: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<String, Error> {
    let settings = clm_config::load(Some(root), env).map_err(|e| Error::Refused(e.to_string()))?;
    let Some(build) = settings.build.value else {
        return Ok("no [build] setup".to_string());
    };
    let cant_run = |e: std::io::Error| {
        Error::Refused(format!("setup failed: can't run `{}`: {e}", build.setup))
    };
    let mut child = std::process::Command::new("sh")
        .args(["-c", &build.setup])
        .current_dir(root)
        .envs(settings.env.iter().map(|(n, s)| (n, &s.value)))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(cant_run)?;
    let (tx, rx) = std::sync::mpsc::channel();
    if let Some(pipe) = child.stdout.take() {
        pump(pipe, 0, tx.clone());
    }
    if let Some(pipe) = child.stderr.take() {
        pump(pipe, 1, tx.clone());
    }
    std::thread::spawn(move || {
        let _ = tx.send(Piped::Ended(child.wait()));
    });
    // Until `sh` ends, and then until both pipes close, or for a moment more when a process
    // the command left running in the background still holds one, so it never holds up
    // `clm setup`. A stream that fails is written to no more, and the pipes are still
    // drained, so the command never blocks on a full one.
    let mut streams: [(&mut dyn Write, bool); 2] = [(out, true), (err, true)];
    let (mut ended, mut closed) = (None, 0);
    while ended.is_none() || closed < 2 {
        let next = match ended {
            None => rx.recv().ok(),
            Some(_) => rx.recv_timeout(std::time::Duration::from_millis(200)).ok(),
        };
        match next {
            Some(Piped::Bytes(i, bytes)) => {
                let (stream, ok) = &mut streams[i];
                if *ok {
                    *ok = stream
                        .write_all(&bytes)
                        .and_then(|()| stream.flush())
                        .is_ok();
                }
            }
            Some(Piped::Closed) => closed += 1,
            Some(Piped::Ended(status)) => ended = Some(status),
            None => break,
        }
    }
    // Once the command has ended, whatever its status: what it printed didn't all get
    // through, which is `clm setup`'s own failure (exit 2).
    let unwritten = streams.iter().any(|(_, ok)| !ok);
    let status = match ended {
        Some(status) => status,
        // Every sender is gone without the end: only the waiter's panic does that.
        None => Err(std::io::Error::other("lost the command's exit status")),
    }
    .map_err(cant_run)?;
    if unwritten {
        return Err(Error::Git(format!(
            "can't write what `{}` printed",
            build.setup
        )));
    }
    if status.success() {
        return Ok(String::new());
    }
    let how = match (status.code(), signal(&status)) {
        (Some(code), _) => format!("exit {code}, status {code}"),
        (None, Some(n)) => format!("killed by signal {n}"),
        (None, None) => "no exit status".to_string(),
    };
    Err(Error::Refused(format!(
        "setup failed: `{}` ended with {how}",
        build.setup
    )))
}

/// What `setup` hears from its command: output on stream `0` (stdout) or `1` (stderr), a
/// pipe that closed, or how `sh` ended.
enum Piped {
    Bytes(usize, Vec<u8>),
    Closed,
    Ended(std::io::Result<std::process::ExitStatus>),
}

/// Sends what `pipe` yields to `tx` as stream `i`'s, then that it closed.
fn pump(
    mut pipe: impl std::io::Read + Send + 'static,
    i: usize,
    tx: std::sync::mpsc::Sender<Piped>,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(Piped::Bytes(i, buf[..n].to_vec())).is_err() {
                        return;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        let _ = tx.send(Piped::Closed);
    });
}

#[cfg(unix)]
fn signal(status: &std::process::ExitStatus) -> Option<i32> {
    std::os::unix::process::ExitStatusExt::signal(status)
}

#[cfg(not(unix))]
fn signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Prepares the build of `node`, writes its prompt to `<root>/.clm/prompts/<node>.md` and
/// returns the launcher that names it (`build-prompt-file`): landed leftovers of an earlier
/// build are removed (`merged-claims-released`), then the local branch `impl/<node>` is
/// resumed when it holds the claim, or the node is claimed and the branch made at the claim.
/// The build holds the clone's place `<node>` from here on (`build-places`).
///
/// A relative `root` is read from `cwd`, the directory the command runs in, and `env` is the
/// environment the settings and the superpowers copy are read with.
pub fn run(
    root: &Path,
    cwd: &Path,
    env: Env,
    node: &str,
    expiry_hours: u64,
    review_model: Option<&str>,
) -> Result<String, RunError> {
    let root = &cwd
        .join(root)
        .canonicalize()
        .map_err(|e| Error::Refused(format!("{}: {e}", root.display())))?;
    if node.is_empty() || node.contains('/') {
        return Err(Error::Refused(format!("`{node}` isn't a component ID")).into());
    }
    // Before anything is claimed, so settings that can't be loaded leave nothing behind.
    let settings = clm_config::load(Some(root), env).map_err(|e| Error::Refused(e.to_string()))?;
    let check = settings.land.value.map(|land| land.check);
    let setup = settings.build.value.map(|build| build.setup);
    let mutate = settings.verify.value.is_some_and(|v| v.mutate.is_some());
    // Before anything is claimed, so a missing skill leaves nothing behind.
    let skills = SKILLS
        .iter()
        .map(|s| {
            Ok((
                s.to_string(),
                superpowers_skill(s.trim_start_matches("superpowers:"), cwd, env)?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let branch = format!("{BRANCH_PREFIX}{node}");

    fetch(root)?;
    // Before the place: a landed leftover would make the new place read as landed.
    tidy(root, node, &branch)?;
    // Before anything else is written or removed, so a full clone leaves nothing behind but
    // the landed leftovers. A place
    // this run takes anew is freed when the run then fails; one it reuses stays with the
    // build it belongs to.
    let placed = take_place(
        root,
        node,
        PlaceKind::Build,
        std::slice::from_ref(&branch),
        settings.max_builds.value,
        expiry_hours,
    )?;
    if let Placed::Full { held, max_builds } = placed {
        return Err(RunError::NoPlace(format!(
            "no free build place: {held} of max_builds = {max_builds} in use; wait for a \
             build to land, or run `clm run --finish <component>` for one whose session has \
             returned"
        )));
    }
    let built = build(
        root,
        node,
        &branch,
        expiry_hours,
        skills,
        Settled {
            check,
            setup,
            mutate,
        },
        review_model,
    );
    if built.is_err() && placed == Placed::New {
        // Best effort: a place left behind stops counting once its time is up.
        let _ = free_place(root, node);
    }
    Ok(built?)
}

/// Why `clm run` prepared no build.
#[derive(Debug)]
pub enum RunError {
    /// The clone has no free build place (exit 3); the message names the cap.
    NoPlace(String),
    /// The run was refused, or git failed, as the [`Error`] says.
    Failed(Error),
}

impl From<Error> for RunError {
    fn from(e: Error) -> Self {
        RunError::Failed(e)
    }
}

/// The repository settings a prompt names.
struct Settled {
    /// `[land] check`.
    check: Option<String>,
    /// `[build] setup`.
    setup: Option<String>,
    /// Whether `[verify]` sets `mutate`.
    mutate: bool,
}

/// What `run` does once the build holds its place.
fn build(
    root: &Path,
    node: &str,
    branch: &str,
    expiry_hours: u64,
    skills: Vec<(String, String)>,
    settled: Settled,
    review_model: Option<&str>,
) -> Result<String, Error> {
    // Before the claim, so a failure here leaves no claim behind.
    exclude_workdir(root)?;
    let prompts = root.join(PROMPTS);
    fs::create_dir_all(&prompts)
        .map_err(|e| Error::Refused(format!("{}: {e}", prompts.display())))?;
    let resumed = prepare(root, node, branch, expiry_hours)?;

    // The lists come from the branch merged into `origin/main`, never from the checkout
    // (`pending-prompt`); the branch never edits `specs/`, so their history is `main`'s.
    let main = format!("{REMOTE}/main");
    let merged = merged_tree(root, &main, &format!("refs/heads/{branch}"))?;
    let read_from_main = merged.is_none();
    let merged = merged.as_deref().unwrap_or(&main);
    let tmp = clm_git::extract_tree(root, merged)?;
    let tree = tmp.path();
    let (graph, markers, pending) = pending_in(tree, node)?;
    let mut job = job(
        root,
        &main,
        (merged, tree),
        &graph,
        &markers,
        node,
        &pending,
    )?;
    job.resumed = resumed;
    job.read_from_main = read_from_main;
    job.skills = skills;
    job.check = settled.check;
    job.setup = settled.setup;
    job.mutate = settled.mutate;
    job.review_model = review_model.map(str::to_string);
    job.slot = place_slot(root, branch)?;
    // A write that fails leaves the claim, which the next `clm run` resumes. The prompt goes
    // to a temporary file renamed over the earlier one, so a failed write leaves that whole.
    let file = prompts.join(format!("{node}.md"));
    let written = tempfile::NamedTempFile::new_in(&prompts).and_then(|mut tmp| {
        tmp.write_all(prompt(&job).as_bytes())?;
        tmp.persist(&file).map_err(|e| e.error)?;
        Ok(())
    });
    written.map_err(|e| {
        Error::Refused(format!(
            "{}: {e}; run `clm run {node}` again once it can be written",
            file.display()
        ))
    })?;
    Ok(launcher(&file, node))
}

/// The tree `git merge-tree --write-tree` writes for `branch` merged into `main`, or none
/// when they conflict.
fn merged_tree(root: &Path, main: &str, branch: &str) -> Result<Option<String>, Error> {
    match git(root, &["merge-tree", "--write-tree", main, branch]) {
        Ok(out) => Ok(out.lines().next().map(str::to_string)),
        // Exit 1: conflicts.
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// `clm run --finish <node>` (`build-places`): frees the place `<node>` and, when
/// `impl/<node>` has landed, removes each clean worktree under the main worktree's
/// `.claude/worktrees/` that has it checked out and whose lock no running process holds
/// (`vcs/held-lock`), or, when `returned` says the build's subagent has handed back,
/// whatever its lock says. Reads no queue state,
/// so it works for any node. Returns what it did, one line each. A relative `root` is read
/// from `cwd`, the directory it runs in, whose worktree is never removed.
pub fn finish(
    root: &Path,
    cwd: &Path,
    node: &str,
    expiry_hours: u64,
    returned: bool,
) -> Result<String, Error> {
    let root = &cwd
        .join(root)
        .canonicalize()
        .map_err(|e| Error::Refused(format!("{}: {e}", root.display())))?;
    if node.is_empty() || node.contains('/') {
        return Err(Error::Refused(format!("`{node}` isn't a component ID")));
    }
    let branch = format!("{BRANCH_PREFIX}{node}");
    fetch(root)?;
    let mut done = Vec::new();
    if clm_orchestrator::places(root, expiry_hours)?
        .iter()
        .any(|p| p.name == node)
    {
        free_place(root, node)?;
        done.push(format!("freed {node}"));
    }
    if landed(root, &branch)? {
        // How `build` learns that a build whose subagent ended without a closing line landed.
        done.push(format!("landed {branch}"));
        let list = worktrees(root)?;
        // Claude Code's worktrees, under the main worktree whichever worktree this runs in.
        let ours = match list.first() {
            Some(main) => main.path.join(".claude/worktrees"),
            None => return Ok(done.join("\n")),
        };
        // The worktree this runs in is the orchestrating session's own, never a build's.
        let here = cwd.canonicalize().ok();
        let own = |p: &Path| root.starts_with(p) || here.as_ref().is_some_and(|h| h.starts_with(p));
        for wt in &list[1..] {
            // One whose directory is gone has nothing to remove.
            if wt.branch.as_deref() != Some(branch.as_str())
                || !wt.path.starts_with(&ours)
                || own(&wt.path)
                || !wt.path.is_dir()
            {
                continue;
            }
            if !clean(&wt.path)? {
                done.push(format!("kept {}: uncommitted changes", wt.path.display()));
            } else if wt.held && !returned {
                // A build still running there, waiting on its land or writing its report
                // (`a-held-lock-keeps-a-worktree`): kept, and left locked. Once its subagent
                // has handed back, the lock Claude Code keeps names only the session.
                done.push(format!(
                    "kept {}: a running process holds its lock",
                    wt.path.display()
                ));
            } else {
                done.push(match clm_git::remove_worktree(root, &wt.path).map_err(|e| e.to_string()) {
                    Ok(()) => format!("removed {}", wt.path.display()),
                    Err(why) => format!(
                        "left {}: {}; delete the directory by hand, then run git worktree prune",
                        wt.path.display(),
                        why.replace('\n', " ").trim()
                    ),
                });
            }
        }
    }
    Ok(done.join("\n"))
}

/// Whether `branch` has landed, as a build's place judges it: at least one of
/// `origin/<branch>` and the local `<branch>` exists, and each that exists is on
/// `origin/main`.
fn landed(root: &Path, branch: &str) -> Result<bool, Error> {
    let main = format!("{REMOTE}/main");
    let mut any = false;
    for rf in [
        format!("refs/remotes/{REMOTE}/{branch}"),
        format!("refs/heads/{branch}"),
    ] {
        if git(root, &["rev-parse", "--verify", "-q", &rf]).is_err() {
            continue;
        }
        if !clm_git::is_ancestor(root, &rf, &main)? {
            return Ok(false);
        }
        any = true;
    }
    Ok(any)
}

/// What `clm run` prints: a few lines that hand the prompt at `file` to the session.
fn launcher(file: &Path, node: &str) -> String {
    format!(
        "Read `{}` with the Read tool, all of it (in parts when it is too large to read at \
         once), before anything else, and follow it as your instructions.\n\
         It is the prompt for building `{node}`, and its first step is \
         `git switch {BRANCH_PREFIX}{node}`.",
        file.display()
    )
}

/// The worktree that has `branch` checked out, refused when it holds uncommitted work.
fn checked_out(root: &Path, node: &str, branch: &str) -> Result<Option<Holder>, Error> {
    let holder = holding(root, branch)?;
    if let Some(h) = &holder
        && !clean(&h.path)?
    {
        return Err(Error::Refused(format!(
            "{} has `{branch}` checked out with uncommitted or untracked changes; inspect \
             it, then commit and push them or remove the worktree to run `{node}` again",
            h.path.display()
        )));
    }
    Ok(holder)
}

/// Removes a linked worktree that has `branch` checked out; the main checkout can't go, so
/// it has to leave the branch.
fn free(root: &Path, node: &str, branch: &str, holder: &Holder) -> Result<(), Error> {
    if holder.main {
        Err(Error::Refused(format!(
            "{} has `{branch}` checked out; switch it to another branch to run `{node}` again",
            holder.path.display()
        )))
    } else {
        clm_git::remove_worktree(root, &holder.path).map_err(|e| Error::Git(e.to_string()))
    }
}

/// Before the place (`run-takes-place`): refuses a worktree on `branch` that holds
/// uncommitted work, landed or not, and removes what an earlier build of the node left once
/// it landed (`merged-claims-released`): a clean worktree on it, then the local branch. Both
/// hold nothing `origin/main` lacks. A landed worktree whose lock a running process holds
/// (`vcs/held-lock`) may still be in use, so it is kept and the run refused
/// (`a-held-lock-keeps-a-worktree`) until `clm run --finish <node> --returned` removes it.
fn tidy(root: &Path, node: &str, branch: &str) -> Result<(), Error> {
    let local = format!("refs/heads/{branch}");
    let holder = checked_out(root, node, branch)?;
    let landed = git(root, &["rev-parse", "--verify", "-q", &local]).is_ok()
        && clm_git::is_ancestor(root, &local, &format!("{REMOTE}/main"))?;
    if landed {
        if let Some(h) = &holder {
            if h.held {
                return Err(Error::Refused(format!(
                    "{} has the landed `{branch}` checked out and a running process holds \
                     its lock, so it may still be in use; once its build has handed back, \
                     run `clm run --finish {node} --returned`, which removes it, and run \
                     `{node}` again, or, when nothing works in it any more, run \
                     `git worktree unlock {}` first",
                    h.path.display(),
                    h.path.display()
                )));
            }
            free(root, node, branch, h)?;
        }
        git(root, &["branch", "-q", "-D", branch])?;
    } else if holder.is_none() && unpushed_claim(root, &local, branch)? {
        git(root, &["branch", "-q", "-D", branch])?;
    }
    Ok(())
}

/// Whether the local `branch` is at a claim commit nobody pushed (`claim-local-before-push`):
/// one parent, the parent's tree, the parent an ancestor of `origin/main`, on a branch that
/// never held `origin/<branch>`'s tip (or with no such branch). It holds no work, whoever
/// made it.
fn unpushed_claim(root: &Path, local: &str, branch: &str) -> Result<bool, Error> {
    if git(root, &["rev-parse", "--verify", "-q", local]).is_err() {
        return Ok(false);
    }
    let line = git(root, &["rev-list", "--parents", "-n", "1", local])?;
    let shas: Vec<&str> = line.split_whitespace().collect();
    let [tip, parent] = shas[..] else {
        return Ok(false);
    };
    let tree = |rev: &str| git(root, &["rev-parse", &format!("{rev}^{{tree}}")]);
    if tree(tip)? != tree(parent)?
        || !clm_git::is_ancestor(root, parent, &format!("{REMOTE}/main"))?
    {
        return Ok(false);
    }
    Ok(!held(root, local, branch)?)
}

/// Whether the remote branch `origin/<branch>` exists and the local branch `local` has held
/// its tip: that tip is an ancestor of the local tip or of an entry of the local branch's
/// reflog, the test git's `--force-if-includes` makes (`run`). A branch rebased since it was
/// pushed has held it, though it no longer contains it.
fn held(root: &Path, local: &str, branch: &str) -> Result<bool, Error> {
    let remote = format!("refs/remotes/{REMOTE}/{branch}");
    if git(root, &["rev-parse", "--verify", "-q", &remote]).is_err() {
        return Ok(false);
    }
    if clm_git::is_ancestor(root, &remote, local)? {
        return Ok(true);
    }
    // No reflog (or an expired one) leaves only the local tip.
    let log = git(root, &["reflog", "show", "--format=%H", local]).unwrap_or_default();
    for sha in log.lines() {
        // An entry whose commit is gone (pruned) holds nothing.
        if clm_git::is_ancestor(root, &remote, sha).unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Once the build holds its place, resumes the local branch `branch` when it contains the
/// claim, or claims the node and makes the branch at the claim. Returns whether it resumed.
/// A local branch that may hold commits nobody pushed is left as it is and the run refused.
fn prepare(root: &Path, node: &str, branch: &str, expiry_hours: u64) -> Result<bool, Error> {
    let local = format!("refs/heads/{branch}");
    let exists = |rf: &str| git(root, &["rev-parse", "--verify", "-q", rf]).is_ok();
    if exists(&local) {
        if !held(root, &local, branch)? {
            return Err(Error::Refused(format!(
                "the local branch `{branch}` isn't landed and never held \
                 `{REMOTE}/{branch}`, so it may hold commits nobody pushed, or miss commits \
                 pushed since; reconcile or delete it to run `{node}` again"
            )));
        }
        // Resume: the new build switches to the branch, so a clean worktree on it goes. Its
        // commits stay on the branch.
        if let Some(h) = checked_out(root, node, branch)? {
            free(root, node, branch, &h)?;
        }
        return Ok(true);
    }
    claim_local(root, node, expiry_hours)?;
    Ok(false)
}

/// A worktree that has a branch checked out, whether it's the main checkout, and whether a
/// running process holds its lock (`vcs/held-lock`).
struct Holder {
    path: PathBuf,
    main: bool,
    held: bool,
}

/// The worktree that has `branch` checked out, if any.
fn holding(root: &Path, branch: &str) -> Result<Option<Holder>, Error> {
    Ok(worktrees(root)?
        .into_iter()
        .enumerate()
        .find(|(_, w)| w.branch.as_deref() == Some(branch))
        .map(|(i, w)| Holder {
            path: w.path,
            main: i == 0,
            held: w.held,
        }))
}

/// The graph of the tree at `dir`, its markers and the whole list `pending` returns, every
/// component's lines, which `job` narrows to `node` and sorts.
fn pending_in(dir: &Path, node: &str) -> Result<(Graph, Vec<Marker>, Vec<Pending>), Error> {
    let graph = load(dir)?;
    if !graph.nodes.contains_key(node) {
        return Err(Error::Refused(format!("no component `{node}`")));
    }
    let markers = clm_coverage::scan(dir).req;
    let pending = clm_coverage::pending(&graph, &markers);
    Ok((graph, markers, pending))
}

/// What blocks `p`, a line of `lines`, the whole list `pending` returns: the `backlog/<slug>`
/// items whose `blocks` name it, then the other pending requirements it waits on
/// (`requirement-needs`, `moved-markers-wait`, `serving-rules-order-capabilities`), and, in
/// place of its own ID, the components its other lines are pending on, which `waits` finds
/// only in the whole list (`check.coverage/wait-lines`).
fn blockers(lines: &[Pending], p: &Pending) -> Blocked {
    let mut cited_in: Vec<String> = clm_coverage::waits(lines, p)
        .into_iter()
        .filter(|w| w.id == p.id)
        .map(|w| w.node.clone())
        .collect();
    cited_in.sort();
    cited_in.dedup();
    Blocked {
        id: p.id.clone(),
        by: p
            .blocked_by
            .iter()
            .map(|s| format!("backlog/{s}"))
            .chain(p.waits_on.iter().filter(|id| **id != p.id).cloned())
            .collect(),
        cited_in,
    }
}

/// The job for `node` in the repository at `root`, whose specs' history is `rev`'s and whose
/// tree-ish `extracted.0`, `rev` merged with the branch, is extracted at `extracted.1`, with
/// `graph`, every marker, `scanned`, and the whole list `pending` returns, `lines`, read from
/// it.
fn job(
    root: &Path,
    rev: &str,
    extracted: (&str, &Path),
    graph: &Graph,
    scanned: &[Marker],
    node: &str,
    lines: &[Pending],
) -> Result<Job, Error> {
    let (merged, tree) = extracted;
    let mut pending: Vec<&Pending> = lines.iter().filter(|p| p.node == node).collect();
    pending.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
    let spec = graph.nodes[node].path.to_string_lossy().into_owned();
    let mut history = History::new(root, rev, &spec);
    let mut items = Vec::new();
    let mut blocked = Vec::new();
    // The approved lists' lines, read once the first behind or obsolete requirement needs them.
    let mut approved: Option<Vec<ListLine>> = None;
    // Once the root sets `wiring`, the owner of each file holding a stale `behind` marker
    // that `gate` gives as not this build's to change, the files `clm land` refuses this
    // build's edit to (`gate/own-files`, `land/build-holds-own-files`).
    let not_ours: HashMap<String, String> = if graph.wiring().is_some() {
        let mut files: Vec<String> = pending
            .iter()
            .filter(|p| p.state == ReqState::Behind)
            .flat_map(|p| p.markers.iter().filter(|m| m.version != p.version))
            .map(|m| m.path.to_string_lossy().into_owned())
            .collect();
        files.sort();
        files.dedup();
        not_ours_to_change(root, rev, (merged, tree), &files, node)?
    } else {
        HashMap::new()
    };
    let foreign_owner = |m: &clm_coverage::Marker| not_ours.get(m.path.to_string_lossy().as_ref());
    for p in pending {
        let b = blockers(lines, p);
        if !b.by.is_empty() || !b.cited_in.is_empty() {
            blocked.push(b);
            continue;
        }
        let diffs = if p.state == ReqState::Behind {
            let current = graph.requirement(&p.id).map(|r| r.text.as_str());
            p.cited
                .iter()
                .map(|cited| {
                    let diff = current.and_then(|now| {
                        history
                            .earlier_text(&p.id, cited)
                            .and_then(|old| diff_text(&old, now).ok())
                    });
                    (cited.clone(), diff)
                })
                .collect()
        } else {
            Vec::new()
        };
        // Obsolete: every marker, since once the root sets `wiring`, `pending` gives each one
        // in a file another component owns to that owner's line, and without it the build may
        // change the file. Behind: those at another version or none, a foreign one apart.
        // Behind: the tests in files the build may change that cite it at any version and
        // that an approved list names, which it deletes rather than re-cites
        // (`cuts-are-what-a-program-lists`).
        let listed = if p.state == ReqState::Behind {
            let approved = approved.get_or_insert_with(|| approved_lists(tree));
            let may_change: Vec<&Marker> = p
                .markers
                .iter()
                .filter(|m| foreign_owner(m).is_none())
                .collect();
            listed_tests(tree, &may_change, approved)
        } else {
            Vec::new()
        };
        let (mut own, mut foreign) = (Vec::new(), Vec::new());
        for m in &p.markers {
            if p.state == ReqState::Behind
                && (m.version == p.version || listed.iter().any(|t| t.holds(m)))
            {
                continue;
            }
            match foreign_owner(m).filter(|_| p.state == ReqState::Behind) {
                Some(owner) => foreign.push((m.to_string(), owner.to_string())),
                None => own.push(m),
            }
        }
        let markers = own.iter().map(|m| m.to_string()).collect();
        let mut elsewhere = None;
        let (delete, keep) = if p.state == ReqState::Behind {
            (listed.into_iter().map(|t| t.name).collect(), Vec::new())
        } else if p.state == ReqState::Obsolete {
            let approved = approved.get_or_insert_with(|| approved_lists(tree));
            if owner_of_id(graph, &p.id) == node {
                obsolete_tests_in(graph, tree, &p.id, &own)
                    .into_iter()
                    .partition(|test| approved_names(approved, test))
            } else {
                let (delete, e) = not_its_own(graph, tree, scanned, node, &p.id, &own, approved);
                elsewhere = Some(e);
                (delete, Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };
        items.push(Item {
            id: p.id.clone(),
            version: p.version.clone(),
            state: p.state.to_string(),
            diffs,
            markers,
            foreign,
            delete,
            keep,
            elsewhere,
        });
    }
    let bundle = clm_context::bundle(graph, tree, node);
    Ok(Job {
        node: node.into(),
        spec,
        skills: Vec::new(),
        pending: items,
        blocked,
        bundle,
        resumed: false,
        check: None,
        review_model: None,
        capability_tests: graph.capability_tests_node() == node,
        read_from_main: false,
        slot: 0,
        setup: None,
        wiring: graph.wiring().is_some(),
        mutate: false,
    })
}

/// The component requirement `id` belongs to: the one its ID names, or for a human-layer
/// entry the one that builds the capability tests (`model/capability-tests-component`).
fn owner_of_id<'g>(graph: &'g Graph, id: &'g str) -> &'g str {
    if clm_model::Layer::of(id).is_some() {
        graph.capability_tests_node()
    } else {
        id.split_once('/').map_or(id, |(n, _)| n)
    }
}

/// For obsolete `id`, which was not `node`'s, with `own` its markers in `node`'s files: the
/// tests an `approved` list names, to delete, and what else the build does
/// (`pending-prompt`). A test goes to the first that fits: an approved list names it; a
/// move takes `id` to a requirement of `node`, to re-cite it to; it also cites a requirement
/// the spec holds, so only its marker goes; else it is left. A marker outside a test goes.
fn not_its_own(
    graph: &Graph,
    tree: &Path,
    scanned: &[Marker],
    node: &str,
    id: &str,
    own: &[&Marker],
    approved: &[ListLine],
) -> (Vec<String>, Elsewhere) {
    let moved = clm_coverage::moved_to(graph, id)
        .into_iter()
        .find(|to| to.split_once('/').is_some_and(|(n, _)| n == node));
    let named = id.split_once('/').map_or(id, |(n, _)| n);
    // A file of `named`'s, as `check.coverage/pending-report` counts them: one it owns, one
    // a `wiring` glob matches, or one no component owns.
    let of_named = |m: &Marker| {
        let file = m.path.to_string_lossy();
        let wired = graph
            .wiring()
            .is_some_and(|w| w.iter().any(|g| clm_coverage::matches(g, &file)));
        wired || clm_coverage::owner(graph, &m.path).is_none_or(|o| o == named)
    };
    let mut e = Elsewhere {
        removed_by: clm_coverage::removed(graph).remove(id).unwrap_or_default(),
        owner_code: (clm_model::Layer::of(id).is_none()
            && graph.nodes.contains_key(named)
            && !scanned.iter().any(|m| m.id == id && of_named(m)))
        .then(|| named.to_string()),
        ..Elsewhere::default()
    };
    let mut take_out: Vec<&Marker> = Vec::new();
    let mut delete = Vec::new();
    let mut paths: Vec<&PathBuf> = own.iter().map(|m| &m.path).collect();
    paths.sort();
    paths.dedup();
    for path in paths {
        let text = fs::read_to_string(tree.join(path)).unwrap_or_default();
        let tests: Vec<_> = clm_coverage::tests_in(path, &text)
            .into_iter()
            .filter(|t| !t.group)
            .collect();
        let held = clm_coverage::markers_in(path, &text);
        let mine: Vec<&Marker> = own.iter().copied().filter(|m| &m.path == path).collect();
        for m in &mine {
            if !tests.iter().any(|t| (t.first..=t.last).contains(&m.line)) {
                take_out.push(m);
            }
        }
        for t in &tests {
            let span = t.first..=t.last;
            let at: Vec<&Marker> = mine
                .iter()
                .copied()
                .filter(|m| span.contains(&m.line))
                .collect();
            if at.is_empty() {
                continue;
            }
            let test = format!("{}::{}", path.display(), t.name);
            let cites_held = held
                .iter()
                .any(|h| span.contains(&h.line) && graph.requirement(&h.id).is_some());
            if approved_names(approved, &test) {
                delete.push(test);
            } else if let Some(to) = &moved {
                e.recite.push((test, to.clone()));
            } else if cites_held {
                take_out.extend(at);
            } else {
                e.leave.push(test);
            }
        }
    }
    take_out.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    e.take_out = take_out.iter().map(|m| m.to_string()).collect();
    (delete, e)
}

/// Of `files`, each with its owner, those `gate` gives as not `node`'s build's to change
/// (`gate/own-files`): it judges a commit object on `base` holding the tree-ish
/// `extracted.0`, extracted at `extracted.1`, with each of `files` modified, as `clm land`
/// would judge a branch that changes them. The commit is written as objects only, with no
/// ref naming it, so nothing of the clone moves.
fn not_ours_to_change(
    root: &Path,
    base: &str,
    extracted: (&str, &Path),
    files: &[String],
    node: &str,
) -> Result<HashMap<String, String>, Error> {
    if files.is_empty() {
        return Ok(HashMap::new());
    }
    let (treeish, tree) = extracted;
    let scratch = tempfile::TempDir::new().map_err(|e| Error::Refused(format!("temp dir: {e}")))?;
    let index = scratch.path().join("index");
    let with = |env: &[(&str, &std::ffi::OsStr)], args: &[&str], stdin: Option<&[u8]>| {
        let options = clm_git::Options { stdin, env };
        clm_git::run(root, "git", args, options)
            .map(|out| String::from_utf8_lossy(&out).trim().to_string())
    };
    let own_index = [("GIT_INDEX_FILE", index.as_os_str())];
    with(&own_index, &["read-tree", treeish], None)?;
    for file in files {
        let mut text = fs::read(tree.join(file)).unwrap_or_default();
        text.push(b'\n');
        let blob = with(&[], &["hash-object", "-w", "--stdin"], Some(&text))?;
        let entry = format!("100644,{blob},{file}");
        with(
            &own_index,
            &["update-index", "--add", "--cacheinfo", &entry],
            None,
        )?;
    }
    let probe = with(&own_index, &["write-tree"], None)?;
    let who: &std::ffi::OsStr = "clm run".as_ref();
    let mail: &std::ffi::OsStr = "clm@localhost".as_ref();
    let identity = [
        ("GIT_AUTHOR_NAME", who),
        ("GIT_AUTHOR_EMAIL", mail),
        ("GIT_COMMITTER_NAME", who),
        ("GIT_COMMITTER_EMAIL", mail),
    ];
    let commit = with(
        &identity,
        &[
            "commit-tree",
            &probe,
            "-p",
            base,
            "-m",
            "the files clm run asks about",
        ],
        None,
    )?;
    let found = clm_gate::foreign_files(root, base, &commit, node).map_err(Error::Git)?;
    Ok(found
        .into_iter()
        .filter(|f| files.contains(&f.path))
        .map(|f| (f.path, f.owner))
        .collect())
}

/// The lines of every change's `deleted-tests.txt` in the tree at `tree`, live or archived
/// (`changes/<id>/` or `changes/archive/<id>/`), the lists `clm land` reads as approved
/// (`land/deleted-tests-approved`).
fn approved_lists(tree: &Path) -> Vec<ListLine> {
    let changes = tree.join("changes");
    let ids = |dir: &Path| -> Vec<PathBuf> {
        fs::read_dir(dir)
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default()
    };
    let mut dirs = ids(&changes);
    dirs.extend(ids(&changes.join("archive")));
    dirs.sort();
    dirs.iter()
        .filter(|d| d.file_name().is_some_and(|n| n != "archive"))
        .filter_map(|d| fs::read_to_string(d.join("deleted-tests.txt")).ok())
        .flat_map(|text| list_lines(&text))
        .collect()
}

/// A test that holds some of a requirement's markers: its `<file>::<name>`, its file and its
/// lines.
struct Listed {
    name: String,
    path: PathBuf,
    lines: std::ops::RangeInclusive<usize>,
}

impl Listed {
    fn holds(&self, m: &Marker) -> bool {
        m.path == self.path && self.lines.contains(&m.line)
    }
}

/// The tests in the tree at `tree` that hold one of `markers` and that an `approved` line
/// names, sorted by file and line.
fn listed_tests(tree: &Path, markers: &[&Marker], approved: &[ListLine]) -> Vec<Listed> {
    let mut paths: Vec<&PathBuf> = markers.iter().map(|m| &m.path).collect();
    paths.sort();
    paths.dedup();
    let mut listed = Vec::new();
    for path in paths {
        let text = fs::read_to_string(tree.join(path)).unwrap_or_default();
        for t in clm_coverage::tests_in(path, &text)
            .into_iter()
            .filter(|t| !t.group)
        {
            let test = Listed {
                name: format!("{}::{}", path.display(), t.name),
                path: path.clone(),
                lines: t.first..=t.last,
            };
            if markers.iter().any(|m| test.holds(m)) && approved_names(approved, &test.name) {
                listed.push(test);
            }
        }
    }
    listed
}

/// Whether an `approved` line names `test`, `<file>::<name>`, compared by file and
/// `bare_name` as `clm land` compares them.
fn approved_names(approved: &[ListLine], test: &str) -> bool {
    let (file, name) = test.split_once("::").unwrap_or(("", test));
    let bare = clm_coverage::bare_name(name);
    approved
        .iter()
        .any(|l| l.file == file && clm_coverage::bare_name(&l.name) == bare)
}

/// The tests in the tree at `tree` that hold `markers` and cite removed `id`, each
/// `<file>::<name>`.
fn obsolete_tests_in(
    graph: &Graph,
    tree: &Path,
    id: &str,
    markers: &[&clm_coverage::Marker],
) -> Vec<String> {
    let mut paths: Vec<&PathBuf> = markers.iter().map(|m| &m.path).collect();
    paths.sort();
    paths.dedup();
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .filter_map(|path| Some((path.clone(), fs::read_to_string(tree.join(path)).ok()?)))
        .collect();
    clm_coverage::obsolete_tests(graph, &files)
        .into_iter()
        .filter(|t| t.ids.iter().any(|i| i == id))
        .map(|t| t.test)
        .collect()
}

/// The history of a node's `SPEC.md` up to a revision: its commits, newest first, and each
/// revision's text once read, so a node with several `behind` requirements walks it once.
struct History<'a> {
    dir: &'a Path,
    spec: &'a str,
    commits: Vec<String>,
    texts: HashMap<String, Option<String>>,
}

impl<'a> History<'a> {
    fn new(dir: &'a Path, rev: &str, spec: &'a str) -> Self {
        let commits = git(dir, &["log", "--format=%H", rev, "--", spec])
            .map(|log| log.lines().map(String::from).collect())
            .unwrap_or_default();
        History {
            dir,
            spec,
            commits,
            texts: HashMap::new(),
        }
    }

    /// The text requirement `id` had at version `cited`: the newest commit at which it had
    /// that version. `None` after the walk, or for `none` (a bare marker).
    fn earlier_text(&mut self, id: &str, cited: &str) -> Option<String> {
        if cited == "none" {
            return None;
        }
        for i in 0..self.commits.len() {
            let sha = self.commits[i].clone();
            let text = self
                .texts
                .entry(sha.clone())
                .or_insert_with(|| git(self.dir, &["show", &format!("{sha}:{}", self.spec)]).ok());
            let Some(text) = text else {
                continue;
            };
            if let Some(found) =
                requirement_in(self.spec, text, id).filter(|r| r.version() == cited)
            {
                return Some(found.text.clone());
            }
        }
        None
    }
}

/// Requirement `id` as the text of `spec` (a `SPEC.md`) defines it, parsed on its own.
fn requirement_in(spec: &str, text: &str, id: &str) -> Option<clm_model::Requirement> {
    let files: Files = [(PathBuf::from(spec), text.as_bytes().to_vec())].into();
    let (graph, _) = clm_model::load_files(&files);
    graph.requirement(id).cloned()
}

/// A unified diff from `old` to `new`, hunks only, via `git diff --no-index`.
fn diff_text(old: &str, new: &str) -> Result<String, Error> {
    let tmp = tempfile::TempDir::new().map_err(|e| Error::Refused(format!("temp dir: {e}")))?;
    let (a, b) = (tmp.path().join("cited"), tmp.path().join("current"));
    let io = |e: std::io::Error| Error::Refused(format!("diff: {e}"));
    fs::write(&a, format!("{}\n", old.trim_end())).map_err(io)?;
    fs::write(&b, format!("{}\n", new.trim_end())).map_err(io)?;
    // `git diff --no-index` exits 1 when the files differ, and `clm-git` keeps no output
    // from a failure, so the diff goes to a file.
    let diff = tmp.path().join("diff");
    let output = format!("--output={}", diff.display());
    let (a, b) = (a.to_string_lossy(), b.to_string_lossy());
    let args = ["diff", "--no-index", "--no-color", "-U2", &output, &a, &b];
    match git(tmp.path(), &args) {
        Ok(_) => {}
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => {}
        Err(e) => return Err(Error::Refused(e.to_string())),
    }
    let text = fs::read_to_string(&diff).map_err(io)?;
    Ok(text
        .lines()
        .skip_while(|l| !l.starts_with("@@"))
        .map(|l| format!("{l}\n"))
        .collect())
}

/// Superpowers at the pin, from `$CLM_SUPERPOWERS_DIR` or the cache, fetching it once; a
/// relative directory the environment names is read from `cwd`.
fn superpowers_dir(cwd: &Path, env: Env) -> Result<PathBuf, Error> {
    if let Some(dir) = env("CLM_SUPERPOWERS_DIR").filter(|d| !d.is_empty()) {
        return Ok(cwd.join(dir));
    }
    let cache = match (env("XDG_CACHE_HOME"), env("HOME")) {
        (Some(xdg), _) if !xdg.is_empty() => cwd.join(xdg),
        (_, Some(home)) => cwd.join(home).join(".cache"),
        _ => {
            return Err(Error::Refused(
                "neither XDG_CACHE_HOME nor HOME is set".into(),
            ));
        }
    }
    .join("clm/superpowers");
    let dir = cache.join(SUPERPOWERS_COMMIT);
    if dir.is_dir() {
        return Ok(dir);
    }
    let io = |e: std::io::Error| Error::Refused(format!("{}: {e}", cache.display()));
    fs::create_dir_all(&cache).map_err(io)?;
    // Fetch into a temporary directory and rename it into place, so an interrupted fetch
    // never leaves a partial checkout at the pin.
    let tmp = tempfile::TempDir::new_in(&cache).map_err(io)?;
    let repo = tmp.path().join("superpowers");
    fs::create_dir(&repo).map_err(io)?;
    for args in [
        &["init", "-q"][..],
        &[
            "fetch",
            "-q",
            "--depth",
            "1",
            SUPERPOWERS_URL,
            SUPERPOWERS_COMMIT,
        ],
        &["checkout", "-q", "FETCH_HEAD"],
    ] {
        git_isolated(&repo, args)?;
    }
    if fs::rename(&repo, &dir).is_err() && !dir.is_dir() {
        return Err(Error::Refused(format!(
            "can't move superpowers to {}",
            dir.display()
        )));
    }
    Ok(dir)
}

/// Keeps `.clm/` out of git through `info/exclude`, which every worktree shares.
fn exclude_workdir(root: &Path) -> Result<(), Error> {
    let common = git(root, &["rev-parse", "--git-common-dir"])?;
    let info = root.join(&common).join("info");
    let exclude = info.join("exclude");
    let line = format!("{WORKDIR}/");
    let current = fs::read_to_string(&exclude).unwrap_or_default();
    if current.lines().any(|l| l == line) {
        return Ok(());
    }
    let io = |e: std::io::Error| Error::Refused(format!("{}: {e}", exclude.display()));
    fs::create_dir_all(&info).map_err(io)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude)
        .map_err(io)?;
    let sep = if current.is_empty() || current.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    writeln!(f, "{sep}{line}").map_err(io)
}

/// Fetches `origin`, pruning, so a claim whose branch was deleted is gone, as `clm claim`
/// sees it.
fn fetch(root: &Path) -> Result<(), Error> {
    // Tried again after a failure, so a fetch that lost a race with another build of the
    // clone over a remote-tracking ref goes on (`run-fetch-retries-race`).
    clm_git::fetch_retrying(root, &["fetch", "--prune", "--quiet", REMOTE])
        .map_err(|e| Error::Git(e.to_string()))
}

/// The graph at `dir`, or the violations that keep it from loading.
fn load(dir: &Path) -> Result<Graph, Error> {
    let (graph, violations) = clm_model::load(dir);
    if violations.is_empty() {
        Ok(graph)
    } else {
        Err(Error::Spec(violations))
    }
}

/// Every worktree, the main checkout first. Paths are canonical where they exist, so they
/// compare with a caller's.
fn worktrees(root: &Path) -> Result<Vec<clm_git::Worktree>, Error> {
    let mut list = clm_git::worktrees(root)?;
    for wt in &mut list {
        if let Ok(path) = wt.path.canonicalize() {
            wt.path = path;
        }
    }
    Ok(list)
}

/// No uncommitted changes and no untracked files that aren't ignored.
fn clean(dir: &Path) -> Result<bool, Error> {
    Ok(git(dir, &["status", "--porcelain"])?.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn diff_text_keeps_the_hunks_of_differing_text() {
        let diff = super::diff_text("a\nold\n", "a\nnew\n").unwrap();
        assert_eq!(diff, "@@ -1,2 +1,2 @@\n a\n-old\n+new\n");
    }

    use super::*;

    /// A component's `SPEC.md` with `extra` frontmatter and requirement `s`.
    fn spec(extra: &str) -> String {
        format!(
            "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: []\n{extra}---\n\n\
             ## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n\
             ### Requirement: s\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        )
    }

    /// The tree `files` make, its graph, and the prompt for `c` written from a pending list
    /// placing obsolete `id` on `c` with `markers`, each `(<file>, <line>)`.
    fn prompt_for_c(files: &[(&str, &str)], id: &str, markers: &[(&str, usize)]) -> String {
        let dir = tempfile::TempDir::new().unwrap();
        for (path, text) in files {
            let at = dir.path().join(path);
            fs::create_dir_all(at.parent().unwrap()).unwrap();
            fs::write(at, text).unwrap();
        }
        let graph = load(dir.path()).unwrap();
        let scanned: Vec<Marker> = files
            .iter()
            .flat_map(|(path, text)| clm_coverage::markers_in(Path::new(path), text))
            .collect();
        let markers: Vec<Marker> = markers
            .iter()
            .map(|(path, line)| Marker {
                id: id.into(),
                version: None,
                path: PathBuf::from(path),
                line: *line,
            })
            .collect();
        let pending = Pending {
            id: id.into(),
            node: "c".into(),
            version: None,
            state: ReqState::Obsolete,
            cited: Vec::new(),
            markers,
            priority: clm_model::Priority::P5,
            blocked_by: Vec::new(),
            waits_on: Vec::new(),
        };
        let job = job(
            dir.path(),
            "HEAD",
            ("HEAD", dir.path()),
            &graph,
            &scanned,
            "c",
            &[pending],
        )
        .unwrap();
        prompt(&job)
    }

    #[test]
    fn scenario_another_component_s_removed_capability() {
        // req:orchestrator.run/pending-prompt@15d82893
        let cite = format!("// {}:capability/gone", "req");
        let test = format!("#[test]\nfn old_cap() {{\n    {cite}\n}}\n");
        let p = prompt_for_c(
            &[
                ("ARCHITECTURE.md", "# Arch\n"),
                ("CAPABILITIES.md", "# Capabilities\n"),
                ("specs/SPEC.md", &spec("wiring: []\n")),
                (
                    "specs/capability/SPEC.md",
                    spec("paths: [\"crates/clm/tests/capability/**\"]\n")
                        .split("\n### Requirement")
                        .next()
                        .unwrap(),
                ),
                ("specs/c/SPEC.md", &spec("paths: [\"crates/c/**\"]\n")),
                (
                    "changes/20260101-x/change.md",
                    "---\nstatus: applied\nremoves_capabilities: [gone]\n---\n",
                ),
                (
                    "changes/20260101-x/deleted-tests.txt",
                    "DELETE crates/c/t.rs::old_cap\n",
                ),
                ("crates/c/t.rs", &test),
            ],
            "capability/gone",
            &[("crates/c/t.rs", 3)],
        );
        assert!(p.contains("- `capability/gone` obsolete"), "{p}");
        assert!(
            p.contains("it was not `c`'s: `20260101-x` removed it"),
            "{p}"
        );
        assert!(
            p.contains("(`land/deleted-tests-approved`): `crates/c/t.rs::old_cap`."),
            "{p}"
        );
        assert!(!p.contains("`deferred` item naming"), "{p}");
        assert!(!p.contains("Remove the code they mark"), "{p}");
    }

    #[test]
    fn scenario_another_component_s_requirement_whose_own_files_don_t_cite_it() {
        // req:orchestrator.run/pending-prompt@15d82893
        let gone = format!("// {}:b/gone\n", "req");
        let lib = format!("//\n//\n//\n{gone}pub fn f() {{}}\n");
        let files = |at: &str, b_cites: &str| {
            vec![
                ("ARCHITECTURE.md".to_string(), "# Arch\n".to_string()),
                ("specs/SPEC.md".into(), spec("")),
                ("specs/b/SPEC.md".into(), spec("paths: [\"crates/b/**\"]\n")),
                ("specs/c/SPEC.md".into(), spec("paths: [\"crates/c/**\"]\n")),
                (
                    "changes/20260101-x/change.md".into(),
                    "---\nstatus: applied\n---\n".into(),
                ),
                (
                    "changes/20260101-x/deltas/b.md".into(),
                    "---\ncomponent: b\n---\n## REMOVED Requirements\n\n\
                     ### Requirement: gone <!-- base: 0000000000000000 -->\n"
                        .into(),
                ),
                ("crates/c/lib.rs".into(), lib.clone()),
                (at.to_string(), b_cites.to_string()),
            ]
        };
        let run = |at: &str, b_cites: &str, markers: &[(&str, usize)]| {
            let owned = files(at, b_cites);
            let refs: Vec<(&str, &str)> = owned
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            prompt_for_c(&refs, "b/gone", markers)
        };
        let p = run("crates/b/t.rs", "", &[("crates/c/lib.rs", 4)]);
        assert!(
            p.contains("Take out its markers at crates/c/lib.rs:4,"),
            "{p}"
        );
        assert!(
            p.contains(
                "The code `b/gone` needed is `b`'s to remove: add a `deferred` item naming `b` \
                 and `b/gone` unless the backlog holds one."
            ),
            "{p}"
        );
        assert!(!p.contains("Remove the code they mark"), "{p}");
        // A test of `b`'s own that still cites it leaves the code to `b`'s own build.
        let in_b = format!("#[test]\nfn t() {{\n    {gone}}}\n");
        let p = run("crates/b/t.rs", &in_b, &[("crates/c/lib.rs", 4)]);
        assert!(!p.contains("`deferred` item naming"), "{p}");
        // So does one in a file no component owns, which `pending` counts as `b`'s.
        let p = run("old/t.rs", &in_b, &[("crates/c/lib.rs", 4)]);
        assert!(!p.contains("`deferred` item naming"), "{p}");
        // A test in `c`'s file is not `b`'s.
        let p = run("crates/c/t.rs", &in_b, &[("crates/c/lib.rs", 4)]);
        assert!(p.contains("`deferred` item naming"), "{p}");
    }

    #[test]
    fn a_wait_on_its_own_id_names_the_other_lines_components_sorted() {
        // req:orchestrator.run/pending-prompt@15d82893
        let line = |node: &str, waits_on: &[&str]| Pending {
            id: "b/gone".into(),
            node: node.into(),
            version: None,
            state: ReqState::Obsolete,
            cited: Vec::new(),
            markers: Vec::new(),
            priority: clm_model::Priority::P5,
            blocked_by: Vec::new(),
            waits_on: waits_on.iter().map(|s| s.to_string()).collect(),
        };
        let lines = vec![
            line("e", &[]),
            line("b", &["a/need", "b/gone"]),
            line("c", &[]),
            line("d", &[]),
        ];
        let b = blockers(&lines, &lines[1]);
        assert_eq!(b.by, vec!["a/need".to_string()]);
        assert_eq!(b.cited_in, vec!["c", "d", "e"]);
    }

    #[test]
    fn a_test_that_also_cites_a_held_requirement_loses_only_its_marker() {
        // req:orchestrator.run/pending-prompt@15d82893
        let test = format!(
            "#[test]\nfn both() {{\n    // {r}:g/gone\n    // {r}:c/s\n}}\n",
            r = "req"
        );
        let p = prompt_for_c(
            &[
                ("ARCHITECTURE.md", "# Arch\n"),
                ("specs/SPEC.md", &spec("")),
                ("specs/c/SPEC.md", &spec("paths: [\"crates/c/**\"]\n")),
                (
                    "changes/20260101-x/change.md",
                    "---\nstatus: applied\n---\n",
                ),
                (
                    "changes/20260101-x/deltas/g.md",
                    "---\ncomponent: g\n---\n## REMOVED Requirements\n\n\
                     ### Requirement: gone <!-- base: 0000000000000000 -->\n",
                ),
                ("crates/c/t.rs", &test),
            ],
            "g/gone",
            &[("crates/c/t.rs", 3)],
        );
        assert!(
            p.contains("Take out its markers at crates/c/t.rs:3,"),
            "{p}"
        );
        assert!(!p.contains("Leave each other test"), "{p}");
    }
}
