//! Run (see specs/orchestrator/run/SPEC.md): `clm run <node>` claims a node, or picks up
//! its unfinished build, and writes the prompt for a session that builds its pending
//! requirements and lands them with `clm land` to `.clm/prompts/<node>.md`, printing a short
//! launcher that names it (`builds-in-subagents`, `build-prompt-file`,
//! `unattended-superpowers`, `land-locally`). It starts no session and makes no worktree:
//! `implement` gives the launcher to a subagent in its own Claude Code worktree.
//! `clm superpowers <skill>` resolves the pinned copy builds follow.

use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use clm_check::{Pending, State as ReqState};
use clm_git::{git, git_isolated};
use clm_model::{Files, Graph};
pub use clm_orchestrator::Error;
use clm_orchestrator::{
    PlaceKind, Placed, REMOTE, claim_local, free_place, place_slot, take_place,
};

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

/// The review (`advisory-code-review`): `/code-review` at this level, plus this subagent.
/// Why a build never waits on a background command by matching processes by name
/// (`background-work-stopped`).
const BACKGROUND_WAIT: &str = ", never through a loop, watch or monitor that matches \
                               processes by name (such as `pgrep -f`): such a match also finds \
                               other builds' processes and outlives the command it waits for.";

/// A land the session started is waited for to its end (`background-work-stopped`).
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
    /// at another version or without one (`stale-markers-keep-behind`).
    pub markers: Vec<String>,
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
    /// The blocked requirements, each with what blocks it: `backlog/<slug>` items, then the
    /// pending requirements it needs.
    pub blocked: Vec<(String, Vec<String>)>,
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
    // (`background-work-stopped`, `prompt-check-command`).
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
    let capability_tests = if job.capability_tests {
        " This component builds the capability tests: each capability under that heading is one \
         its tests prove at the project's public boundary, whether or not a rule serves it; \
         the capability component has no rules to serve them."
    } else {
        ""
    };
    // The marker's name is built so it never stands whole in this file, or `clm affected`
    // would take the file for a reader (`prompt-doc-readers`).
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
    // Only a repository with a `[land] check` has a check to record (`prompt-recorded-check`).
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
         `clm gate --base origin/main` prints `code`, and push with \
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
         code and tests its markers mark (`no-dead-code`). Re-citing a marker of your own requirement, wherever its file sits, is allowed (`architecture/path-ownership`). \
         Your build changes only your own component's files: those your component's `tests` match, \
         wherever they sit, and those its `paths` match that no other component's `tests` match \
         (`path-ownership`, `components-own-their-tests`), apart from the two exceptions \
         `capability-tests-follow-rule` and `template-migration-fix` state below and the \
         re-citing above in a file no component's `tests` match. Put each new test in a file your \
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
         requirement, and which names the entry, so a spec change edits it. When a requirement \
         you build changes the text of a file `clm init` writes, and a requirement of \
         `upgrade`, in the specs as applied, states the migration that carries that change and \
         needs the requirement you build, write that migration in your own branch, under \
         `upgrade`'s `paths`, exactly as that requirement states it (its ID, `old`, `new`, \
         `done` and note), since the upgrade corpus test fails a land of either half alone \
         (`path-ownership`, `template-change-ships-its-migration`). Add no test citing that \
         `upgrade` requirement, which `upgrade`'s own build adds, but bring any existing test \
         of `upgrade`'s that the new migration makes fail (such as one expecting an earlier \
         migration's rewrite to be the last) to what `upgrade`'s requirements as applied \
         state, and name the migration under `## Fixes in other components` in your pull \
         request body.{waits} Keep the \
         code lean and true to the component's Contract and Design, folding in the cleanups the spec asks for. When the list \
         is long, you may open your pull request and land once the tree is green with at most \
         {BUDGET} requirements built, leaving the rest pending for the next run; list what you \
         left under `## Left pending` in the pull request body. A landed branch is progress whatever \
         it leaves.\n\n\
         Once the requirements you built pass their tests, and before the review, spot-check \
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
         the pull request body.\n\n\
         Your build's slot is {slot}. The project's tests run beside other builds' tests on this \
         host (`tests-run-concurrently`), so every test command you run by hand, a \
         spot-check's runs of a mutant included, runs with `CLM_SLOT={slot}` set before the \
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
         commit whose trailer block holds `Resolves: backlog/<slug>`. A `question`, `human-task` \
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
            p.push_str(&format!(
                "  A live change removed it; remove the code and tests marked at {}.\n",
                item.markers.join(", ")
            ));
        } else if item.state == "behind" && !item.markers.is_empty() {
            p.push_str(&format!(
                "  Re-cite each marker that cites it at another version or without one, wherever \
                 its file sits, another component's `paths` included, after checking that its \
                 test proves the current text (`path-ownership`): {}.\n",
                item.markers.join(", ")
            ));
        }
    }
    if !job.blocked.is_empty() {
        p.push_str("\nBlocked, left out of the list above; build nothing that relies on them:\n");
        for (id, items) in &job.blocked {
            p.push_str(&format!(
                "- `{id}`, blocked by {}\n",
                listing_owned(items, "and", true)
            ));
        }
    }
    if let Some(bundle) = &job.bundle {
        p.push_str(&format!("\nThe context bundle for `{node}`:\n\n{bundle}"));
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

/// The path of `skill`'s `SKILL.md` in the pinned superpowers copy builds follow.
pub fn superpowers_skill(skill: &str) -> Result<String, Error> {
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
    let path = superpowers_dir()?
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
/// The settings' `[env]` variables are set for the command, over the caller's own.
pub fn setup(root: &Path) -> Result<String, Error> {
    let settings = clm_config::load(Some(root), &clm_config::process_env)
        .map_err(|e| Error::Refused(e.to_string()))?;
    let Some(build) = settings.build.value else {
        return Ok("no [build] setup".to_string());
    };
    let status = std::process::Command::new("sh")
        .args(["-c", &build.setup])
        .current_dir(root)
        .envs(settings.env.iter().map(|(n, s)| (n, &s.value)))
        .stdin(std::process::Stdio::null())
        .status()
        .map_err(|e| Error::Refused(format!("setup failed: can't run `{}`: {e}", build.setup)))?;
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
pub fn run(
    root: &Path,
    node: &str,
    expiry_hours: u64,
    review_model: Option<&str>,
) -> Result<String, RunError> {
    let root = &root
        .canonicalize()
        .map_err(|e| Error::Refused(format!("{}: {e}", root.display())))?;
    if node.is_empty() || node.contains('/') {
        return Err(Error::Refused(format!("`{node}` isn't a component ID")).into());
    }
    // Before anything is claimed, so settings that can't be loaded leave nothing behind.
    let settings = clm_config::load(Some(root), &clm_config::process_env)
        .map_err(|e| Error::Refused(e.to_string()))?;
    let check = settings.land.value.map(|land| land.check);
    let setup = settings.build.value.map(|build| build.setup);
    // Before anything is claimed, so a missing skill leaves nothing behind.
    let skills = SKILLS
        .iter()
        .map(|s| {
            Ok((
                s.to_string(),
                superpowers_skill(s.trim_start_matches("superpowers:"))?,
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
        Settled { check, setup },
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
    let tmp = clm_git::extract_tree(root, merged.as_deref().unwrap_or(&main))?;
    let tree = tmp.path();
    let (graph, pending) = pending_in(tree, node)?;
    let mut job = job(root, &main, tree, &graph, node, &pending, resumed)?;
    job.read_from_main = merged.is_none();
    job.skills = skills;
    job.check = settled.check;
    job.setup = settled.setup;
    job.review_model = review_model.map(str::to_string);
    job.slot = place_slot(root, branch)?;
    // A write that fails leaves the claim, which the next `clm run` resumes.
    let file = prompts.join(format!("{node}.md"));
    fs::write(&file, prompt(&job)).map_err(|e| {
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

/// `clm run --finish <node>`, once a build's subagent has returned (`build-places`): frees
/// the place `<node>` and, when `impl/<node>` has landed, removes each clean worktree under
/// the main worktree's `.claude/worktrees/` that has it checked out. Reads no queue state,
/// so it works for any node. Returns what it did, one line each.
pub fn finish(root: &Path, node: &str, expiry_hours: u64) -> Result<String, Error> {
    let root = &root
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
        let here = env::current_dir().ok().and_then(|d| d.canonicalize().ok());
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
            if clean(&wt.path)? {
                done.push(match clm_git::remove_worktree(root, &wt.path).map_err(|e| e.to_string()) {
                    Ok(()) => format!("removed {}", wt.path.display()),
                    Err(why) => format!(
                        "left {}: {}; delete the directory by hand, then run git worktree prune",
                        wt.path.display(),
                        why.replace('\n', " ").trim()
                    ),
                });
            } else {
                done.push(format!("kept {}: uncommitted changes", wt.path.display()));
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
/// hold nothing `origin/main` lacks.
fn tidy(root: &Path, node: &str, branch: &str) -> Result<(), Error> {
    let local = format!("refs/heads/{branch}");
    let holder = checked_out(root, node, branch)?;
    let landed = git(root, &["rev-parse", "--verify", "-q", &local]).is_ok()
        && clm_git::is_ancestor(root, &local, &format!("{REMOTE}/main"))?;
    if landed {
        if let Some(h) = &holder {
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

/// A worktree that has a branch checked out, and whether it's the main checkout.
struct Holder {
    path: PathBuf,
    main: bool,
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
        }))
}

/// The graph of the tree at `dir` and `node`'s pending requirements, most urgent first.
fn pending_in(dir: &Path, node: &str) -> Result<(Graph, Vec<Pending>), Error> {
    let graph = load(dir)?;
    if !graph.nodes.contains_key(node) {
        return Err(Error::Refused(format!("no component `{node}`")));
    }
    let markers = clm_check::scan(dir).req;
    let mut pending: Vec<Pending> = clm_check::pending(&graph, &markers)
        .into_iter()
        .filter(|p| p.node == node)
        .collect();
    pending.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
    Ok((graph, pending))
}

/// What blocks `p`: the `backlog/<slug>` items whose `blocks` name it, then the other pending
/// requirements it waits on (`requirement-needs`, `moved-markers-wait`, `serving-rules-order-capabilities`).
fn blockers(p: &Pending) -> Vec<String> {
    p.blocked_by
        .iter()
        .map(|s| format!("backlog/{s}"))
        .chain(p.waits_on.iter().cloned())
        .collect()
}

/// The job for `node`, whose tree at `rev` in the repository at `root` is extracted at
/// `tree`.
fn job(
    root: &Path,
    rev: &str,
    tree: &Path,
    graph: &Graph,
    node: &str,
    pending: &[Pending],
    resumed: bool,
) -> Result<Job, Error> {
    let spec = graph.nodes[node].path.to_string_lossy().into_owned();
    let mut history = History::new(root, rev, &spec);
    let mut items = Vec::new();
    let mut blocked = Vec::new();
    for p in pending {
        let by = blockers(p);
        if !by.is_empty() {
            blocked.push((p.id.clone(), by));
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
        items.push(Item {
            id: p.id.clone(),
            version: p.version.clone(),
            state: p.state.to_string(),
            diffs,
            markers: p
                .markers
                .iter()
                .filter(|m| p.state != ReqState::Behind || m.version != p.version)
                .map(ToString::to_string)
                .collect(),
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
        resumed,
        check: None,
        review_model: None,
        capability_tests: graph.capability_tests_node() == node,
        read_from_main: false,
        slot: 0,
        setup: None,
    })
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

/// Superpowers at the pin, from `$CLM_SUPERPOWERS_DIR` or the cache, fetching it once.
fn superpowers_dir() -> Result<PathBuf, Error> {
    if let Some(dir) = env::var_os("CLM_SUPERPOWERS_DIR").filter(|d| !d.is_empty()) {
        return Ok(dir.into());
    }
    let cache = match (env::var_os("XDG_CACHE_HOME"), env::var_os("HOME")) {
        (Some(xdg), _) if !xdg.is_empty() => PathBuf::from(xdg),
        (_, Some(home)) => Path::new(&home).join(".cache"),
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
}
