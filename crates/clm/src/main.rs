use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod closed_stdout;
mod upgrade_command;
mod upgrade_gate;

#[derive(Parser)]
#[command(
    name = "clm",
    about = "Spec-driven development for teams",
    version = env!("CLM_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum ViewName {
    Spec,
    Flight,
    Blockers,
}

impl From<ViewName> for clm_view::Name {
    fn from(n: ViewName) -> Self {
        match n {
            ViewName::Spec => clm_view::Name::Spec,
            ViewName::Flight => clm_view::Name::Flight,
            ViewName::Blockers => clm_view::Name::Blockers,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ViewFormat {
    Text,
    Ansi,
    Svg,
    Json,
}

impl From<ViewFormat> for clm_view::Format {
    fn from(f: ViewFormat) -> Self {
        use clm_view::Format;
        match f {
            ViewFormat::Text => Format::Text,
            ViewFormat::Ansi => Format::Ansi,
            ViewFormat::Svg => Format::Svg,
            ViewFormat::Json => Format::Json,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum CapabilitiesFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Run every deterministic check; exit 1 on any violation.
    Check {
        /// Run no dead-code or unused-dependency detector, for a caller that knows no code
        /// changed.
        #[arg(long)]
        no_detectors: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List every requirement as current, behind or unbuilt (with citing markers), then the
    /// obsolete ones.
    Coverage {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List the requirements whose code hasn't caught up with their spec: behind, unbuilt or
    /// obsolete, with what blocks them.
    Pending {
        /// Component ID; every component when omitted.
        #[arg(value_name = "COMPONENT")]
        node: Option<String>,
        /// List this commit's pending requirements, read from git's objects, instead of the
        /// working tree's.
        #[arg(long)]
        at: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Report each component's spec and code size in tokens against its budget (advisory).
    Size {
        /// Component ID; every component when omitted.
        #[arg(value_name = "COMPONENT")]
        node: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Apply a change's deltas to specs/, all or nothing.
    Apply {
        /// Change ID (directory name under changes/).
        change: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Carry out the renames a proposed change records: its deltas, applied, with every
    /// reference re-pointed, and the change archived as done.
    Rename {
        /// Change ID (directory name under changes/).
        change: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Move a done or abandoned change to changes/archive/.
    Archive {
        /// Change ID.
        change: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List backlog items: ideas, deferred work, questions and bugs.
    Backlog {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print a summary of the repository, then its components, changes and claims; never fetches.
    Status {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print a view of the repository: the architecture, the work in flight or what holds
    /// it up; never fetches.
    View {
        /// The view.
        name: ViewName,
        /// The form: a chart, the chart with ANSI colour, an SVG document or the model.
        #[arg(long, value_enum, default_value_t = ViewFormat::Text)]
        format: ViewFormat,
        /// Columns the chart fits in, at least 1.
        #[arg(long, default_value_t = clm_view::DEFAULT_WIDTH as u16, value_parser = clap::value_parser!(u16).range(1..))]
        width: u16,
        /// Describe this commit, read from git's objects, instead of the working tree.
        #[arg(long)]
        at: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print the context bundle an agent needs for a component, its ancestors' specs among it.
    Context {
        /// Component ID.
        #[arg(value_name = "COMPONENT")]
        node: String,
        /// Print the bundle's content hash instead of the bundle.
        #[arg(long)]
        hash: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List the migrations this project still needs, or take them: bring a project set up by
    /// an older `clm` up to date.
    Upgrade {
        /// Run the pending apply migrations, rewrite unedited files a note migration
        /// replaces, and stamp `upgraded`.
        #[arg(long, conflicts_with_all = ["note", "decline", "lock"])]
        apply: bool,
        /// Print a migration's note.
        #[arg(long, value_name = "ID", conflicts_with_all = ["apply", "decline", "lock"])]
        note: Option<String>,
        /// Decline a pending note migration.
        #[arg(long, value_name = "ID", conflicts_with_all = ["apply", "note", "lock"])]
        decline: Option<String>,
        /// Lock the migrations a release ships (the release runs this).
        #[arg(long, value_name = "VERSION", conflicts_with_all = ["apply", "note", "decline"])]
        lock: Option<String>,
    },
    /// Print a component's element hashes, for recording as delta bases.
    Hashes {
        /// Component ID.
        #[arg(value_name = "COMPONENT")]
        node: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Decide whether HEAD may merge into a base ref; exit 1 to block.
    Gate {
        /// Base ref the PR merges into.
        #[arg(long)]
        base: String,
        /// The ledger holds an approval of this diff that the policy accepts. `clm land` sets it
        /// from the ledger; passing it by hand only skips `gate/approval`.
        #[arg(long)]
        approved: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List the components ready to build from origin/main, most urgent first.
    Next {
        /// Hours without a commit after which a claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Look again until a look prints a component, then print it.
        #[arg(long)]
        wait: bool,
        /// Minutes between looks with `--wait`.
        #[arg(long, requires = "wait", default_value = "5")]
        interval: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Claim a component by pushing its impl/<component> branch, or a backlog item by pushing
    /// propose/<slug>.
    Claim {
        /// A component ID, as `clm next` prints it, or `backlog/<slug>`; with `--fix`, any
        /// component ID.
        #[arg(value_name = "COMPONENT")]
        node: String,
        /// Claim the component for a code-only fix of this bug or deferred item (`backlog/<slug>`), whatever
        /// its state.
        #[arg(long, value_name = "backlog/<slug>")]
        fix: Option<String>,
        /// Hours without a commit after which a claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Claim a component, or pick up its unfinished build, write the prompt for a session that
    /// builds and lands its pending requirements to `.clm/prompts/<component>.md`, and print a
    /// launcher naming that file. With `--finish`, free the component's build place instead and,
    /// once its branch has landed, remove its clean worktrees under `.claude/worktrees/`.
    Run {
        /// A component ID, as `clm next` prints it.
        #[arg(value_name = "COMPONENT")]
        node: String,
        /// Finish the component's build, whose subagent has returned, instead of starting one.
        #[arg(long)]
        finish: bool,
        /// Hours without a commit after which a claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// The alias of the model the build's final review runs on, passed into the prompt
        /// unchecked (default: the build session's own model).
        #[arg(long, value_name = "MODEL", conflicts_with = "finish")]
        review_model: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List the changes that hold or propose backlog items, and the live backlog claims.
    Proposals {
        /// Print only lines whose components include this one (repeatable).
        #[arg(long = "component", value_name = "COMPONENT")]
        nodes: Vec<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Group the untaken backlog items into units of design work, beside the spec work on
    /// origin and the items waiting on the human, most urgent first.
    Triage {
        /// Hours without a commit after which a backlog claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Match the host's open issues and pull requests to the backlog: for each issue, whether
    /// an item takes it in, a landed change or fix resolved it, or it is new.
    Intake {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Claim backlog items and write the prompt for a session that designs one change from
    /// them unattended, printing a launcher that names it, holding one of the clone's build
    /// places.
    Draft {
        /// The items' slugs, as a `clm triage` group line lists them; with `--fold`, the
        /// change IDs to fold.
        #[arg(required_unless_present = "finish", conflicts_with = "finish")]
        slugs: Vec<String>,
        /// Finish the draft of this change, whose subagent has returned, instead of starting
        /// one.
        #[arg(long, value_name = "CHANGE")]
        finish: Option<String>,
        /// Write the prompt for one session that folds the changes needing review named as arguments
        /// into one spec branch with one pull request, instead of drafting.
        #[arg(long, conflicts_with = "finish")]
        fold: bool,
        /// A file of what the human said in the triage session, put into the draft's
        /// prompt; a relative path is read against the current directory.
        #[arg(long, value_name = "FILE", conflicts_with_all = ["finish", "fold"])]
        answers: Option<PathBuf>,
        /// Hours without a commit after which a backlog claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Build what `clm next` lists unattended: prepare each component with `clm run`, run
    /// `claude -p` headless in a worktree of its own, and finish it with `clm run --finish`
    /// when it exits. Runs on Linux and macOS.
    Runner {
        /// Passed to every `claude` session unchecked (required).
        #[arg(long)]
        permission_mode: Option<String>,
        /// Minutes to wait between looks for work when nothing finishes (a positive number).
        #[arg(long, default_value = "5", value_parser = interval_minutes)]
        interval: std::time::Duration,
        /// Exit once nothing more can be started and every session has exited.
        #[arg(long)]
        once: bool,
        /// Hours without a commit after which a claim lapses.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Remove merged work, report work that isn't safe on origin, and say whether the
    /// session can end.
    #[command(name = "wrap-up")]
    WrapUp {
        /// Repository root, or any worktree of it (default: the CLM repository holding the
        /// current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Rebase the branch onto origin/main, regenerate applied specs and run `clm check`;
    /// exit 1 with what is left to resolve.
    Rebase {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Push a snapshot of the spec work on a `spec/*` branch to origin as `autosync/<branch>`.
    Autosync {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print the path of a skill's SKILL.md in the pinned superpowers copy builds follow.
    Superpowers {
        /// The skill's name, such as `brainstorming`.
        skill: String,
    },
    /// Run a Claude Code hook on the JSON event on stdin (used by the plugin).
    Hook {
        #[command(subcommand)]
        hook: Hook,
    },
    /// Report a defect in CLM itself, the report on stdin, as the `defects` setting says:
    /// a backlog item, an issue on clunny/clm, or nothing.
    Defect {
        /// The report's title.
        #[arg(long)]
        title: String,
        /// A CLM component the defect concerns; repeat for several.
        #[arg(long = "component", value_name = "COMPONENT")]
        nodes: Vec<String>,
    },
    /// Read CLM's settings for the CLM repository found from the current directory.
    Config {
        #[command(subcommand)]
        config: Config,
    },
    /// A branch's pull request on the host the `host` setting names.
    Pr {
        #[command(subcommand)]
        page: Page,
    },
    /// File an issue where the host the `host` setting names keeps them and print its URL;
    /// without a host, print the report and exit 1.
    Issue {
        /// The issue's title.
        #[arg(long)]
        title: String,
        /// The file holding the issue's body.
        #[arg(long)]
        body_file: PathBuf,
        /// The repository to file it in, such as `clunny/clm`.
        #[arg(long)]
        repo: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Contributions from people without write access, through the host adapter the `host`
    /// setting names: a fork's pull requests and issues.
    Contribution {
        #[command(subcommand)]
        contribution: Contribution,
    },
    /// Remote branches, through the host adapter the `host` setting names.
    Branch {
        #[command(subcommand)]
        branch: Branch,
    },
    /// The ledger of approvals, checks and lands: print it, audit `main` against it,
    /// or prune your own superseded records.
    Ledger {
        #[command(subcommand)]
        ledger: Ledger,
    },
    /// The work Claude Code sessions spent, recorded in the ledger.
    Effort {
        #[command(subcommand)]
        effort: Effort,
    },
    /// What each capability costs, from the ledger's effort records and git: the list, or one
    /// capability's page.
    #[command(subcommand_negates_reqs = true)]
    Capabilities {
        #[command(subcommand)]
        command: Option<Capabilities>,
        /// The capability's slug (default: list them all; `decide` is the subcommand).
        slug: Option<String>,
        /// Text, or the report (with a slug, the capability's cost) as JSON.
        #[arg(long, value_enum, default_value = "text")]
        format: CapabilitiesFormat,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// What each architectural constraint costs and whether its enforcing rules are built:
    /// the list, or one constraint's page.
    #[command(subcommand_negates_reqs = true)]
    Architecture {
        #[command(subcommand)]
        command: Option<Architecture>,
        /// The constraint's slug (default: list them all; `decide` is the subcommand).
        slug: Option<String>,
        /// Text, or the list (with a slug, the constraint's entry) as JSON.
        #[arg(long, value_enum, default_value = "text")]
        format: CapabilitiesFormat,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Land a branch on `main`: gate it, merge it, check the merge, push it and record the
    /// land in the ledger; print `landed <sha> <kind>`.
    Land {
        /// The branch (default: the current one).
        branch: Option<String>,
        /// Run the land's check command and `clm check` on the working tree, and land and
        /// record nothing: print `checks pass` or name the command that failed.
        #[arg(long)]
        check_only: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Record your approval of a branch's current diff in the ledger.
    Approve {
        /// The branch (default: the current one).
        branch: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Push a `main` landed with no remote, and the local ledgers, to a newly added `origin`.
    Reconnect {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Set up CLM's tooling in the CLM repository found from the current directory, or at the
    /// top of the git repository when there is none, and print the steps only a human can do.
    Init {
        /// A language the repository uses, for its dead-code detector and build script;
        /// repeatable. Known: rust, typescript, python.
        #[arg(long = "lang")]
        langs: Vec<String>,
        /// Write no detectors, for an existing codebase whose dead code isn't cleaned up yet.
        #[arg(long)]
        no_detectors: bool,
        /// Where the remote lives (default: from `origin`'s URL, or `none` without one).
        #[arg(long, value_parser = ["none", "git", "github", "azure"])]
        host: Option<String>,
        /// Write nothing: print the allow rules unattended sessions need that the project's
        /// Claude Code settings lack, and exit 1 when one is missing.
        #[arg(long, conflicts_with_all = ["langs", "no_detectors", "host"])]
        check_permissions: bool,
    },
    /// Print what a change can break: `node <id>` and `file <path>` lines, or `all <path>`
    /// when every test must run.
    Affected {
        /// Compare with the merge base of this ref and HEAD.
        #[arg(long, default_value = "origin/main")]
        base: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
        /// Print only the `file` lines of the tests that read what changed, doc-only paths included.
        #[arg(long)]
        readers: bool,
    },
    /// Print one advisory report of the repository's health; exits 0 whatever it finds.
    Health {
        /// Hours after which an unmerged claim has lapsed.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Flag that a requirement was hard to build: write `backlog/workaround-<component>-<slug>-<n>.md`
    /// with the situation from stdin and print its path. Commits nothing.
    Workaround {
        /// The requirement, `<component>/<slug>`.
        requirement: String,
        /// The flag's title, naming what made it hard.
        #[arg(long)]
        title: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List the public items of the workspace's Rust libraries and TypeScript and Python
    /// packages that nothing outside their own library or package uses; exit 1 when it lists any.
    Unreferenced {
        /// Skip a library (`<crate>`) or an item (`<crate>::<name>`); an entry that skips
        /// nothing is reported. Repeatable.
        #[arg(long)]
        allow: Vec<String>,
        /// An attribute (its last path segment) that marks an item called from outside Rust;
        /// one that marks no public item is reported. Repeatable.
        #[arg(long)]
        entry: Vec<String>,
        /// Repository root (default: the CLM repository holding the current directory, else
        /// the top of the git repository).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print a compact map of the repository's code, for drafting its spec graph.
    Survey {
        /// Check this component map (YAML: `components:` a list of `{ id, paths, depends_on }`, and
        /// optional `outside` globs) against the code instead; exit 1 with what's wrong.
        #[arg(long = "components", value_name = "FILE")]
        nodes: Option<PathBuf>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print CLM's guide: with no page, the index of its pages; with one, that page.
    Guide {
        /// The page to print, by name (`index` for the guide's start).
        page: Option<String>,
    },
    /// Print a fingerprint of HEAD's changes relative to a base ref.
    Fingerprint {
        /// Base ref the PR merges into.
        #[arg(long)]
        base: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Hook {
    /// PreToolUse: exit 2 with the reason when a Bash command would cut a release or push to
    /// `main`; print the `ask` decision when it would run `clm approve`, or file a CLM defect
    /// as an issue.
    Guard,
    /// SessionStart: print `clm status` and the rule for reporting CLM defects when the
    /// session's cwd is a CLM repository.
    SessionStart,
    /// Stop: push a snapshot of the spec work on a `spec/*` branch; silent, never fails.
    /// The `autosync` setting (`clm config get autosync`) turns it off.
    Autosync,
}

#[derive(Subcommand)]
enum Page {
    /// Open the branch's pull request against `main` on the host, or find the open one, and
    /// print its URL, or `none` without a host.
    Open {
        /// The branch, such as `spec/x`.
        branch: String,
        /// The pull request's title.
        #[arg(long)]
        title: String,
        /// The file holding the pull request's body.
        #[arg(long)]
        body_file: PathBuf,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Set the body of the current branch's open pull request to a file's contents.
    Body {
        /// The file holding the body.
        file: PathBuf,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Contribution {
    /// Print this repository's parent as `<owner>/<name>`, or `none` when it is not a fork.
    Parent {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Bring `origin`'s `main` up to its parent's, without force.
    Sync {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Open a pull request from this fork's branch to its parent's `main`, or find the open
    /// one, and print its URL.
    Open {
        /// The branch, such as `spec/x`.
        branch: String,
        /// The pull request's title.
        #[arg(long)]
        title: String,
        /// The file holding the pull request's body.
        #[arg(long)]
        body_file: PathBuf,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print a pull request or issue: its fields, then its changed files, then its body.
    Show {
        /// The pull request's or issue's number.
        number: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Make a local branch at a pull request's head, without checking it out.
    Fetch {
        /// The pull request's number.
        number: u64,
        /// The local branch to make; it must not exist.
        branch: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Comment on a pull request or issue, closing it with `--close`.
    Comment {
        /// The pull request's or issue's number.
        number: u64,
        /// The file holding the comment.
        #[arg(long)]
        body_file: PathBuf,
        /// Close it after commenting.
        #[arg(long)]
        close: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Add a label to a pull request or issue.
    Label {
        /// The pull request's or issue's number.
        number: u64,
        /// The label.
        label: String,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Branch {
    /// Delete `origin`'s branch when its tip is still `--lease` (default: the tip last
    /// fetched) and this clone pushed it, on every host.
    Delete {
        /// The branch, such as `impl/a`.
        branch: String,
        /// The commit `origin`'s branch must still be at.
        #[arg(long)]
        lease: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Push the checked-out branch to `origin`, overwriting only what this clone last saw.
    Push {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Effort {
    /// Read this machine's transcripts and append an `effort` record for each session and
    /// branch whose counts changed.
    Record {
        /// Only this branch's sessions.
        #[arg(long)]
        branch: Option<String>,
        /// The transcripts directory (default: `$CLAUDE_CONFIG_DIR/projects`, else
        /// `~/.claude/projects`).
        #[arg(long)]
        transcripts: Option<PathBuf>,
        /// A directory whose sessions count, besides the repository's worktrees.
        #[arg(long = "cwd")]
        cwds: Vec<PathBuf>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Capabilities {
    /// Record a review decision on a capability, settling the workaround flags open on it.
    Decide {
        /// The capability's slug.
        slug: String,
        /// What the capability is worth.
        #[arg(long, value_enum)]
        value: RatingArg,
        /// What to do with it.
        #[arg(long, value_enum)]
        decision: DecisionArg,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Architecture {
    /// Record a review decision on an architectural constraint, settling the workaround flags
    /// open on it.
    Decide {
        /// The constraint's slug.
        slug: String,
        /// What the constraint is worth.
        #[arg(long, value_enum)]
        value: RatingArg,
        /// What to do with it.
        #[arg(long, value_enum)]
        decision: DecisionArg,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum RatingArg {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, ValueEnum)]
enum DecisionArg {
    Keep,
    Cut,
    Redesign,
    Accept,
}

#[derive(Subcommand)]
enum Ledger {
    /// Rewrite your own ledger without approvals of landed diffs and checks older than the
    /// newest land, push it, and print how many records went.
    Compact {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Report each commit on `main` no `landed` record names, merge or not, and each land no
    /// `checked` record covers; exit 1 when there is one. Doesn't fetch.
    Verify {
        /// Audit only the commits after this one (default: from the first landed commit).
        #[arg(long)]
        since: Option<String>,
        /// Also recompute each signed land's kind and whether it needed review with the gate,
        /// and report a land whose record states less than the gate gives.
        #[arg(long)]
        review: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print every record, one per line in time order. Doesn't fetch.
    Show {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Config {
    /// Print a key's value and the layer it came from, tab-separated.
    Get {
        /// The key, such as `defects`.
        key: String,
    },
    /// Print every key with its value and the layer it came from, tab-separated.
    List,
}

fn main() -> ExitCode {
    closed_stdout::run(dispatch)
}

fn dispatch() -> ExitCode {
    let command = Cli::parse().command;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Some(error) = upgrade_gate::blocked(&command, &cwd, clm_upgrade::require_current) {
        eprintln!("{error}");
        return ExitCode::from(4);
    }
    match command {
        Command::Check { no_detectors, root } => {
            let root = repo_root(root);
            let mut violations = check(&root, !no_detectors);
            violations.extend(ledger_on_main(&root));
            for v in &violations {
                println!("{}", clm_check::format(v));
            }
            if violations.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Command::Coverage { root } => match clm_check::coverage(&repo_root(root)) {
            Ok(report) => {
                for line in report {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(violations) => {
                for v in &violations {
                    println!("{}", clm_check::format(v));
                }
                ExitCode::FAILURE
            }
        },
        Command::Pending { node, at, root } => {
            let (head, result) =
                clm_view_repo::pending(&repo_root(root), node.as_deref(), at.as_deref());
            if let Some(head) = head {
                eprintln!("{head}");
            }
            match result {
                Ok(list) => {
                    for line in list {
                        println!("{line}");
                    }
                    ExitCode::SUCCESS
                }
                Err(clm_view_repo::Error::Violations(violations)) => report(&violations),
                Err(clm_view_repo::Error::At(why)) => {
                    eprintln!("{why}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Size { node, root } => size(&repo_root(root), node.as_deref()),
        Command::Apply { change, root } => match clm_apply::apply(&repo_root(root), &change) {
            Ok(ops) => {
                println!("applied {change}");
                for op in ops {
                    let verb = match op {
                        clm_apply::FileOp::Write(..) => "wrote",
                        clm_apply::FileOp::Remove(_) => "removed",
                    };
                    println!("  {verb} {}", op.path().display());
                }
                ExitCode::SUCCESS
            }
            Err(violations) => {
                for v in &violations {
                    println!("{}", clm_check::format(v));
                }
                ExitCode::FAILURE
            }
        },
        Command::Rename { change, root } => match clm_rename::rename(&repo_root(root), &change) {
            Ok(lines) => {
                lines.iter().for_each(|line| println!("{line}"));
                ExitCode::SUCCESS
            }
            Err(failure) => {
                eprintln!("{failure}");
                ExitCode::from(failure.exit_code())
            }
        },
        Command::Archive { change, root } => match clm_apply::archive(&repo_root(root), &change) {
            Ok(to) => {
                println!("archived {change} -> {}", to.display());
                ExitCode::SUCCESS
            }
            Err(violations) => report(&violations),
        },
        Command::Backlog { root } => match clm_check::backlog(&repo_root(root)) {
            Ok(items) => {
                for item in items {
                    println!("{item}");
                }
                ExitCode::SUCCESS
            }
            Err(violations) => report(&violations),
        },
        Command::Status { root } => {
            let root = repo_root(root);
            match clm_view_repo::status(&root, budgets(&root)) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(violations) => report(&violations),
            }
        }
        Command::View {
            name,
            format,
            width,
            at,
            root,
        } => {
            let root = repo_root(root);
            // Only the `spec` view marks oversized nodes; settings load only for it.
            let spec = matches!(name, ViewName::Spec);
            let source = match at.as_deref() {
                None => clm_view_repo::Source::Tree(spec.then(|| budgets(&root)).flatten()),
                Some(rev) => clm_view_repo::Source::At {
                    rev,
                    budgets: spec.then_some(&budgets_from as clm_view_repo::BudgetsFrom),
                },
            };
            let result = clm_view_repo::view(
                &root,
                name.into(),
                format.into(),
                usize::from(width),
                source,
            );
            print_view(result, &mut std::io::stdout(), &mut std::io::stderr())
        }
        Command::Context { node, hash, root } => {
            let root = repo_root(root);
            let (graph, violations) = clm_model::load(&root);
            if !violations.is_empty() {
                return report(&violations);
            }
            match clm_context::bundle(&graph, &root, &node) {
                Some(bundle) if hash => {
                    println!("{}", clm_context::hash(&bundle));
                    ExitCode::SUCCESS
                }
                Some(bundle) => {
                    print!("{bundle}");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("clm context: no component `{node}`");
                    ExitCode::from(2)
                }
            }
        }
        Command::Upgrade {
            apply,
            note,
            decline,
            lock,
        } => {
            let request = upgrade_command::Request {
                apply,
                note,
                decline,
                lock,
            };
            match upgrade_command::run(&repo_root(None), request) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(why) => {
                    eprintln!("clm upgrade: {why}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Hashes { node, root } => {
            let (graph, _) = clm_model::load(&repo_root(root));
            if let Some(layer) = clm_model::Layer::ALL.into_iter().find(|l| l.id() == node) {
                for (element, hash) in clm_apply::layer_hashes(&graph, layer) {
                    println!("{element} {hash}");
                }
                return ExitCode::SUCCESS;
            }
            match graph.nodes.get(&node) {
                Some(n) => {
                    for (element, hash) in clm_apply::hashes(n) {
                        println!("{element} {hash}");
                    }
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("clm hashes: no valid component `{node}`");
                    ExitCode::from(2)
                }
            }
        }
        Command::Gate {
            base,
            approved,
            root,
        } => match clm_gate::evaluate_at(&repo_root(root), &base, "HEAD", approved) {
            Ok(outcome) => {
                if outcome.review {
                    println!("{} review", outcome.kind);
                } else {
                    println!("{}", outcome.kind);
                }
                for v in &outcome.violations {
                    println!("{}", clm_check::format(v));
                }
                if outcome.violations.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("clm gate: {e}");
                ExitCode::from(2)
            }
        },
        Command::Next {
            expiry,
            wait,
            interval,
            root,
        } => {
            let root = repo_root(root);
            orchestrated(
                "next",
                clm_config::load(Some(&root), &clm_config::process_env)
                    .map_err(|e| clm_orchestrator::Error::Refused(format!("settings: {e}")))
                    .and_then(|s| {
                        let max = s.max_builds.value;
                        if wait {
                            clm_orchestrator::next_wait(&root, expiry, max, &interval, &mut |n| {
                                eprintln!("{n}")
                            })
                        } else {
                            clm_orchestrator::next(&root, expiry, max)
                        }
                    })
                    .map(|n| n.join("\n")),
            )
        }
        Command::Claim {
            node,
            fix,
            expiry,
            root,
        } => orchestrated(
            "claim",
            match fix {
                Some(item) => clm_orchestrator::claim_fix(&repo_root(root), &node, &item, expiry),
                None => clm_orchestrator::claim(&repo_root(root), &node, expiry),
            },
        ),
        Command::Run {
            node,
            finish,
            expiry,
            review_model,
            root,
        } => {
            if finish {
                orchestrated("run", clm_run::finish(&repo_root(root), &node, expiry))
            } else {
                match clm_run::run(&repo_root(root), &node, expiry, review_model.as_deref()) {
                    Err(clm_run::RunError::NoPlace(why)) => {
                        eprintln!("clm run: {why}");
                        ExitCode::from(3)
                    }
                    Err(clm_run::RunError::Failed(e)) => orchestrated("run", Err(e)),
                    Ok(launcher) => orchestrated("run", Ok(launcher)),
                }
            }
        }
        Command::Proposals { nodes, root } => orchestrated(
            "proposals",
            clm_orchestrator::proposals(&repo_root(root), &nodes).map(|p| {
                for warning in &p.warnings {
                    eprintln!("clm proposals: warning: {warning}");
                }
                p.lines.join("\n")
            }),
        ),
        Command::Triage { expiry, root } => orchestrated(
            "triage",
            clm_triage::triage(&repo_root(root), expiry).map(|t| {
                for warning in &t.warnings {
                    eprintln!("clm triage: warning: {warning}");
                }
                t.lines.join("\n")
            }),
        ),
        Command::Intake { root } => {
            let root = repo_root(root);
            match clm_config::load(Some(&root), &clm_config::process_env) {
                Ok(settings) => orchestrated(
                    "intake",
                    clm_triage::intake(&root, &*clm_host::adapter(settings.host.value))
                        .map(|lines| lines.join("\n")),
                ),
                Err(e) => {
                    eprintln!("clm intake: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Draft {
            finish: Some(change),
            expiry,
            root,
            ..
        } => orchestrated(
            "draft",
            clm_triage::finish(&repo_root(root), &change, expiry),
        ),
        Command::Draft {
            slugs: ids,
            fold: true,
            root,
            ..
        } => match clm_triage::fold(&repo_root(root), &ids) {
            Ok(fold) => {
                for (id, why) in &fold.left {
                    eprintln!("left {id}: {why}");
                }
                match fold.launcher {
                    Some(launcher) => orchestrated("draft", Ok(launcher)),
                    None => ExitCode::FAILURE,
                }
            }
            Err(e) => orchestrated("draft", Err(e)),
        },
        Command::Draft {
            slugs,
            finish: None,
            answers,
            expiry,
            root,
            ..
        } => {
            // Against the directory it runs in, before `repo_root` may move to the root.
            let answers = answers.map(|a| std::path::absolute(&a).unwrap_or(a));
            let root = repo_root(root);
            let settings = match clm_config::load(Some(&root), &clm_config::process_env) {
                Ok(s) => s,
                Err(e) => {
                    let e = clm_orchestrator::Error::Refused(format!("settings: {e}"));
                    return orchestrated("draft", Err(e));
                }
            };
            let clock = clm_triage::Clock {
                now: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
                tz: std::env::var_os("TZ"),
            };
            match clm_triage::draft(
                &root,
                &slugs,
                answers.as_deref(),
                settings.max_builds.value,
                expiry,
                &clock,
            ) {
                Ok(draft) => {
                    for (slug, why) in &draft.left {
                        eprintln!("left {slug}: {why}");
                    }
                    match draft.drafted {
                        Some(d) => {
                            let code = orchestrated("draft", Ok(d.launcher));
                            eprintln!("draft {}", d.id);
                            code
                        }
                        None => ExitCode::FAILURE,
                    }
                }
                Err(clm_triage::DraftError::NoPlace(why)) => {
                    eprintln!("clm draft: {why}");
                    ExitCode::from(3)
                }
                Err(clm_triage::DraftError::Failed(e)) => orchestrated("draft", Err(e)),
            }
        }
        Command::Runner {
            permission_mode,
            interval,
            once,
            expiry,
            root,
        } => ExitCode::from(clm_runner::runner(&clm_runner::Options {
            clm: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("clm")),
            claude: "claude".into(),
            root: repo_root(root),
            permission_mode,
            interval,
            once,
            expiry,
        })),
        Command::WrapUp { root } => wrap_up(root),
        Command::Rebase { root } => rebase(root),
        Command::Autosync { root } => match clm_sync::autosync(&repo_root(root)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => failed("autosync", &e),
        },
        Command::Superpowers { skill } => {
            orchestrated("superpowers", clm_run::superpowers_skill(&skill))
        }
        Command::Hook { hook } => {
            // A hook that can't read its event has nothing to do; it never breaks a session.
            let mut event = String::new();
            let _ = std::io::stdin().read_to_string(&mut event);
            match hook {
                Hook::Guard => match clm_guard::guard(&event) {
                    clm_guard::Decision::Allow => ExitCode::SUCCESS,
                    clm_guard::Decision::Ask(why) => {
                        println!("{}", clm_guard::ask_json(&why));
                        ExitCode::SUCCESS
                    }
                    clm_guard::Decision::Block(why) => {
                        eprintln!("{why}");
                        ExitCode::from(2)
                    }
                },
                Hook::SessionStart => {
                    print!("{}", clm_hooks::session_start(&event));
                    ExitCode::SUCCESS
                }
                Hook::Autosync => {
                    clm_hooks::autosync(&event);
                    ExitCode::SUCCESS
                }
            }
        }
        Command::Defect { title, nodes } => {
            let mut report = String::new();
            let filed = std::io::stdin()
                .read_to_string(&mut report)
                .map_err(|e| format!("reading the report: {e}"))
                .and_then(|_| {
                    std::env::current_dir().map_err(|e| format!("current directory: {e}"))
                })
                .and_then(|cwd| clm_hooks::defect(&cwd, &title, &nodes, &report));
            match filed {
                Ok(out) => {
                    println!("{out}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("clm defect: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Ledger { ledger } => ledger_command(ledger),
        Command::Effort {
            effort:
                Effort::Record {
                    branch,
                    transcripts,
                    cwds,
                    root,
                },
        } => {
            // Relative paths are the shell's, before `--root` is found.
            let absolute = |path: PathBuf| std::path::absolute(&path).unwrap_or(path);
            let options = clm_effort::Options {
                branch,
                transcripts: transcripts.map(absolute),
                cwds: cwds.into_iter().map(absolute).collect(),
            };
            match clm_effort::record(&repo_root(root), &options, &clm_config::process_env) {
                Ok(done) => {
                    for warning in &done.warnings {
                        eprintln!("warning: {warning}");
                    }
                    for line in &done.lines {
                        println!("{line}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("clm effort: {e}");
                    ExitCode::from(e.exit_code())
                }
            }
        }
        Command::Capabilities {
            command:
                Some(Capabilities::Decide {
                    slug,
                    value,
                    decision,
                    root,
                }),
            slug: parent_slug,
            root: parent_root,
            ..
        } => match parent_slug {
            // `clm capabilities <slug> decide ...` isn't a decision on `<slug>`.
            Some(parent_slug) => {
                eprintln!(
                    "clm capabilities: `decide` takes the slug after it: clm capabilities \
                     decide <slug> (not {parent_slug})"
                );
                ExitCode::from(2)
            }
            None => decide(&repo_root(root.or(parent_root)), &slug, value, decision),
        },
        Command::Capabilities {
            slug, format, root, ..
        } => capabilities(&repo_root(root), slug, format),
        Command::Architecture {
            command:
                Some(Architecture::Decide {
                    slug,
                    value,
                    decision,
                    root,
                }),
            slug: parent_slug,
            root: parent_root,
            ..
        } => match parent_slug {
            // `clm architecture <slug> decide ...` isn't a decision on `<slug>`.
            Some(parent_slug) => {
                eprintln!(
                    "clm architecture: `decide` takes the slug after it: clm architecture \
                     decide <slug> (not {parent_slug})"
                );
                ExitCode::from(2)
            }
            None => decide_architecture(&repo_root(root.or(parent_root)), &slug, value, decision),
        },
        Command::Architecture {
            slug, format, root, ..
        } => architecture(&repo_root(root), slug, format),
        Command::Land {
            branch,
            check_only: true,
            root,
        } => landed("land", |env| {
            clm_land::check_only(&repo_root(root), branch.as_deref(), env)
        }),
        Command::Land { branch, root, .. } => landed("land", |env| {
            clm_land::land(&repo_root(root), branch.as_deref(), env)
        }),
        Command::Approve { branch, root } => landed("approve", |env| {
            clm_land::approve(&repo_root(root), branch.as_deref(), env)
        }),
        Command::Reconnect { root } => landed("reconnect", |env| {
            clm_land::reconnect(&repo_root(root), env)
        }),
        Command::Config { config } => {
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    eprintln!("clm config: current directory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let root = clm_hooks::clm_root(&cwd);
            let lines =
                clm_config::load(root, &clm_config::process_env).and_then(|s| match config {
                    Config::Get { key } => s
                        .get(&key)
                        .map(|(value, layer)| vec![format!("{value}\t{layer}")]),
                    Config::List => s.list(),
                });
            match lines {
                Ok(lines) => {
                    for line in lines {
                        println!("{line}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("clm config: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Pr { page } => pr(page),
        Command::Issue {
            title,
            body_file,
            repo,
            root,
        } => hosted("issue", root, |host, root| {
            host.issue(root, &title, &body_file, &repo).map(Some)
        }),
        Command::Contribution { contribution } => contribution_command(contribution),
        Command::Branch {
            branch:
                Branch::Delete {
                    branch,
                    lease,
                    root,
                },
        } => hosted("branch delete", root, |host, root| {
            clm_host::delete_pushed_branch(host, root, &branch, lease.as_deref()).map(|()| None)
        }),
        Command::Branch {
            branch: Branch::Push { root },
        } => hosted("branch push", root, |_, root| {
            clm_host::push_branch(root).map(Some)
        }),
        Command::Init {
            langs,
            no_detectors,
            host,
            check_permissions,
        } => {
            if check_permissions {
                return match std::env::current_dir()
                    .map_err(|e| format!("current directory: {e}"))
                    .and_then(|cwd| clm_init::check_permissions(&cwd))
                {
                    Ok(text) if text.is_empty() => ExitCode::SUCCESS,
                    Ok(text) => {
                        print!("{text}");
                        ExitCode::FAILURE
                    }
                    Err(e) => {
                        eprintln!("clm init: {e}");
                        ExitCode::FAILURE
                    }
                };
            }
            let opts = clm_init::Options {
                langs,
                detectors: !no_detectors,
                host: host.map(|h| match h.as_str() {
                    "none" => clm_config::Host::None,
                    "git" => clm_config::Host::Git,
                    "github" => clm_config::Host::Github,
                    "azure" => clm_config::Host::Azure,
                    other => unreachable!("clap allows no host `{other}`"),
                }),
            };
            let done = std::env::current_dir()
                .map_err(|e| format!("current directory: {e}"))
                .and_then(|cwd| clm_init::init(&cwd, &opts));
            match done {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("clm init: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Guide { page } => match clm_init::guide(page.as_deref()) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprint!("{e}");
                ExitCode::FAILURE
            }
        },
        Command::Affected {
            base,
            root,
            readers,
        } => affected(&repo_root(root), &base, readers),
        Command::Health { expiry, root } => health(&repo_root(root), expiry),
        Command::Workaround {
            requirement,
            title,
            root,
        } => flag_workaround(&repo_root(root), &requirement, &title),
        Command::Unreferenced { allow, entry, root } => {
            match clm_unreferenced::scan(&survey_root(root), &allow, &entry) {
                Ok(lines) if lines.is_empty() => ExitCode::SUCCESS,
                Ok(lines) => {
                    for line in lines {
                        println!("{line}");
                    }
                    ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("clm unreferenced: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Survey { nodes: None, root } => match clm_init::survey(&survey_root(root)) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("clm survey: {e}");
                ExitCode::from(2)
            }
        },
        Command::Survey {
            nodes: Some(file),
            root,
        } => {
            // Read before `repo_root`, which may move to the repository's top.
            let checked = std::fs::read_to_string(&file)
                .map_err(|e| format!("{}: {e}", file.display()))
                .and_then(|yaml| clm_init::check_nodes(&survey_root(root), &file, &yaml));
            match checked {
                Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
                Ok(violations) => report(&violations),
                Err(e) => {
                    eprintln!("clm survey: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Fingerprint { base, root } => match clm_gate::fingerprint(&repo_root(root), &base)
        {
            Ok(id) => {
                println!("{id}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("clm fingerprint: {e}");
                ExitCode::from(2)
            }
        },
    }
}

/// `clm pr`.
fn pr(page: Page) -> ExitCode {
    match page {
        Page::Open {
            branch,
            title,
            body_file,
            root,
        } => hosted("pr open", root, |host, root| {
            let url = host.page_open(root, &branch, &title, &body_file)?;
            Ok(Some(url.unwrap_or_else(|| "none".into())))
        }),
        Page::Body { file, root } => {
            hosted("pr body", root, |host, root| host.page_body(root, &file))
        }
    }
}

/// `clm contribution`: `host`'s contribution operations (`clm/contribution-command`).
fn contribution_command(contribution: Contribution) -> ExitCode {
    match contribution {
        Contribution::Parent { root } => hosted("contribution parent", root, |host, root| {
            Ok(Some(
                host.parent(root)
                    .map_err(refused)?
                    .unwrap_or_else(|| "none".into()),
            ))
        }),
        Contribution::Sync { root } => hosted("contribution sync", root, |host, root| {
            host.sync(root).map(|()| None).map_err(refused)
        }),
        Contribution::Open {
            branch,
            title,
            body_file,
            root,
        } => hosted("contribution open", root, |host, root| {
            host.open_on_parent(root, &branch, &title, &body_file)
                .map(Some)
                .map_err(refused)
        }),
        Contribution::Show { number, root } => hosted("contribution show", root, |host, root| {
            host.show(root, number)
                .map(|shown| Some(shown_text(&shown)))
                .map_err(refused)
        }),
        Contribution::Fetch {
            number,
            branch,
            root,
        } => hosted("contribution fetch", root, |host, root| {
            host.fetch(root, number, &branch)
                .map(|()| None)
                .map_err(refused)
        }),
        Contribution::Comment {
            number,
            body_file,
            close,
            root,
        } => hosted("contribution comment", root, |host, root| {
            host.comment(root, number, &body_file, close)
                .map(|()| None)
                .map_err(refused)
        }),
        Contribution::Label {
            number,
            label,
            root,
        } => hosted("contribution label", root, |host, root| {
            host.label(root, number, &label)
                .map(|()| None)
                .map_err(refused)
        }),
    }
}

/// A failed `gh` or `git` is a refusal for `clm contribution`: every failure exits 1 with its
/// reason.
fn refused(e: clm_host::Error) -> clm_host::Error {
    match e {
        clm_host::Error::Git(why) => clm_host::Error::Refused(why),
        other => other,
    }
}

/// What `clm contribution show` prints: a labelled line per field, a line per changed file,
/// then the body.
fn shown_text(shown: &clm_host::Shown) -> String {
    let none = || "none".to_string();
    let mut lines = vec![
        format!("author: {}", shown.author),
        format!("author id: {}", shown.author_id),
        format!("state: {}", shown.state),
        format!("title: {}", shown.title),
    ];
    if shown.pull_request {
        lines.push(format!(
            "head repository: {}",
            shown.head_repo_url.clone().unwrap_or_else(none)
        ));
        lines.push(format!(
            "head branch: {}",
            shown.head_branch.clone().unwrap_or_else(none)
        ));
        lines.push(format!(
            "maintainers can edit: {}",
            shown.maintainers_can_edit
        ));
        lines.extend(shown.files.iter().map(|f| format!("file: {f}")));
    }
    lines.push("body:".into());
    lines.push(shown.body.clone());
    lines.join("\n")
}

/// `clm wrap-up`.
fn wrap_up(root: Option<PathBuf>) -> ExitCode {
    match clm_checkpoint::checkpoint(&repo_root(root)) {
        Ok(report) => {
            for line in &report.lines {
                println!("{line}");
            }
            if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => orchestrated("wrap-up", Err(e)),
    }
}

/// `clm rebase`.
fn rebase(root: Option<PathBuf>) -> ExitCode {
    // `sync/outcome-line`: the last line of stdout states the outcome, whatever happened,
    // for a caller that can't read the exit code.
    let root = repo_root(root);
    // Called with the CLM root of `HEAD` checked out in a temporary worktree.
    let (code, outcome) = match clm_sync::sync(&root, sync_settings) {
        Ok(report) => {
            for warning in &report.warnings {
                eprintln!("clm rebase: {warning}");
            }
            for line in &report.lines {
                println!("{line}");
            }
            if report.ok {
                (ExitCode::SUCCESS, "ok")
            } else {
                (ExitCode::FAILURE, "incomplete")
            }
        }
        Err(e) => {
            let outcome = match e.exit_code() {
                1 => "incomplete",
                _ => "failed",
            };
            (failed("rebase", &e), outcome)
        }
    };
    println!("rebase {outcome}");
    code
}

/// Runs `op` with the host adapter the repository's `host` setting names, printing the line
/// it returns, if any: exit 1 when it refuses, 2 when the settings don't load or `git` or the
/// host's tool fails.
fn hosted(
    command: &str,
    root: Option<PathBuf>,
    op: impl FnOnce(&dyn clm_host::Host, &Path) -> Result<Option<String>, clm_host::Error>,
) -> ExitCode {
    let root = repo_root(root);
    let host = match clm_config::load(Some(&root), &clm_config::process_env) {
        Ok(settings) => settings.host.value,
        Err(e) => {
            eprintln!("clm {command}: {e}");
            return ExitCode::from(2);
        }
    };
    match op(&*clm_host::adapter(host), &root) {
        Ok(line) => {
            if let Some(line) = line {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(clm_host::Error::Refused(why)) => {
            eprintln!("clm {command}: {why}");
            ExitCode::FAILURE
        }
        Err(clm_host::Error::Nowhere(report)) => {
            print!("{report}");
            if !report.ends_with('\n') {
                println!();
            }
            eprintln!("clm {command}: host `{host}` has nowhere to file it");
            ExitCode::FAILURE
        }
        Err(clm_host::Error::Git(e)) => {
            eprintln!("clm {command}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `root-from-cwd`: an explicit `--root` as given; else `.` in the CLM repository holding the
/// current directory, as `session-start` finds it, moving there first when it's higher up;
/// else `.`. Every path a command prints is then the same as when run at the top.
fn repo_root(given: Option<PathBuf>) -> PathBuf {
    if let Some(root) = given {
        return root;
    }
    let here = PathBuf::from(".");
    let Ok(cwd) = std::env::current_dir() else {
        return here;
    };
    match clm_hooks::clm_root(&cwd) {
        Some(found) if found != cwd => match std::env::set_current_dir(found) {
            Ok(()) => here,
            Err(_) => found.to_path_buf(),
        },
        _ => here,
    }
}

/// `clm runner --interval`: a positive decimal number of minutes.
fn interval_minutes(s: &str) -> Result<std::time::Duration, String> {
    let minutes: f64 = s.parse().map_err(|_| format!("`{s}` isn't a number"))?;
    if minutes.is_nan() || minutes <= 0.0 {
        return Err(format!("`{s}` isn't a positive number of minutes"));
    }
    std::time::Duration::try_from_secs_f64(minutes * 60.0).map_err(|e| format!("`{s}`: {e}"))
}

/// `repo_root`, except that with no CLM repository found, as before `clm init`, it's the
/// top of the git repository, so the survey maps all of it from any directory.
fn survey_root(given: Option<PathBuf>) -> PathBuf {
    let found = std::env::current_dir()
        .ok()
        .is_some_and(|cwd| clm_hooks::clm_root(&cwd).is_some());
    if given.is_some() || found {
        return repo_root(given);
    }
    clm_git::git(Path::new("."), &["rev-parse", "--show-toplevel"])
        .ok()
        .filter(|top| !top.is_empty())
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// Prints an orchestrator result: 0 on success, 1 when refused, 2 when git fails.
fn orchestrated(cmd: &str, result: Result<String, clm_orchestrator::Error>) -> ExitCode {
    use clm_orchestrator::Error;
    match result {
        Ok(text) => {
            if !text.is_empty() {
                println!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(Error::Refused(why)) => {
            eprintln!("clm {cmd}: {why}");
            ExitCode::FAILURE
        }
        Err(Error::Spec(violations)) => report(&violations),
        Err(Error::Git(e)) => {
            eprintln!("clm {cmd}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `clm <cmd>: <message>` on stderr, and the `Failure`'s exit code: 1 when `clm <cmd>`
/// refused, 2 when git failed.
fn failed(cmd: &str, e: &clm_git::Failure) -> ExitCode {
    eprintln!("clm {cmd}: {e}");
    ExitCode::from(e.exit_code())
}

/// `run`'s violations, the imports `depends_on` doesn't declare, then, with `detectors`,
/// those of the detectors the repository's settings name, or why the settings can't be
/// loaded.
fn check(root: &Path, detectors: bool) -> Vec<clm_check::Violation> {
    let mut violations = clm_check::run(root);
    match clm_init::import_edges(root) {
        Ok(edges) => {
            let (graph, _) = clm_model::load(root);
            violations.extend(clm_check::undeclared_imports(&graph, &edges));
        }
        Err(e) => violations.push(clm_check::Violation {
            path: root.to_path_buf(),
            rule: "check/undeclared-imports",
            message: e,
        }),
    }
    if !detectors {
        return violations;
    }
    match clm_config::load(Some(root), &clm_config::process_env) {
        Ok(settings) => {
            let list: Vec<_> = settings
                .detectors
                .value
                .into_iter()
                .filter(|d| match &d.requires {
                    Some(path) if !root.join(path).exists() => {
                        eprintln!("{} skipped: no {path}", d.name);
                        false
                    }
                    _ => true,
                })
                .map(|d| clm_check::Detector {
                    name: d.name,
                    command: d.command,
                    hint: d.hint,
                    timeout: d.timeout,
                })
                .collect();
            violations.extend(clm_check::detectors(root, &list));
        }
        Err(e) => violations.push(settings_violation(e, "check/detectors")),
    }
    violations
}

/// `clm land`, `clm approve` and `clm reconnect`: the lines on stdout, the
/// warnings on stderr; exit 1 when it refused, 2 when git, the ledger or the settings failed.
fn landed(
    command: &str,
    op: impl FnOnce(&clm_land::Env) -> Result<clm_land::Output, clm_land::Error>,
) -> ExitCode {
    match op(&clm_config::process_env) {
        Ok(out) => {
            for warning in out.warnings {
                eprintln!("clm {command}: warning: {warning}");
            }
            for line in out.lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(clm_land::Error::Refused(why)) => {
            eprintln!("clm {command}: {why}");
            ExitCode::FAILURE
        }
        Err(clm_land::Error::Git(e)) => {
            eprintln!("clm {command}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `clm ledger <command>`: exit 1 on a finding, 2 when git or the settings fail.
fn ledger_command(ledger: Ledger) -> ExitCode {
    let failed = |e: clm_ledger::Error| {
        eprintln!("clm ledger: {e}");
        ExitCode::from(2)
    };
    match ledger {
        Ledger::Compact { root } => {
            match clm_ledger::compact(&repo_root(root), &clm_config::process_env) {
                Ok(done) => {
                    for warning in &done.warnings {
                        eprintln!("warning: {warning}");
                    }
                    println!("removed {}", done.removed);
                    ExitCode::SUCCESS
                }
                Err(e) => failed(e),
            }
        }
        Ledger::Verify {
            since,
            review,
            root,
        } => match clm_ledger::verify(&repo_root(root), "main", since.as_deref(), review) {
            Ok(findings) if findings.is_empty() => ExitCode::SUCCESS,
            Ok(findings) => report(&findings.into_iter().map(finding).collect::<Vec<_>>()),
            Err(e) => failed(e),
        },
        Ledger::Show { root } => match clm_ledger::fold(&repo_root(root)) {
            Ok(fold) => {
                for warning in &fold.warnings {
                    eprintln!("warning: {warning}");
                }
                for line in clm_ledger::show(&fold) {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => failed(e),
        },
    }
}

/// A `clm ledger verify` finding in `check`'s format, against `main`.
fn finding(f: clm_ledger::Finding) -> clm_check::Violation {
    clm_check::Violation {
        path: "main".into(),
        rule: f.rule,
        message: f.message,
    }
}

/// `clm ledger verify`'s findings when the checked-out branch is `main` and the repository
/// has a ledger branch, so a push to `main` that didn't go through `clm land` fails there
/// (`no-silent-drift`).
fn ledger_on_main(root: &Path) -> Vec<clm_check::Violation> {
    let on_main = matches!(clm_git::current_branch(root), Ok(Some(b)) if b == "main");
    if !on_main {
        return Vec::new();
    }
    let audit = clm_ledger::has_ledger(root).and_then(|has| match has {
        // Without `--review`: recomputing runs the gate once per signed land, on every check of
        // `main`.
        true => clm_ledger::verify(root, "main", None, false),
        false => Ok(Vec::new()),
    });
    match audit {
        Ok(findings) => findings.into_iter().map(finding).collect(),
        // Not audited is not clean.
        Err(e) => vec![clm_check::Violation {
            path: "main".into(),
            rule: "ledger/verify",
            message: format!("can't audit main against the ledger: {e}"),
        }],
    }
}

/// Why the settings can't be loaded, as a violation of `rule` against their source.
fn settings_violation(e: clm_config::Error, rule: &'static str) -> clm_check::Violation {
    clm_check::Violation {
        path: e.source.unwrap_or_else(|| "settings".into()).into(),
        rule,
        message: one_line(&e.message),
    }
}

/// `config`'s `health` and `node_size`, as `clm-health` takes them.
fn health_settings(settings: clm_config::Settings) -> clm_health::Settings {
    let (health, size) = (settings.health.value, settings.node_size.value);
    clm_health::Settings {
        reconsider: health.reconsider,
        scenarios: health.scenarios,
        exceptions: health.exceptions,
        stale_days: health.stale_days,
        lcov: health.lcov,
        component_size: clm_check::Budgets {
            spec_tokens: size.spec_tokens,
            code_tokens: size.code_tokens,
        },
    }
}

/// What `clm rebase` reads from the settings at `root`: `health` for the reconsider items, or
/// the error as `clm check` prints a violation.
fn sync_settings(root: &Path) -> Result<clm_health::Settings, String> {
    clm_config::load(Some(root), &clm_config::process_env)
        .map(health_settings)
        .map_err(|e| clm_check::format(&settings_violation(e, "check.health/health-report")))
}

/// `clm health`: the report on stdout, with `config`'s `health` and `node_size`; exit 1
/// when the settings or the graph can't be loaded, 2 when git fails.
fn health(root: &Path, expiry: u64) -> ExitCode {
    let settings = match clm_config::load(Some(root), &clm_config::process_env) {
        Ok(s) => s,
        Err(e) => {
            let v = settings_violation(e, "check.health/health-report");
            eprintln!("clm health: {}", clm_check::format(&v));
            return ExitCode::FAILURE;
        }
    };
    match clm_health::report(root, &health_settings(settings), expiry) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("clm health: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

/// `clm affected`: its lines on stdout, with `config`'s `[affected] all` globs.
fn affected(root: &Path, base: &str, readers: bool) -> ExitCode {
    if readers {
        return match clm_affected::affected_readers(root, base) {
            Ok(files) => {
                for file in files {
                    println!("file {file}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("clm affected: {e}");
                ExitCode::from(e.exit_code())
            }
        };
    }
    let all = match clm_config::load(Some(root), &clm_config::process_env) {
        Ok(settings) => settings.affected.value.map(|a| a.all).unwrap_or_default(),
        Err(e) => {
            let v = settings_violation(e, "check.affected/affected-all");
            eprintln!("clm affected: {}", clm_check::format(&v));
            return ExitCode::FAILURE;
        }
    };
    match clm_affected::affected(root, base, &all) {
        Ok(clm_affected::Selection::All(path)) => println!("all {path}"),
        Ok(clm_affected::Selection::Nodes { nodes, files }) => {
            for node in nodes {
                println!("node {node}");
            }
            for file in files {
                println!("file {file}");
            }
        }
        Err(e) => {
            eprintln!("clm affected: {e}");
            return ExitCode::from(e.exit_code());
        }
    }
    ExitCode::SUCCESS
}

/// `config`'s `node_size`: the budgets of a node that sets none.
fn node_size(root: &Path) -> Result<clm_check::Budgets, clm_check::Violation> {
    let n = clm_config::load(Some(root), &clm_config::process_env)
        .map_err(|e| settings_violation(e, "check/node-size"))?
        .node_size
        .value;
    Ok(clm_check::Budgets {
        spec_tokens: n.spec_tokens,
        code_tokens: n.code_tokens,
    })
}

/// `config`'s `node_size`, for `clm status` and `clm view spec` to mark the oversized
/// nodes (`oversized-is-a-state`); `None`, with `size not checked: <error>` on stderr, when
/// the settings can't be loaded.
fn budgets(root: &Path) -> Option<clm_check::Budgets> {
    node_size(root)
        .inspect_err(|v| eprintln!("size not checked: {}", v.message))
        .ok()
}

/// The budgets a commit's `clm.toml` holding `text` gives, for `clm view spec --at`: that
/// file alone, since the view describes the commit, not this clone (`view-at-rev`).
fn budgets_from(text: Option<&str>) -> Result<clm_check::Budgets, String> {
    let n = match text {
        None => clm_config::NodeSize::default(),
        Some(text) => {
            clm_config::node_size_from(text).map_err(|e| format!("clm.toml: {}", e.message))?
        }
    };
    Ok(clm_check::Budgets {
        spec_tokens: n.spec_tokens,
        code_tokens: n.code_tokens,
    })
}

/// `clm size [<node>]`: each node's line, or only `node`'s, with `config`'s `node_size` as
/// the budgets a node doesn't set.
fn size(root: &Path, node: Option<&str>) -> ExitCode {
    let defaults = match node_size(root) {
        Ok(defaults) => defaults,
        Err(v) => return report(&[v]),
    };
    // Advisory: the sizes of the nodes that loaded, whatever `clm check` would report.
    let (graph, _) = clm_model::load(root);
    if let Some(node) = node.filter(|n| !graph.nodes.contains_key(*n)) {
        return report(&[clm_check::Violation {
            path: Path::new("specs")
                .join(node.replace('.', "/"))
                .join("SPEC.md"),
            rule: "check/node-size",
            message: format!("unknown component `{node}`"),
        }]);
    }
    match clm_check::size(root, &graph, &clm_check::scan(root).req, defaults) {
        Ok(sizes) => {
            for s in sizes.iter().filter(|s| node.is_none_or(|n| s.node == n)) {
                println!("{s}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("check/node-size: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `clm capabilities [<slug>]`: the list or one capability's page, as text or JSON.
fn capabilities(root: &Path, slug: Option<String>, format: CapabilitiesFormat) -> ExitCode {
    let report = match clm_effort::capabilities(root) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("clm capabilities: {e}");
            return ExitCode::from(e.exit_code());
        }
    };
    let out = match &slug {
        None => match format {
            CapabilitiesFormat::Text => clm_effort::list_text(&report).join("\n"),
            CapabilitiesFormat::Json => serde_json::to_string_pretty(&report).unwrap_or_default(),
        },
        Some(slug) => {
            let Some(cost) = report.capabilities.iter().find(|c| c.slug == *slug) else {
                println!("no capability {slug}");
                return ExitCode::FAILURE;
            };
            match format {
                CapabilitiesFormat::Text => clm_effort::page_text(cost).join("\n"),
                CapabilitiesFormat::Json => serde_json::to_string_pretty(cost).unwrap_or_default(),
            }
        }
    };
    println!("{out}");
    ExitCode::SUCCESS
}

/// `clm workaround`: the body from stdin, the flag written, its path printed (`workaround-command`).
fn flag_workaround(root: &Path, requirement: &str, title: &str) -> ExitCode {
    let mut body = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut body) {
        eprintln!("clm workaround: reading the body: {e}");
        return ExitCode::FAILURE;
    }
    match clm_health::workaround(root, requirement, title, &body) {
        Ok(path) => {
            println!("{}", path.display());
            workarounds(root, requirement)
        }
        Err(e) => {
            eprintln!("clm workaround: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

/// After `clm workaround` wrote its flag: exit 3, warning, for each capability the requirement
/// serves that now has `[effort] workaround_flag` unsettled flags (`workaround-command`,
/// `redesign-before-workaround`); 0, warning, when they can't be counted.
fn workarounds(root: &Path, requirement: &str) -> ExitCode {
    let counted = clm_config::load(Some(root), &clm_config::process_env)
        .map_err(|e| e.to_string())
        .and_then(|s| {
            clm_health::crowded(root, requirement, s.effort.value.workaround_flag)
                .map_err(|e| e.to_string())
        });
    match counted {
        Ok(crowded) if crowded.is_empty() => ExitCode::SUCCESS,
        Ok(crowded) => {
            for (capability, n) in crowded {
                eprintln!(
                    "clm workaround: warning: {n} unsettled workarounds on {capability}: stop and \
                     ask whether to redesign it (redesign-before-workaround)"
                );
            }
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("clm workaround: warning: the workarounds weren't counted: {e}");
            ExitCode::SUCCESS
        }
    }
}

/// `clm capabilities decide`: append a `decided` record (`effort/decide-command`).
fn decide(root: &Path, slug: &str, value: RatingArg, decision: DecisionArg) -> ExitCode {
    decided(
        "capabilities",
        clm_effort::decide,
        root,
        slug,
        value,
        decision,
    )
}

/// `clm architecture decide`: append a `decided` record
/// (`effort/architecture-decide-command`).
fn decide_architecture(
    root: &Path,
    slug: &str,
    value: RatingArg,
    decision: DecisionArg,
) -> ExitCode {
    decided(
        "architecture",
        clm_effort::decide_architecture,
        root,
        slug,
        value,
        decision,
    )
}

type Decide = fn(
    &Path,
    &str,
    clm_ledger::Rating,
    clm_ledger::Decision,
    &dyn Fn(&str) -> Option<String>,
) -> Result<clm_effort::Recorded, clm_effort::Error>;

fn decided(
    command: &str,
    decide: Decide,
    root: &Path,
    slug: &str,
    value: RatingArg,
    decision: DecisionArg,
) -> ExitCode {
    let value = match value {
        RatingArg::High => clm_ledger::Rating::High,
        RatingArg::Medium => clm_ledger::Rating::Medium,
        RatingArg::Low => clm_ledger::Rating::Low,
    };
    let decision = match decision {
        DecisionArg::Keep => clm_ledger::Decision::Keep,
        DecisionArg::Cut => clm_ledger::Decision::Cut,
        DecisionArg::Redesign => clm_ledger::Decision::Redesign,
        DecisionArg::Accept => clm_ledger::Decision::Accept,
    };
    match decide(root, slug, value, decision, &clm_config::process_env) {
        Ok(done) => {
            for line in &done.lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        // As the page for an unknown slug says it.
        Err(e @ (clm_effort::Error::NoCapability(_) | clm_effort::Error::NoArchitecture(_))) => {
            println!("{e}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("clm {command} decide: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

/// `clm architecture [<slug>]`: the list or one entry's page, as text or JSON
/// (`effort/architecture-list`, `effort/architecture-page`, `effort/architecture-json`).
fn architecture(root: &Path, slug: Option<String>, format: CapabilitiesFormat) -> ExitCode {
    let list = match clm_effort::architecture_review(root) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("clm architecture: {e}");
            return ExitCode::from(e.exit_code());
        }
    };
    let out = match &slug {
        None => match format {
            CapabilitiesFormat::Text => clm_effort::architecture_text(&list).join("\n"),
            CapabilitiesFormat::Json => serde_json::to_string_pretty(&list).unwrap_or_default(),
        },
        Some(slug) => {
            let Some(entry) = list.iter().find(|c| c.slug == *slug) else {
                println!("no architectural constraint {slug}");
                return ExitCode::FAILURE;
            };
            match format {
                CapabilitiesFormat::Text => clm_effort::constraint_text(entry).join("\n"),
                CapabilitiesFormat::Json => serde_json::to_string_pretty(entry).unwrap_or_default(),
            }
        }
    };
    println!("{out}");
    ExitCode::SUCCESS
}

/// One line per violation: a TOML error's first line (where) and last line (what), without
/// the source snippet between them.
fn one_line(message: &str) -> String {
    let mut lines = message.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.next().unwrap_or_default();
    match lines.next_back() {
        Some(last) => format!("{first}: {last}"),
        None => first.to_string(),
    }
}

/// `clm view`'s result: the view on `out` and exit 0, or exit 1 with the spec graph's
/// violations on `out` as [`report`] prints them, or with a revision that couldn't be read
/// as one line on `err` and nothing on `out`.
fn print_view(
    result: Result<String, clm_view_repo::Error>,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> ExitCode {
    match result {
        Ok(text) => match out.write_all(text.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Err(clm_view_repo::Error::Violations(violations)) => report_to(&violations, out),
        Err(clm_view_repo::Error::At(why)) => {
            let _ = writeln!(err, "{why}");
            ExitCode::FAILURE
        }
    }
}

fn report(violations: &[clm_check::Violation]) -> ExitCode {
    report_to(violations, &mut std::io::stdout())
}

/// One line per violation on `out`, exiting 1.
fn report_to(violations: &[clm_check::Violation], out: &mut dyn std::io::Write) -> ExitCode {
    for v in violations {
        let _ = writeln!(out, "{}", clm_check::format(v));
    }
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_prints_on_stdout_and_exits_0() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = print_view(Ok("a view\n".into()), &mut out, &mut err);
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(out, b"a view\n");
        assert!(err.is_empty());
    }
}

#[cfg(test)]
mod version;
