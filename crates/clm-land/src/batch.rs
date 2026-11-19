//! Lands taken together (`land/batched-lands`, `land/failed-batch`): the land that takes the
//! land lock lands the lands waiting for it on one run of `[land] check`, each as its own merge
//! commit, in one push, and appends the records of all of them before it lets the lock go.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clm_ledger::{Basis, Body, Entry, Fold, LandKind, Recheck, TimedStep};

use crate::land::{
    ON_MERGE, Pushed, changes_added, cleanup, free_build_target, propose_tips, push_main,
    release_claims, specs_changed_on_both, unconflicted_tree,
};
use crate::lock::{self, Held, Rank};
use crate::scratch::Scratch;
use crate::{CheckHow, Error, Output, Repo, Selected, Selection, Verdict};

/// Up to this many suspects each land alone; more are halved (`land/failed-batch`).
const ALONE_UP_TO: usize = 4;

/// What the land that holds the lock hands the batch.
pub(crate) struct Ctx<'a, 'r> {
    pub(crate) repo: &'a Repo<'r>,
    pub(crate) exe: &'a Path,
    pub(crate) places: &'a Path,
    pub(crate) target: &'a Path,
    pub(crate) check: &'a str,
    pub(crate) branch: &'a str,
    pub(crate) held: &'a Held,
    /// The holder's own effort records, read before its first attempt.
    pub(crate) effort: &'a [Body],
    /// The rank the check lock is taken in.
    pub(crate) rank: Rank,
}

/// What the lands taken together came to for the land that holds the lock.
pub(crate) enum Outcome {
    /// The holder's land is done: print this.
    Landed(Output),
    /// Nothing was done: the holder lands as before.
    Proceed,
    /// Something was pushed or marked, so the holder starts its attempt afresh, saying these
    /// lines.
    Restart(Vec<String>),
}

/// What a judged merge commit is: the land's kind, fingerprint and change IDs.
pub(crate) struct Judged {
    pub(crate) fingerprint: String,
    pub(crate) kind: LandKind,
    pub(crate) review: bool,
    pub(crate) changes: Vec<String>,
}

/// Judges `head`'s merge onto `first`, which is checked out in `dir` as `commit`: the
/// fingerprint taken from the commit's two parents, approved under the policy at the first
/// parent, a request for changes holding it, and the gate run on the commit against the first
/// parent. With `derived`, `specs/` is proved derived too.
#[allow(clippy::too_many_arguments)]
pub(crate) fn judge(
    repo: &Repo,
    exe: &Path,
    dir: &Scratch,
    fold: &Fold,
    branch: &str,
    first: &str,
    head: &str,
    signed: bool,
    derived: bool,
) -> Result<Judged, Error> {
    let fingerprint = repo.approval.fingerprint(first, head)?;
    if let Some(why) = clm_approval::held(fold, branch, &fingerprint) {
        return Err(Error::Refused(why));
    }
    let policy = repo.approval.policy(first)?;
    let approvals = repo.approval.approval(
        fold,
        &fingerprint,
        head,
        signed.then_some(first),
        policy.other,
    )?;
    let hint = crate::land::approval_hint(&approvals, branch, &policy);
    let verdict = repo.gate_on(exe, dir.path(), first, approvals.accepted, &hint)?;
    if derived {
        crate::derived::check(repo, first, head, dir.path())?;
    }
    let (kind, review) = match verdict {
        Verdict::Code => (LandKind::Code, false),
        Verdict::Spec { review } => (LandKind::Spec, review),
    };
    let changes = match kind {
        LandKind::Spec => repo.approval.changes(head)?,
        LandKind::Code => Vec::new(),
    };
    Ok(Judged {
        fingerprint,
        kind,
        review,
        changes,
    })
}

/// A land in a batch: its branch and, but for the holder's, the ticket it waits with.
#[derive(Clone)]
struct Wanted {
    branch: String,
    ticket: Option<String>,
}

impl Wanted {
    fn holder(&self) -> bool {
        self.ticket.is_none()
    }
}

/// What a land's part of the check's selection is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    /// The files whose tests it runs.
    Files(BTreeSet<String>),
    /// An `all` line: every test, the line as `clm affected` printed it.
    All(String),
    /// `clm affected` failed: every test, with no selection at all.
    Failed,
}

impl Part {
    fn of(selected: Selected) -> Part {
        match selected {
            Selected::Nothing => Part::Files(BTreeSet::new()),
            Selected::Failed => Part::Failed,
            Selected::Tests(selection) => {
                let text = std::fs::read_to_string(&selection.path).unwrap_or_default();
                let mut files = BTreeSet::new();
                for line in text.lines() {
                    let line = line.trim();
                    match line.split_once(char::is_whitespace) {
                        Some(("file", path)) => {
                            files.insert(path.trim().to_string());
                        }
                        _ if line == "all" || line.starts_with("all ") => {
                            return Part::All(line.to_string());
                        }
                        _ => {}
                    }
                }
                Part::Files(files)
            }
        }
    }

    /// The union of `self` and `other`.
    fn add(&mut self, other: &Part) {
        if matches!(self, Part::Failed) {
            return;
        }
        match other {
            Part::Failed => *self = Part::Failed,
            Part::All(line) => {
                if !matches!(self, Part::All(_)) {
                    *self = Part::All(line.clone());
                }
            }
            Part::Files(theirs) => {
                if let Part::Files(mine) = self {
                    mine.extend(theirs.iter().cloned());
                }
            }
        }
    }

    /// Whether the part holds a test that one of `paths` selected: every test does.
    fn holds(&self, paths: &[String]) -> bool {
        match self {
            Part::Files(files) => files
                .iter()
                .any(|f| paths.iter().any(|p| Path::new(p) == Path::new(f))),
            Part::All(_) | Part::Failed => true,
        }
    }
}

/// A land of the chain with the variables its check runs under and what it warns of.
type Built = (Member, Vec<(String, String)>, Vec<String>);

/// A land of the chain.
struct Member {
    wanted: Wanted,
    head: String,
    on_origin: bool,
    /// Its merge commit as the chain was built to be gated; the pushed one differs in its
    /// message alone.
    commit: String,
    tree: String,
    judged: Judged,
    /// What it would have run landing alone.
    own: Part,
}

/// The merge commits of lands taken together, in order, on the `main` fetched.
struct Chain {
    base: String,
    members: Vec<Member>,
    fold: Fold,
    signed: bool,
    vars: Vec<(String, String)>,
    command: String,
    warnings: Vec<String>,
}

/// The commits that are pushed, and the code key their check runs on.
struct Finals {
    commits: Vec<String>,
    key: String,
}

/// The chain a check passed on, held in the land tree, and how it was checked: the check
/// the lands taken together keep when `main` moves (`land/batch-keeps-its-check`).
struct Kept {
    chain: Chain,
    finals: Finals,
    /// The land tree, holding the last of `finals`' commits.
    dir: Scratch,
    /// The seconds and steps of every run of the check that went into it.
    seconds: u64,
    steps: Vec<TimedStep>,
    files: Option<u64>,
    recheck: Recheck,
}

/// What a round of the kept check came to.
enum Next {
    /// The chain was built on the new `main` and checked.
    Round(Box<Kept>),
    /// The lands go on as lands never taken together.
    GiveWay,
    /// The check failed, and `failed-batch` has run.
    Failed,
}

struct Batch<'c, 'a, 'r> {
    ctx: &'c Ctx<'a, 'r>,
    warnings: Vec<String>,
    /// Whether anything reached `main`.
    pushed: bool,
    /// The holder's land, once it is on `main`.
    landed: Option<Output>,
    /// Whether the holder is left to land alone, as a suspect or as a land left out.
    moved: bool,
}

/// Lands the waiting lands with the holder's, when there are any it can take.
pub(crate) fn run(ctx: &Ctx) -> Result<Outcome, Error> {
    // A land marked to land alone takes no other with it (`land/failed-batch`).
    if ctx.held.alone() {
        return Ok(Outcome::Proceed);
    }
    let waiting = lock::waiting_lands(ctx.target, ctx.held)?;
    let mut wanted = vec![Wanted {
        branch: ctx.branch.to_string(),
        ticket: None,
    }];
    for w in waiting {
        if !wanted.iter().any(|x| x.branch == w.branch) {
            wanted.push(Wanted {
                branch: w.branch,
                ticket: Some(w.ticket),
            });
        }
    }
    if wanted.len() < 2 {
        return Ok(Outcome::Proceed);
    }
    let mut batch = Batch {
        ctx,
        warnings: Vec::new(),
        pushed: false,
        landed: None,
        moved: false,
    };
    let Some(chain) = batch.build(&wanted)? else {
        return Ok(Outcome::Proceed);
    };
    // The holder's own land first, and with another: else it lands as before.
    if chain.members.len() < 2 || !chain.members[0].wanted.holder() {
        return Ok(Outcome::Proceed);
    }
    batch.together(chain)?;
    Ok(match batch.landed {
        Some(mut out) => {
            let mut warnings = batch.warnings;
            warnings.append(&mut out.warnings);
            out.warnings = warnings;
            Outcome::Landed(out)
        }
        None if batch.pushed || batch.moved || !batch.warnings.is_empty() => {
            Outcome::Restart(batch.warnings)
        }
        None => Outcome::Proceed,
    })
}

impl Batch<'_, '_, '_> {
    /// The chain of `wanted` on the `main` fetched now, with each land left out that its own
    /// attempt, or the lands before it, would refuse: none when there is no land.
    fn build(&self, wanted: &[Wanted]) -> Result<Option<Chain>, Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        repo.fetch()?;
        let base = repo
            .rev(repo.base())?
            .ok_or_else(|| Error::Refused(format!("no `{}` to land on", repo.base())))?;
        let fold = clm_ledger::fold(&repo.root)?;
        let signed = repo.approval.signed(&base)?;
        if signed {
            repo.approval.require_key()?;
        }
        let dir = Scratch::land_tree(repo, ctx.places, &base)?;
        let mut members: Vec<Member> = Vec::new();
        let mut warnings = Vec::new();
        let mut tip = base.clone();
        let mut vars: Option<Vec<(String, String)>> = None;
        for w in wanted {
            // A land its own attempt or the ones before it would refuse waits as before.
            let Ok((member, own_vars, notes)) =
                self.member(&dir, &fold, &base, &tip, w, signed, vars.as_deref())
            else {
                continue;
            };
            tip.clone_from(&member.commit);
            vars.get_or_insert(own_vars);
            for note in notes {
                if !warnings.contains(&note) {
                    warnings.push(note);
                }
            }
            members.push(member);
        }
        let Some(vars) = vars else {
            return Ok(None);
        };
        let command = crate::checked_command(ctx.check, &vars);
        Ok(Some(Chain {
            base,
            members,
            fold,
            signed,
            vars,
            command,
            warnings,
        }))
    }

    /// `w` merged onto `tip`, gated and judged there: what its own attempt does before its
    /// check, with the gate on its merge commit against `tip` (`land/batched-lands`).
    #[allow(clippy::too_many_arguments)]
    fn member(
        &self,
        dir: &Scratch,
        fold: &Fold,
        base: &str,
        tip: &str,
        w: &Wanted,
        signed: bool,
        vars0: Option<&[(String, String)]>,
    ) -> Result<Built, Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        let (head, on_origin) = repo.approval.head(&w.branch, true)?;
        crate::own_files::check(repo, &w.branch, base, &head)?;
        crate::deleted_tests::check(repo, Some(&w.branch), base, &head)?;
        if w.branch.starts_with("spec/") {
            let files = clm_gate::files_outside_spec_paths(&repo.root, base, &head)
                .map_err(Error::Refused)?;
            if !files.is_empty() {
                return Err(Error::Refused(format!(
                    "a spec branch holds only spec paths; {} changes {}",
                    w.branch,
                    files.join(", ")
                )));
            }
        }
        let notes = crate::verify_stop::check(repo, base, &head, fold)?;
        let tree = unconflicted_tree(repo, tip, &head)?
            .ok_or_else(|| Error::Refused("the branch conflicts with main".into()))?;
        let merge_base = repo.git(&["merge-base", tip, &head])?;
        if specs_changed_on_both(repo, &merge_base, tip, &head)? {
            return Err(Error::Refused(
                "`specs/` changed on both the branch and main".into(),
            ));
        }
        // A land that checks under other variables checks alone (`land/check-env`).
        let vars = repo.env_at(&tree)?;
        if vars0.is_some_and(|v| v != vars.as_slice()) {
            return Err(Error::Refused("its check environment differs".into()));
        }
        let message = format!("Merge {} into main", w.branch);
        let commit = repo.git(&["commit-tree", &tree, "-p", tip, "-p", &head, "-m", &message])?;
        dir.checkout(&commit)?;
        let judged = judge(
            repo, ctx.exe, dir, fold, &w.branch, tip, &head, signed, true,
        )?;
        // What it would run landing alone: the readers of what differs on checked code, the
        // tests the branch can break on the rest.
        let command = crate::checked_command(ctx.check, &vars);
        let alone_checked = match unconflicted_tree(repo, base, &head)? {
            Some(alone) => {
                let key = clm_ledger::code_key(&repo.root, &alone)?;
                crate::land::checked_with(fold, &key, &command)
            }
            None => false,
        };
        let own = Part::of(repo.selection(ctx.exe, dir.path(), tip, alone_checked));
        let member = Member {
            wanted: w.clone(),
            head,
            on_origin,
            commit,
            tree,
            judged,
            own,
        };
        Ok((member, vars, notes))
    }

    /// The commits that are pushed: the chain's trees with the lines naming their changes and,
    /// for a chain of more than one, the code key its check runs on, computed from the trees
    /// before any commit is made.
    fn finals(&self, chain: &Chain) -> Result<Finals, Error> {
        let repo = self.ctx.repo;
        let last = chain.members.last().expect("a chain holds a land");
        let key = clm_ledger::code_key(&repo.root, &last.tree)?;
        let mut commits: Vec<String> = Vec::new();
        for m in &chain.members {
            let prev = commits.last().unwrap_or(&chain.base).clone();
            let mut message = format!("Merge {} into main", m.wanted.branch);
            let change = m
                .judged
                .changes
                .first()
                .filter(|_| m.judged.kind == LandKind::Spec);
            if let Some(id) = change {
                message.push_str(&format!("\n\nChange: {id}"));
            }
            if chain.members.len() > 1 {
                message.push_str(if change.is_some() { "\n" } else { "\n\n" });
                message.push_str(&format!("Checked: {key}"));
            }
            crate::end_with_gate_rules(&mut message);
            commits.push(repo.git(&[
                "commit-tree",
                &m.tree,
                "-p",
                &prev,
                "-p",
                &m.head,
                "-m",
                &message,
            ])?);
        }
        Ok(Finals { commits, key })
    }

    /// Runs the check on the chain once and lands it, or finds who broke it.
    fn together(&mut self, chain: Chain) -> Result<(), Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        let finals = self.finals(&chain)?;
        let tip = finals.commits.last().expect("a chain holds a land").clone();
        let dir = Scratch::land_tree(repo, ctx.places, &tip)?;
        let (parts, union) = self.selection(&chain, &dir)?;
        let selection = match &union {
            Part::Failed => None,
            part => Some(write_selection(repo, part)?),
        };
        let mut vars: Vec<(&str, &Path)> = Vec::new();
        if let Some(sel) = &selection {
            vars.push(("CLM_SELECTION", sel.path.as_path()));
        }
        vars.push(("CLM_LAND_DIR", ctx.target));
        let slot = repo.slot(ctx.branch)?;
        let run = repo.run_project_check(
            dir.path(),
            ctx.check,
            &vars,
            CheckHow {
                env: &chain.vars,
                log: None,
                slot,
                rank: Some(ctx.rank),
            },
        )?;
        if !run.passed {
            // A signal, not an exit, is the holder's own failure.
            if run.code.is_none() {
                return Err(Error::Refused(format!("`{}` {ON_MERGE}", ctx.check)));
            }
            return self.failed(chain, &parts, Some(&run));
        }
        match repo.clm_check(ctx.exe, dir.path(), true, ON_MERGE) {
            Ok(()) => {}
            Err(Error::Refused(_)) => return self.failed(chain, &parts, None),
            Err(e) => return Err(e),
        }
        let files = match &union {
            Part::Files(files) => Some(files.len() as u64),
            _ => None,
        };
        let kept = Kept {
            chain,
            finals,
            dir,
            seconds: run.seconds,
            steps: run.steps,
            files,
            recheck: Recheck::Selected,
        };
        self.push(kept)
    }

    /// Each land's part of the selection (what it would have run alone, with what it and each
    /// other land can both break), and the union the check runs.
    fn selection(&self, chain: &Chain, dir: &Scratch) -> Result<(Vec<Part>, Part), Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        let n = chain.members.len();
        let mut parts: Vec<Part> = chain.members.iter().map(|m| m.own.clone()).collect();
        for i in 0..n {
            for j in (i + 1)..n {
                let both = Part::of(repo.overlap(
                    ctx.exe,
                    dir.path(),
                    &chain.members[i].head,
                    &chain.members[j].head,
                ));
                parts[i].add(&both);
                parts[j].add(&both);
            }
        }
        let mut union = Part::Files(BTreeSet::new());
        for part in &parts {
            union.add(part);
        }
        Ok((parts, union))
    }

    /// The check failed on the chain: nothing is pushed, the holder says so, and the lands go
    /// on as `failed-batch` says (`land/failed-batch`).
    fn failed(
        &mut self,
        chain: Chain,
        parts: &[Part],
        run: Option<&crate::runner::CheckRun>,
    ) -> Result<(), Error> {
        let paths: Vec<String> = match run {
            Some(run) if run.step.is_some() => run
                .steps
                .last()
                .map(|s| s.selected_by.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let mut suspects: Vec<usize> = (0..parts.len())
            .filter(|&i| !paths.is_empty() && parts[i].holds(&paths))
            .collect();
        if suspects.is_empty() {
            suspects = (0..parts.len()).collect();
        }
        let others: Vec<Wanted> = (0..parts.len())
            .filter(|i| !suspects.contains(i))
            .map(|i| chain.members[i].wanted.clone())
            .collect();
        let blamed: Vec<Wanted> = suspects
            .iter()
            .map(|&i| chain.members[i].wanted.clone())
            .collect();
        let next = if blamed.len() <= ALONE_UP_TO {
            format!(
                "{} land together again and the {} that may have broken it each land alone",
                others.len(),
                blamed.len()
            )
        } else {
            format!(
                "{} land together again and the {} that may have broken it land in halves",
                others.len(),
                blamed.len()
            )
        };
        self.warnings.push(format!(
            "clm land: the check of the lands taken together failed; {next}"
        ));
        if !others.is_empty()
            && let Some(rest) = self.build(&others)?
        {
            self.together(rest)?;
        }
        if blamed.len() <= ALONE_UP_TO {
            for w in &blamed {
                match &w.ticket {
                    None => {}
                    Some(ticket) => lock::mark_alone(self.ctx.target, ticket)?,
                }
            }
            return Ok(());
        }
        let (first, second) = blamed.split_at(blamed.len().div_ceil(2));
        for half in [first, second] {
            if let Some(rest) = self.build(half)? {
                self.together(rest)?;
            }
        }
        Ok(())
    }

    /// The push turn, `main` unchanged, the records signed, one push, and every land's records
    /// before the lock is let go (`land/batched-lands`).
    fn push(&mut self, first: Kept) -> Result<(), Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        // The record the commits' `Checked:` lines name, before the push.
        repo.append(Body::Checked {
            code_key: first.finals.key.clone(),
            command: first.chain.command.clone(),
            basis: None,
        })?;
        let efforts: Vec<Vec<Body>> = first
            .chain
            .members
            .iter()
            .map(|m| {
                if m.wanted.holder() {
                    ctx.effort.to_vec()
                } else {
                    self.effort_of(&m.wanted.branch)
                }
            })
            .collect();
        let turn = lock::push(ctx.places, ctx.repo.err)?;
        let mut kept = first;
        // Rounds of the check kept on a `main` that moved, before the push that goes through
        // (`land/batch-keeps-its-check`).
        let mut rounds = 0;
        let entries = loop {
            repo.fetch()?;
            let now = repo.rev(repo.base())?;
            // Every pass counts, so a push that keeps being refused ends too.
            rounds += 1;
            if rounds > crate::land::ATTEMPTS {
                self.moved = true;
                return Ok(());
            }
            if now.as_deref() != Some(kept.chain.base.as_str()) {
                match self.keep(kept, now)? {
                    Next::Round(next) => kept = *next,
                    Next::GiveWay => {
                        self.moved = true;
                        return Ok(());
                    }
                    Next::Failed => return Ok(()),
                }
                continue;
            }
            let (chain, finals) = (&kept.chain, &kept.finals);
            let mut entries: Vec<Entry> = Vec::new();
            for (i, m) in chain.members.iter().enumerate() {
                entries.extend(efforts[i].iter().cloned().map(Into::into));
                let landed = Body::Landed {
                    commit: finals.commits[i].clone(),
                    branch: m.wanted.branch.clone(),
                    kind: m.judged.kind,
                    review: m.judged.review,
                    fingerprint: m.judged.fingerprint.clone(),
                    code_key: Some(finals.key.clone()),
                    changes: m.judged.changes.clone(),
                    check_seconds: (i == 0).then_some(kept.seconds),
                    steps: if i == 0 {
                        kept.steps.clone()
                    } else {
                        Vec::new()
                    },
                    recheck: Some(kept.recheck),
                    recheck_files: kept.files,
                };
                // Signed before the push, so a holder that can't sign lands no batch.
                entries.push(if chain.signed {
                    repo.approval.sign(landed, clm_approval::LAND)?.into()
                } else {
                    landed.into()
                });
            }
            let tip = finals.commits.last().expect("a chain holds a land");
            if push_main(repo, tip, &chain.base, ctx.branch)? == Pushed::Done {
                break entries;
            }
            // `main` moved again: the next round counts it, the commit last checked standing
            // for the one before.
        };
        let Kept {
            chain,
            finals,
            dir,
            seconds,
            steps,
            ..
        } = kept;
        let (finals, dir) = (&finals, &dir);
        drop(turn);
        self.pushed = true;
        let mut warnings = Vec::new();
        if let Some(land) = repo.settings.land.value.as_ref() {
            crate::cap::apply(ctx.target, land.cap_gb(), &mut warnings);
        }
        let appended = repo.append_all(entries)?;
        if let Some(last) = appended.last()
            && !last.pushed
        {
            warnings.push(format!(
                "the landed record isn't on origin yet: git push origin clm/ledger/{}",
                last.record.writer
            ));
        }
        let Some(at) = chain.members.iter().position(|m| m.wanted.holder()) else {
            return Ok(());
        };
        let m = &chain.members[at];
        let commit = finals.commits[at].clone();
        cleanup(repo, &m.wanted.branch, &m.head, m.on_origin, &mut warnings);
        let mut out = Output {
            warnings: chain.warnings.iter().cloned().chain(warnings).collect(),
            ..Default::default()
        };
        if m.wanted.branch.starts_with("spec/") {
            let added = changes_added(repo, &chain.base, &m.head, &m.judged.changes);
            let tips = propose_tips(repo);
            release_claims(repo, &commit, &added, &tips, &mut out.warnings);
        }
        let target_seconds = repo
            .settings
            .land
            .value
            .as_ref()
            .map(|l| l.target_seconds());
        if let Some(target) = target_seconds {
            let slow = (seconds > target).then(|| {
                crate::time_report::slowest(&steps, |path| {
                    repo.why(ctx.exe, dir.path(), &chain.base, path)
                })
            });
            let past = chain.fold.timed_lands(crate::time_report::LANDS);
            let over = slow.as_deref().map(|s| (s, steps.as_slice()));
            out.warnings
                .extend(crate::time_report::lines(seconds, target, over, &past));
        }
        out.lines.push(format!(
            "landed {commit} {}",
            kind_words(m.judged.kind, m.judged.review)
        ));
        if m.wanted.branch.starts_with("impl/") {
            free_build_target(repo, &m.wanted.branch, &mut out);
        }
        self.landed = Some(out);
        Ok(())
    }

    /// `main` is now at `now`, other than the base the chain was built on, inside the push
    /// turn: keeps the check that passed on the chain's last commit `P`, and checks again only
    /// what the lands taken and `main`'s new commits can both break, on the same lands merged
    /// onto `now` (`land/batch-keeps-its-check`).
    fn keep(&mut self, kept: Kept, now: Option<String>) -> Result<Next, Error> {
        let (ctx, repo) = (self.ctx, self.ctx.repo);
        let Some(now) = now else {
            return Ok(Next::GiveWay);
        };
        let p = kept.finals.commits.last().expect("a chain holds a land");
        let command = &kept.chain.command;
        // `main`'s new commits must have been checked whole.
        let fold = clm_ledger::fold(&repo.root)?;
        let base_key = clm_ledger::code_key(&repo.root, &now)?;
        if !crate::land::checked_with(&fold, &base_key, command) {
            return Ok(Next::GiveWay);
        }
        let selected = repo.overlap(ctx.exe, kept.dir.path(), p, &now);
        if matches!(selected, Selected::Failed) {
            return Ok(Next::GiveWay);
        }
        // The same lands in the same order, under the same check variables, on the new `main`.
        let wanted: Vec<Wanted> = kept
            .chain
            .members
            .iter()
            .map(|m| m.wanted.clone())
            .collect();
        let Some(chain) = self.build(&wanted)? else {
            return Ok(Next::GiveWay);
        };
        if chain.members.len() < wanted.len() || chain.vars != kept.chain.vars {
            return Ok(Next::GiveWay);
        }
        if chain.base != now {
            // `main` moved again meanwhile: the next round asks about the one it moved to.
            // Building the chain left another tree in the land tree.
            kept.dir.checkout(p)?;
            return Ok(Next::Round(Box::new(kept)));
        }
        let finals = self.finals(&chain)?;
        let tip = finals.commits.last().expect("a chain holds a land").clone();
        let dir = Scratch::land_tree(repo, ctx.places, &tip)?;
        let (mut seconds, mut steps) = (kept.seconds, kept.steps.clone());
        let (recheck, files, line) = match &selected {
            Selected::Tests(selection) => {
                let files = selection.files();
                let line = match files {
                    Some(n) => format!("re-check: overlap, {n} files"),
                    None => "re-check: overlap, every test".to_string(),
                };
                (Recheck::Overlap, files, line)
            }
            _ => {
                let merge_base = repo.git(&["merge-base", &now, p])?;
                let short = repo.git(&["rev-parse", "--short", &merge_base])?;
                let line = format!(
                    "re-check: none, the branch and main since {short} affect no test in common"
                );
                (Recheck::None, None, line)
            }
        };
        if !self.warnings.contains(&line) {
            self.warnings.push(line);
        }
        // Every land taken is a suspect of a check that fails here.
        let suspects = vec![Part::Failed; chain.members.len()];
        if let Selected::Tests(selection) = &selected {
            let vars: Vec<(&str, &Path)> = vec![
                ("CLM_SELECTION", selection.path.as_path()),
                ("CLM_LAND_DIR", ctx.target),
            ];
            let slot = repo.slot(ctx.branch)?;
            let run = repo.run_project_check(
                dir.path(),
                ctx.check,
                &vars,
                CheckHow {
                    env: &chain.vars,
                    log: None,
                    slot,
                    rank: Some(ctx.rank),
                },
            )?;
            if !run.passed {
                // A signal, not an exit, is the holder's own failure.
                if run.code.is_none() {
                    return Err(Error::Refused(format!("`{}` {ON_MERGE}", ctx.check)));
                }
                self.failed(chain, &suspects, None)?;
                return Ok(Next::Failed);
            }
            seconds += run.seconds;
            steps.extend(run.steps);
        }
        match repo.clm_check(ctx.exe, dir.path(), true, ON_MERGE) {
            Ok(()) => {}
            Err(Error::Refused(_)) => {
                self.failed(chain, &suspects, None)?;
                return Ok(Next::Failed);
            }
            Err(e) => return Err(e),
        }
        // The record the commits' `Checked:` lines name, before the push, unless it counts as
        // checked already.
        let fold = clm_ledger::fold(&repo.root)?;
        if !crate::land::checked_with(&fold, &finals.key, &chain.command) {
            repo.append(Body::Checked {
                code_key: finals.key.clone(),
                command: chain.command.clone(),
                basis: Some(Basis {
                    head: kept.finals.key.clone(),
                    base: base_key,
                }),
            })?;
        }
        // An earlier round that ran tests still stands when this one selected none.
        let (recheck, files) = if recheck == Recheck::None && kept.recheck == Recheck::Overlap {
            (kept.recheck, kept.files)
        } else {
            (recheck, files)
        };
        Ok(Next::Round(Box::new(Kept {
            chain,
            finals,
            dir,
            seconds,
            steps,
            files,
            recheck,
        })))
    }

    /// `branch`'s effort records, as `clm effort record --branch` would append them.
    fn effort_of(&self, branch: &str) -> Vec<Body> {
        let options = clm_effort::Options {
            branch: Some(branch.to_string()),
            ..Default::default()
        };
        clm_effort::unrecorded(&self.ctx.repo.root, &options, self.ctx.repo.env)
            .map(|read| read.bodies)
            .unwrap_or_default()
    }
}

/// The words after `landed <sha>` for a land of `kind`.
pub(crate) fn kind_words(kind: LandKind, review: bool) -> &'static str {
    match (kind, review) {
        (LandKind::Code, _) => "code",
        (LandKind::Spec, false) => "spec",
        (LandKind::Spec, true) => "spec review",
    }
}

/// `part` as the file `CLM_SELECTION` names.
fn write_selection(repo: &Repo, part: &Part) -> Result<Selection, Error> {
    let dir = tempfile::Builder::new()
        .prefix("land-sel-")
        .tempdir_in(crate::temp_parent(repo.env)?)?;
    let path: PathBuf = dir.path().join("selection");
    let text = match part {
        Part::All(line) => format!("{line}\n"),
        Part::Files(files) => files.iter().map(|f| format!("file {f}\n")).collect(),
        Part::Failed => String::new(),
    };
    std::fs::write(&path, text)?;
    Ok(Selection { _dir: dir, path })
}

/// The landed record of `branch`'s head on `main`'s first-parent chain that a land taken
/// together wrote, when the land that waited for it finds it (`land/batched-lands`).
pub(crate) struct Found {
    pub(crate) commit: String,
    pub(crate) kind: LandKind,
    pub(crate) review: bool,
    pub(crate) changes: Vec<String>,
}

/// The `landed` record of a land of `branch` at `head` that is on `main` now: its merge commit
/// has `head` as its second parent and `origin/main` holds it.
pub(crate) fn landed_record(
    repo: &Repo,
    fold: &Fold,
    branch: &str,
    head: &str,
) -> Result<Option<Found>, Error> {
    for record in fold.lands.values().flatten() {
        let Body::Landed {
            commit,
            branch: landed,
            kind,
            review,
            changes,
            ..
        } = &record.body
        else {
            continue;
        };
        if landed != branch {
            continue;
        }
        let Ok(second) = repo.git(&["rev-parse", &format!("{commit}^2")]) else {
            continue;
        };
        if second == head && clm_git::is_ancestor(&repo.root, commit, repo.base()).unwrap_or(false)
        {
            return Ok(Some(Found {
                commit: commit.clone(),
                kind: *kind,
                review: *review,
                changes: changes.clone(),
            }));
        }
    }
    Ok(None)
}
