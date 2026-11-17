use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod closed_stdout;
mod none_remote;
mod stop_signal;
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
enum MachineFormat {
    Text,
    Json,
    Ansi,
}

#[derive(Subcommand)]
enum Command {
    Check(clm_check::CheckArgs),
    Coverage(clm_check::CoverageArgs),
    Pending(clm_view_repo::PendingArgs),
    Size(clm_check::SizeArgs),
    /// Apply a change's deltas to specs/, all or nothing.
    Apply(clm_apply::ApplyArgs),
    /// Carry out the renames a proposed change records: its deltas, applied, with every
    /// reference re-pointed, and the change archived as done.
    Rename(clm_rename::RenameArgs),
    /// Move a done or abandoned change to changes/archive/.
    Archive(clm_apply::ArchiveArgs),
    Backlog(clm_check::BacklogArgs),
    Status(clm_view_repo::StatusArgs),
    View(clm_view_repo::ViewArgs),
    /// Print the context bundle an agent needs for a component, its ancestors' specs among it.
    Context(clm_context::Args),
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
    Hashes(clm_apply::HashesArgs),
    /// Decide whether HEAD may merge into a base ref; exit 1 to block.
    Gate(clm_gate::Args),
    /// List the components ready to build from origin/main, most urgent first.
    Next(clm_orchestrator::NextArgs),
    /// Claim a component by pushing its impl/<component> branch, or a backlog item by pushing
    /// propose/<slug>.
    Claim(clm_orchestrator::ClaimArgs),
    /// Claim a component, or pick up its unfinished build, write the prompt for a session that
    /// builds and lands its pending requirements to `.clm/prompts/<component>.md`, and print a
    /// launcher naming that file. With `--finish`, free the component's build place instead and,
    /// once its branch has landed, remove its clean worktrees under `.claude/worktrees/`.
    Run(clm_run::RunArgs),
    /// List the changes that hold or propose backlog items, and the live backlog claims.
    Proposals(clm_orchestrator::ProposalsArgs),
    /// Group the untaken backlog items into units of design work, beside the spec work on
    /// origin and the items waiting on the human, most urgent first.
    Triage(clm_triage::TriageArgs),
    /// Match the host's open issues and pull requests to the backlog: for each issue, whether
    /// an item takes it in, a landed change or fix resolved it, or it is new.
    Intake(clm_triage::IntakeArgs),
    /// Claim backlog items and write the prompt for a session that designs one change from
    /// them unattended, printing a launcher that names it, holding one of the clone's build
    /// places.
    Draft(clm_triage::DraftArgs),
    /// Run the clone's build loop: decide every step of draining the queue and print the
    /// actions the session carries out with the agent harness's subagents.
    Build {
        #[command(subcommand)]
        action: BuildAction,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long, global = true)]
        root: Option<PathBuf>,
    },
    /// Remove merged work, report work that isn't safe on origin, and say whether the
    /// session can end.
    #[command(name = "wrap-up")]
    WrapUp(clm_checkpoint::WrapUpArgs),
    /// Rebase the branch onto origin/main, regenerate applied specs and run `clm check`;
    /// exit 1 with what is left to resolve.
    Rebase(clm_sync::RebaseArgs),
    /// Push a snapshot of the spec work on a `spec/*` branch to origin as `autosync/<branch>`.
    Autosync(clm_sync::AutosyncArgs),
    /// Run the repository's `[build] setup`, which makes a fresh worktree ready to run its tests.
    Setup(clm_run::SetupArgs),
    /// Print the path of a skill's SKILL.md in the pinned superpowers copy builds follow.
    Superpowers(clm_run::SuperpowersArgs),
    /// Run a Claude Code hook on the JSON event on stdin (used by the plugin).
    Hook {
        #[command(subcommand)]
        hook: Hook,
    },
    /// Report a defect in CLM itself, the report on stdin, as the `defects` setting says:
    /// a backlog item, an issue on clunny/clm, or nothing.
    Defect(clm_hooks::DefectArgs),
    /// Read CLM's settings for the CLM repository found from the current directory.
    Config(clm_config::Args),
    /// A branch's pull request on the host the `host` setting names.
    Pr(clm_host::PrArgs),
    /// File an issue where the host the `host` setting names keeps them and print its URL;
    /// without a host, print the report and exit 1.
    Issue(clm_host::IssueArgs),
    /// Contributions from people without write access, through the host adapter the `host`
    /// setting names: a fork's pull requests and issues.
    Contribution {
        #[command(subcommand)]
        contribution: Contribution,
    },
    /// Remote branches, through the host adapter the `host` setting names.
    Branch(clm_host::BranchArgs),
    /// The ledger of approvals, checks and lands: print it, audit `main` against it,
    /// or prune your own superseded records.
    Ledger(clm_ledger::LedgerArgs),
    // Their help is `clm-effort`'s, which its own call prints too.
    Effort(clm_effort::EffortArgs),
    Capabilities(clm_effort::CapabilitiesArgs),
    Architecture(clm_effort::ArchitectureArgs),
    /// Land a branch on `main`: gate it, merge it, check the merge, push it and record the
    /// land in the ledger; print `landed <sha> <kind>`.
    Land(clm_land::LandArgs),
    /// Run the project's slow tests after the land, at the moments `[verify]` names, and
    /// record the result in the ledger; print one line starting `verify `. Exits 0 when it
    /// passed, nothing was due or there was nothing to import; 1 when it failed or can't run;
    /// 2 when git fails; 3 when `--dry-run` finds nothing due; 4 when the result is pending.
    #[command(group(clap::ArgGroup::new("what").required(true).args(["due", "commit", "import", "bisect"])))]
    Verify {
        /// Whether a trigger has fired, and when one has, verify `origin/main`'s head.
        #[arg(long)]
        due: bool,
        /// With `--due`, an event that happened: `work-done`.
        #[arg(long, requires = "due", value_parser = ["work-done"])]
        event: Option<String>,
        /// With `--due`, only say whether a run is due.
        #[arg(long, requires = "due")]
        dry_run: bool,
        /// Verify one commit of `main`, or answer from the ledger.
        #[arg(long, value_name = "SHA")]
        commit: Option<String>,
        /// In `import` mode, record the pipeline's completed runs; run nothing.
        #[arg(long)]
        import: bool,
        /// Find the land that broke a red commit, and record it.
        #[arg(long, value_name = "SHA")]
        bisect: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    // Its help is `clm-approval`'s, which its own call prints too.
    Approve(clm_approval::ApproveArgs),
    /// Push a `main` landed with no remote, and the local ledgers, to a newly added `origin`.
    Reconnect(clm_land::ReconnectArgs),
    /// Set up CLM's tooling in the CLM repository found from the current directory, or at the
    /// top of the git repository when there is none, and print the steps only a human can do.
    Init(clm_init::InitArgs),
    /// Print what a change can break: `node <id>` and `file <path>` lines, or `all <path>`
    /// when every test must run.
    Affected(clm_affected::Args),
    /// Print the machine's CPU load, available memory and free disk, judged against what this
    /// clone's builds use; exits 0 whatever the states.
    Machine {
        /// The form: three text lines, the model as JSON, or the chart rows with ANSI colour.
        #[arg(long, value_enum, default_value = "text")]
        format: MachineFormat,
        /// The columns the chart rows fit.
        #[arg(long, default_value_t = 80)]
        width: usize,
        /// Hours after which an unmerged claim has lapsed.
        #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
        expiry: u64,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print one advisory report of the repository's health; exits 0 whatever it finds.
    Health(clm_health::HealthArgs),
    /// Flag that a requirement was hard to build: write `backlog/workaround-<component>-<slug>-<n>.md`
    /// with the situation from stdin and print its path. Commits nothing.
    Workaround(clm_health::WorkaroundArgs),
    /// List the public items of the workspace's Rust libraries and TypeScript and Python
    /// packages that nothing outside their own library or package uses; exit 1 when it lists any.
    Unreferenced(clm_unreferenced::Args),
    /// Print a compact map of the repository's code, for drafting its spec graph.
    Survey(clm_init::SurveyArgs),
    /// Print the agent harness this command runs under and what it can do.
    AgentHarness,
    /// Print CLM's guide: with no page, the index of its pages; with one, that page.
    Guide(clm_init::GuideArgs),
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
enum BuildAction {
    /// Begin or adopt the loop, and print the first actions.
    Start {
        /// The alias of the model the session runs on.
        #[arg(long)]
        session_model: String,
        /// Take the loop although another session's lease is live.
        #[arg(long)]
        take_over: bool,
    },
    /// Print the actions to take now.
    Step,
    /// Record the agent harness's task ID for a run the session launched.
    Launched { name: String, task_id: String },
    /// Record a run's return and print the actions to take now.
    Returned {
        name: String,
        /// The run's last line, as it stands.
        #[arg(long, allow_hyphen_values = true)]
        result: String,
        /// Why a run that didn't land ended.
        #[arg(long)]
        cause: Option<String>,
        /// When a `limit` lifts, as an RFC 3339 time.
        #[arg(long)]
        until: Option<String>,
    },
    /// Record that the human approved a fold batch with `clm approve`.
    Approved { batch: String },
    /// Record that the human cleared a condition (`disk`).
    Cleared { condition: String },
    /// Give up a component's build: delete its branch on origin, keep the local one as
    /// `released/<component>/<time>`, free its place and forget its run.
    Release { component: String },
    /// Wait until a step would act, renewing the lease, and print why it woke.
    Wait {
        /// Minutes between looks at `origin`: a positive number of at most 7.
        #[arg(long, default_value = "5")]
        interval: String,
    },
    /// Record a request to drain or to stop the loop.
    #[command(group(clap::ArgGroup::new("mode").required(true).args(["drain", "now"])))]
    Stop {
        #[arg(long, conflicts_with = "now")]
        drain: bool,
        #[arg(long)]
        now: bool,
    },
}

#[derive(Subcommand)]
enum Hook {
    /// PreToolUse: exit 2 with the reason when a Bash command would cut a release or push to
    /// `main`; print the `ask` decision when it would run `clm approve`, or file a CLM defect
    /// as an issue.
    Guard(clm_guard::GuardArgs),
    /// SessionStart: print `clm status` and the rule for reporting CLM defects when the
    /// session's cwd is a CLM repository.
    SessionStart(clm_hooks::HookArgs),
    /// Stop: push a snapshot of the spec work on a `spec/*` branch; silent, never fails.
    /// The `autosync` setting (`clm config get autosync`) turns it off.
    Autosync(clm_hooks::HookArgs),
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

fn main() -> ExitCode {
    closed_stdout::run(dispatch)
}

fn dispatch() -> ExitCode {
    let command = Cli::parse().command;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let required = |root: &Path| {
        clm_upgrade::require_current_in(&clm_init::migrations(), &clm_init::supersedes(), root)
    };
    if let Some(error) = upgrade_gate::blocked(&command, &cwd, required) {
        eprintln!("{error}");
        return ExitCode::from(4);
    }
    for line in none_remote::keep(&command, &cwd) {
        eprintln!("{line}");
    }
    match command {
        Command::Check(args) => {
            let inputs = clm_check::CheckInputs {
                detectors: &detectors,
                more: &clm_ledger::on_main,
            };
            ExitCode::from(clm_check::check_command(
                command_args(),
                &found_root(&args.root),
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
                &inputs,
            ))
        }
        Command::Coverage(args) => ExitCode::from(clm_check::coverage_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Pending(args) => ExitCode::from(clm_view_repo::pending_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Size(args) => ExitCode::from(clm_check::size_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
            &node_size,
        )),
        Command::Apply(args) => ExitCode::from(clm_apply::apply_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Rename(args) => ExitCode::from(clm_rename::rename_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Archive(args) => ExitCode::from(clm_apply::archive_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Backlog(args) => ExitCode::from(clm_check::backlog_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Status(args) => ExitCode::from(clm_view_repo::status_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
            &node_size,
        )),
        Command::View(args) => ExitCode::from(clm_view_repo::view_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
            &node_size,
            &budgets_from,
        )),
        Command::Context(args) => ExitCode::from(clm_context::command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
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
        Command::Hashes(args) => ExitCode::from(clm_apply::hashes_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Gate(args) => ExitCode::from(clm_gate::command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Next(args) => {
            let root = repo_root(args.root.clone());
            let max_builds = clm_config::load(Some(&root), &clm_config::process_env)
                .map(|s| s.max_builds.value)
                .map_err(|e| e.to_string());
            ExitCode::from(clm_orchestrator::next_command(
                command_args(),
                &root,
                &clm_config::process_env,
                max_builds,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Claim(args) => ExitCode::from(clm_orchestrator::claim_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Run(args) => {
            // Found first: finding the root may move the current directory there.
            let root = found_root(&args.root);
            ExitCode::from(clm_run::run_command(
                command_args(),
                &root,
                &current_dir(),
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Proposals(args) => ExitCode::from(clm_orchestrator::proposals_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Triage(args) => ExitCode::from(clm_triage::triage_command(
            command_args(),
            &found_root(&args.root),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Intake(args) => {
            let root = repo_root(args.root.clone());
            let host = clm_config::load(Some(&root), &clm_config::process_env)
                .map(|s| clm_host::adapter(s.host.value))
                .map_err(|e| e.to_string());
            ExitCode::from(clm_triage::intake_command(
                command_args(),
                &root,
                &cwd,
                &clm_config::process_env,
                host,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Draft(args) => {
            let root = repo_root(args.root.clone());
            let settings = clm_config::load(Some(&root), &clm_config::process_env)
                .map(|s| clm_triage::DraftSettings {
                    setup: s.build.value.map(|b| b.setup),
                    max_builds: s.max_builds.value,
                })
                .map_err(|e| e.to_string());
            ExitCode::from(clm_triage::draft_command(
                command_args(),
                &root,
                &cwd,
                &clm_config::process_env,
                settings,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Build { action, root } => build(repo_root(root), cwd, action),
        Command::WrapUp(args) => ExitCode::from(clm_checkpoint::wrap_up_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        // The settings are loaded from the CLM root of `HEAD` checked out in a temporary
        // worktree.
        Command::Rebase(args) => ExitCode::from(clm_sync::rebase_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
            sync_settings,
        )),
        Command::Autosync(args) => ExitCode::from(clm_sync::autosync_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Setup(args) => {
            let root = found_root(&args.root);
            ExitCode::from(clm_run::setup_command(
                command_args(),
                &root,
                &current_dir(),
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Superpowers(_) => {
            // It reads no repository, so the current directory is never moved.
            let cwd = current_dir();
            ExitCode::from(clm_run::superpowers_command(
                command_args(),
                &cwd,
                &cwd,
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Hook { hook } => {
            // A hook that can't read its event has nothing to do; it never breaks a session.
            let mut event = String::new();
            let _ = std::io::stdin().read_to_string(&mut event);
            // The hook's own arguments, after `clm hook <name>`.
            let hook_args = std::env::args_os().skip(3);
            let env = &clm_config::process_env;
            let (out, err) = (&mut Printed::Out, &mut Printed::Err);
            ExitCode::from(match hook {
                Hook::Guard(_) => clm_guard::guard_command(hook_args, &event, env, out, err),
                Hook::SessionStart(_) => clm_hooks::session_start_command(
                    hook_args,
                    clm_hooks::clm_root(&cwd),
                    &cwd,
                    &event,
                    env,
                    out,
                    err,
                ),
                Hook::Autosync(_) => clm_hooks::autosync_command(
                    hook_args,
                    clm_hooks::clm_root(&cwd),
                    &cwd,
                    &event,
                    env,
                    out,
                    err,
                ),
            })
        }
        Command::Defect(_) => {
            let mut report = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut report) {
                eprintln!("clm defect: reading the report: {e}");
                return ExitCode::FAILURE;
            }
            // The directory it runs in, not `dispatch`'s fallback: a report can't be filed
            // from a directory that is gone.
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    eprintln!("clm defect: current directory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            ExitCode::from(clm_hooks::defect_command(
                command_args(),
                clm_hooks::clm_root(&cwd),
                &cwd,
                &report,
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Ledger(args) => ExitCode::from(clm_ledger::ledger_command(
            command_args(),
            &found_root(args.root()),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Effort(args) => ExitCode::from(clm_effort::effort_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Capabilities(args) => ExitCode::from(clm_effort::capabilities_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Architecture(args) => ExitCode::from(clm_effort::architecture_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Land(args) => ExitCode::from(clm_land::land_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Verify {
            due,
            event,
            dry_run,
            commit,
            import,
            bisect,
            root,
        } => {
            let request = match (commit, bisect) {
                (Some(sha), _) => clm_verify::Request::Commit(sha),
                (_, Some(sha)) => clm_verify::Request::Bisect(sha),
                _ if import => clm_verify::Request::Import,
                _ => {
                    debug_assert!(due);
                    clm_verify::Request::Due {
                        work_done: event.is_some(),
                        dry_run,
                    }
                }
            };
            verify(&repo_root(root), &request)
        }
        Command::Approve(args) => ExitCode::from(clm_approval::approve_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Reconnect(args) => ExitCode::from(clm_land::reconnect_command(
            command_args(),
            &found_root(&args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Config(_) => {
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    eprintln!("clm config: current directory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            ExitCode::from(clm_config::command(
                command_args(),
                clm_hooks::clm_root(&cwd),
                &cwd,
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Pr(args) => ExitCode::from(clm_host::pr_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Issue(args) => ExitCode::from(clm_host::issue_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Contribution { contribution } => contribution_command(contribution),
        Command::Branch(args) => ExitCode::from(clm_host::branch_command(
            command_args(),
            &found_root(&args.root().map(PathBuf::from)),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Init(_) => ExitCode::from(clm_init::init_command(
            command_args(),
            clm_hooks::clm_root(&cwd),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::AgentHarness => {
            print!("{}", agent_harness_report(&clm_config::process_env));
            ExitCode::SUCCESS
        }
        Command::Guide(_) => ExitCode::from(clm_init::guide_command(
            command_args(),
            clm_hooks::clm_root(&cwd),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Affected(args) => ExitCode::from(clm_affected::command(
            command_args(),
            &repo_root(args.root),
            &clm_config::process_env,
            &|root, env| {
                clm_config::load(Some(root), env)
                    .map(|s| s.affected.value.map(|a| a.all).unwrap_or_default())
                    .map_err(|e| {
                        clm_check::format(&settings_violation(e, "check.affected/affected-all"))
                    })
            },
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Machine {
            format,
            width,
            expiry,
            root,
        } => machine(&repo_root(root), format, width, expiry),
        Command::Health(args) => {
            let root = repo_root(args.root);
            let settings = clm_config::load(Some(&root), &clm_config::process_env)
                .map(health_settings)
                .map_err(|e| settings_violation(e, "check.health/health-report"));
            ExitCode::from(clm_health::health_command(
                command_args(),
                &root,
                settings,
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Workaround(args) => {
            let root = repo_root(args.root);
            let workaround_flag = clm_config::load(Some(&root), &clm_config::process_env)
                .map(|s| s.effort.value.workaround_flag)
                .map_err(|e| e.to_string());
            ExitCode::from(clm_health::workaround_command(
                command_args(),
                &root,
                workaround_flag,
                &mut std::io::stdin(),
                &clm_config::process_env,
                &mut Printed::Out,
                &mut Printed::Err,
            ))
        }
        Command::Unreferenced(args) => ExitCode::from(clm_unreferenced::command(
            command_args(),
            &survey_root(args.root),
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
        Command::Survey(_) => ExitCode::from(clm_init::survey_command(
            command_args(),
            clm_hooks::clm_root(&cwd),
            &cwd,
            &clm_config::process_env,
            &mut Printed::Out,
            &mut Printed::Err,
        )),
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

/// `clm build`: the loop's own commands, each printing the actions it returns, one per line.
/// `cwd` is the directory `clm build` was started in, where `clm wrap-up` looks at the caller's
/// own work. Exit 0 on success, 1 when the work isn't possible and 2 when git fails.
fn build(root: PathBuf, cwd: PathBuf, action: BuildAction) -> ExitCode {
    let settings = match clm_config::load(Some(&root), &clm_config::process_env) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("clm build: settings: {e}");
            return ExitCode::from(1);
        }
    };
    let options = clm_loop::Options {
        clm: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("clm")),
        root,
        cwd,
        max_builds: settings.max_builds.value,
        agent_harness: clm_agent_harness::detect(clm_config::process_env),
    };
    let actions = match action {
        BuildAction::Start {
            session_model,
            take_over,
        } => clm_loop::start(&options, &session_model, take_over),
        BuildAction::Step => clm_loop::step(&options),
        BuildAction::Launched { name, task_id } => {
            clm_loop::launched(&options, &name, &task_id).map(|()| Vec::new())
        }
        BuildAction::Returned {
            name,
            result,
            cause,
            until,
        } => clm_loop::returned(&options, &name, &result, cause.as_deref(), until.as_deref()),
        BuildAction::Approved { batch } => clm_loop::approved(&options, &batch),
        BuildAction::Cleared { condition } => clm_loop::cleared(&options, &condition),
        BuildAction::Release { component } => clm_loop::release(&options, &component),
        BuildAction::Wait { interval } => match interval.parse::<f64>() {
            Ok(minutes) => clm_loop::wait(&options, minutes).map(|why| vec![why]),
            Err(_) => Err(clm_git::Failure::Refused(format!(
                "--interval `{interval}` isn't a number of minutes"
            ))),
        },
        BuildAction::Stop { drain, now: _ } => {
            let mode = if drain {
                clm_loop::StopMode::Drain
            } else {
                clm_loop::StopMode::Now
            };
            clm_loop::stop(&options, mode).map(|()| Vec::new())
        }
    };
    match actions {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => failed("build", &e),
    }
}

/// `repo_root`, except that with no CLM repository found it's the top of the git repository,
/// so `clm unreferenced` checks the whole workspace of a project without CLM's specs.
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

/// `clm <cmd>: <message>` on stderr, and the `Failure`'s exit code: 1 when `clm <cmd>`
/// refused, 2 when git failed.
fn failed(cmd: &str, e: &clm_git::Failure) -> ExitCode {
    eprintln!("clm {cmd}: {e}");
    ExitCode::from(e.exit_code())
}

/// The root a crate's call takes: the CLM repository found from the current directory, unless
/// the command's `--root` names one, which the call reads itself.
fn found_root(given: &Option<PathBuf>) -> PathBuf {
    match given {
        Some(_) => PathBuf::from("."),
        None => repo_root(None),
    }
}

/// The directory the command runs in, for a call that takes it; `.` when it can't be read.
fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// The command's own arguments, after `clm` and its name, as a crate's call parses them.
fn command_args() -> impl Iterator<Item = std::ffi::OsString> {
    std::env::args_os().skip(2)
}

/// The process's stdout or stderr for a crate's call, failing as `println!` and `eprintln!`
/// do: a write that fails panics with std's message, so `closed-stdout` holds.
enum Printed {
    Out,
    Err,
}

impl std::io::Write for Printed {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let (written, name) = match self {
            Printed::Out => (std::io::stdout().write_all(buf), "stdout"),
            Printed::Err => (std::io::stderr().write_all(buf), "stderr"),
        };
        if let Err(e) = written {
            panic!("failed printing to {name}: {e}");
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Printed::Out => std::io::stdout().flush(),
            Printed::Err => std::io::stderr().flush(),
        }
    }
}

/// `config`'s detectors for `clm check` on `root`, which skips those whose `requires` names
/// nothing there (`detector-requires-skip`); why the settings can't be loaded when they can't.
fn detectors(root: &Path) -> Result<Vec<clm_check::Detector>, clm_check::Violation> {
    let settings = clm_config::load(Some(root), &clm_config::process_env)
        .map_err(|e| settings_violation(e, "check/detectors"))?;
    Ok(settings
        .detectors
        .value
        .into_iter()
        .map(|d| clm_check::Detector {
            name: d.name,
            command: d.command,
            hint: d.hint,
            timeout: d.timeout,
            requires: d.requires,
        })
        .collect())
}

/// `clm verify`: the one `verify ...` line on stdout, notes on stderr, and the outcome's exit
/// code; 1 with the reason when it can't run, 2 when git fails.
fn verify(root: &Path, request: &clm_verify::Request) -> ExitCode {
    // The outcome is printed as soon as the result is recorded, before a mutation pass runs;
    // the pass's own notes follow. From then until the call returns, `SIGINT` and `SIGTERM`
    // stop the pass instead of the process (`verify-command`).
    let mut shown = 0;
    let mut catching = None;
    let announce = |outcome: &clm_verify::Outcome| {
        println!("{}", outcome.line);
        for note in &outcome.notes {
            eprintln!("clm verify: {note}");
        }
        shown = outcome.notes.len();
        // Only a run whose check passed goes on to the pass; `clm-verify` doesn't say whether
        // this one does, nor when its pass ends before the filing that follows it
        // (`backlog/clm-verify-can-t-tell-its-caller-when-the-mutation-pass-runs`).
        if outcome.line.starts_with("verify passed ") {
            catching = Some(stop_signal::install());
        }
    };
    let call = clm_verify::Call::standard(&stop_signal::STOP);
    let result = clm_verify::verify_with(root, request, &clm_config::process_env, &call, announce);
    let caught = catching.and_then(stop_signal::Installed::restore);
    match result {
        Ok(outcome) => {
            for note in &outcome.notes[shown..] {
                eprintln!("clm verify: {note}");
            }
            match caught {
                Some(signal) => ExitCode::from(128 + signal as u8),
                None => ExitCode::from(outcome.code),
            }
        }
        Err(clm_verify::Error::Refused(why)) => {
            eprintln!("clm verify: {why}");
            ExitCode::FAILURE
        }
        Err(clm_verify::Error::Git(e)) => {
            eprintln!("clm verify: {e}");
            ExitCode::from(2)
        }
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
    // `config`'s default for a `[land]` that sets no `check_target_seconds` (or no `[land]`).
    let land_target = settings
        .land
        .value
        .as_ref()
        .map_or(120, clm_config::Land::target_seconds);
    clm_health::Settings {
        land_check_target_seconds: land_target,
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

/// What `clm agent-harness` prints: the detected agent harness, then its abilities.
fn agent_harness_report(env: &dyn Fn(&str) -> Option<String>) -> String {
    let harness = clm_agent_harness::detect(env);
    let a = clm_agent_harness::abilities(harness);
    let yes = |b: bool| if b { "yes" } else { "no" };
    let mut out = format!("agent harness {}\n", harness.name());
    for (name, on) in [
        ("confirm", a.confirm),
        ("guard", a.guard),
        ("session-hooks", a.session_hooks),
        ("subagents", a.subagents),
        ("isolated-worktrees", a.isolated_worktrees),
        ("named-agents", a.named_agents),
        ("model-choice", a.model_choice),
    ] {
        out.push_str(&format!("{name} {}\n", yes(on)));
    }
    if let (true, Some(model)) = (a.model_choice, a.build_model) {
        out.push_str(&format!("build-model {model}\n"));
    }
    out
}

/// `clm machine`: the readings in the chosen form, with `config`'s `max_builds`; exit 0 whatever
/// the states, 1 when the settings are invalid or its output can't be written (`machine-command`).
fn machine(root: &Path, format: MachineFormat, width: usize, expiry: u64) -> ExitCode {
    use std::io::Write;
    let max_builds = match clm_config::load(Some(root), &clm_config::process_env) {
        Ok(s) => s.max_builds.value,
        Err(e) => {
            eprintln!("clm machine: settings: {e}");
            return ExitCode::FAILURE;
        }
    };
    let snapshot = clm_machine::read(root, max_builds, expiry, clm_git::now());
    let out = match format {
        MachineFormat::Text => clm_machine::text(&snapshot.report),
        MachineFormat::Json => format!("{}\n", clm_machine::json(&snapshot.report)),
        MachineFormat::Ansi => clm_machine::rows(&snapshot.report, &snapshot.history, width),
    };
    match std::io::stdout().write_all(out.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("clm machine: {e}");
            ExitCode::FAILURE
        }
    }
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

#[cfg(test)]
mod version;
