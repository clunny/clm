//! The lands of `main` and what the ledger says of them: the last verified commit, the last
//! green one and the unverified lands (`verify`'s Design, "Since when").

use std::collections::BTreeSet;

use clm_ledger::{Body, Fold, Record};

use crate::{Ctx, Error};

/// One record of a requirement's run of `mutated` records that counted it against it.
#[derive(Clone)]
pub(crate) struct Left {
    /// The commit the record names, a full ID.
    pub(crate) commit: String,
    /// Why its block's tool failed, when the record's `failed` names the requirement.
    pub(crate) why: Option<String>,
    /// The record's `ended`.
    pub(crate) ended: Option<String>,
}

pub(crate) struct State {
    /// The first-parent commits of the base, oldest first, full IDs.
    pub(crate) chain: Vec<String>,
    pub(crate) fold: Fold,
}

impl State {
    pub(crate) fn load(ctx: &Ctx) -> Result<Self, Error> {
        let out = ctx.git(&["rev-list", "--first-parent", "--reverse", &ctx.base])?;
        let chain: Vec<String> = out.lines().map(str::to_string).collect();
        if chain.is_empty() {
            return Err(Error::Refused(format!("{} holds no commit", ctx.base)));
        }
        Ok(State {
            chain,
            fold: clm_ledger::fold(&ctx.root)?,
        })
    }

    /// The full ID and the position in the chain of the commit `sha` names; refused when it
    /// names none or a commit that isn't a land of the base.
    pub(crate) fn resolve(&self, ctx: &Ctx, sha: &str) -> Result<(String, usize), Error> {
        let rev = format!("{sha}^{{commit}}");
        let full = ctx
            .git(&["rev-parse", "--verify", "--quiet", &rev])
            .map_err(|_| Error::Refused(format!("{sha} is not a commit")))?;
        match self.index(&full) {
            Some(end) => Ok((full, end)),
            None => Err(Error::Refused(format!(
                "{sha} isn't on the first-parent chain of {}, so it isn't a land of main",
                ctx.base
            ))),
        }
    }

    /// The newest `verified` record of `commit`.
    pub(crate) fn newest(&self, commit: &str) -> Option<&Record> {
        self.fold.verifications.get(commit).and_then(|v| v.first())
    }

    fn passed(&self, commit: &str) -> Option<bool> {
        self.newest(commit)
            .map(|r| matches!(r.body, Body::Verified { passed: true, .. }))
    }

    /// The position in the chain of `commit`, a full ID.
    pub(crate) fn index(&self, commit: &str) -> Option<usize> {
        self.chain.iter().position(|c| c == commit)
    }

    /// The newest commit before position `end` that a `verified` record names, whatever its
    /// result.
    pub(crate) fn last_verified(&self, end: usize) -> Option<usize> {
        self.chain[..end]
            .iter()
            .rposition(|c| self.passed(c).is_some())
    }

    /// The newest commit before position `end` whose newest record passed.
    pub(crate) fn last_green(&self, end: usize) -> Option<usize> {
        self.chain[..end]
            .iter()
            .rposition(|c| self.passed(c) == Some(true))
    }

    /// The commit a mutation pass on position `end` starts from: the newest commit before it
    /// that a `mutated` record names, else the commit before the unbroken run of commits
    /// ending at `end` whose `clm.toml` sets `[verify] mutate`, or none when the chain's
    /// first commit already does (`mutation-pass`).
    pub(crate) fn mutation_base(&self, ctx: &Ctx, end: usize) -> Result<Option<String>, Error> {
        let mutated = self.chain[..end]
            .iter()
            .rposition(|c| self.fold.mutations.contains_key(c));
        if let Some(i) = mutated {
            return Ok(Some(self.chain[i].clone()));
        }
        for i in (0..=end).rev() {
            if !sets_mutate(ctx, &self.chain[i]) {
                return Ok(Some(self.chain[i].clone()));
            }
        }
        Ok(None)
    }

    /// What the newest `mutated` record on the chain up to position `end` inclusive left, in
    /// its order, each with the commit its diff runs from: the `since` of the oldest record of
    /// the unbroken run of newest records whose `left` names it (`mutation-pass`). A commit's
    /// newest record is the newest by `at`, which the fold lists first.
    pub(crate) fn carried(&self, end: usize) -> Vec<(String, Option<String>)> {
        let newest: Vec<&Body> = self
            .newest_mutations(end)
            .into_iter()
            .map(|(_, b)| b)
            .collect();
        let Some(Body::Mutated { left, .. }) = newest.first() else {
            return Vec::new();
        };
        left.iter()
            .map(|id| {
                let mut base = None;
                for body in &newest {
                    match body {
                        Body::Mutated { left, since, .. } if left.contains(id) => {
                            base = since.clone()
                        }
                        _ => break,
                    }
                }
                (id.clone(), base)
            })
            .collect()
    }

    /// Each commit on the chain up to position `end` inclusive that a `mutated` record names,
    /// newest commit first, with the body of its newest record by `at`, which the fold lists
    /// first, so two passes on one commit count as one.
    pub(crate) fn newest_mutations(&self, end: usize) -> Vec<(&String, &Body)> {
        self.chain[..=end]
            .iter()
            .rev()
            .filter_map(|c| Some((c, &self.fold.mutations.get(c)?.first()?.body)))
            .collect()
    }

    /// For each requirement some commit's newest `mutated` record on the whole chain holds
    /// under `left`, in ID order, the run of those records, newest commit first, that count it
    /// against it: leaving out those that pass over it, and ending at the first that does
    /// neither (`files-unproven`). A requirement whose run is empty is left out.
    pub(crate) fn left_runs(&self) -> Vec<(String, Vec<Left>)> {
        let newest = self.newest_mutations(self.chain.len() - 1);
        let ids: BTreeSet<&String> = newest
            .iter()
            .flat_map(|(_, body)| match body {
                Body::Mutated { left, .. } => left.as_slice(),
                _ => &[],
            })
            .collect();
        ids.into_iter()
            .filter_map(|id| {
                let mut run = Vec::new();
                for (commit, body) in &newest {
                    let Body::Mutated {
                        left,
                        failed,
                        ended,
                        ..
                    } = body
                    else {
                        break;
                    };
                    if !left.contains(id) {
                        break;
                    }
                    let why = failed.get(id).cloned();
                    // Stopped, or failed because another requirement's block failed: the
                    // pass says nothing of this one.
                    let passes_over = why.is_none()
                        && match ended.as_deref() {
                            Some("stopped") => true,
                            Some(e) => e.starts_with("status ") && !failed.is_empty(),
                            None => false,
                        };
                    if !passes_over {
                        run.push(Left {
                            commit: commit.to_string(),
                            why,
                            ended: ended.clone(),
                        });
                    }
                }
                (!run.is_empty()).then(|| (id.clone(), run))
            })
            .collect()
    }

    /// The lands after the last verified commit before position `end`, up to `end`
    /// inclusive, oldest first, and that commit's position.
    pub(crate) fn unverified(&self, end: usize) -> (Option<usize>, &[String]) {
        let since = self.last_verified(end);
        (since, &self.chain[since.map_or(0, |s| s + 1)..=end])
    }

    /// The lands after the last verified commit of the whole chain, which is none when the
    /// head is verified, and that commit's position.
    pub(crate) fn unverified_lands(&self) -> (Option<usize>, &[String]) {
        let since = self.last_verified(self.chain.len());
        (since, &self.chain[since.map_or(0, |s| s + 1)..])
    }
}

/// Whether the `clm.toml` committed at `commit` parses as TOML and holds a non-empty string
/// at `[verify] mutate`.
fn sets_mutate(ctx: &Ctx, commit: &str) -> bool {
    let Ok(text) = ctx.git(&["show", &format!("{commit}:{}clm.toml", ctx.prefix)]) else {
        return false;
    };
    text.parse::<toml::Table>().is_ok_and(|table| {
        table
            .get("verify")
            .and_then(|v| v.get("mutate"))
            .and_then(toml::Value::as_str)
            .is_some_and(|s| !s.is_empty())
    })
}
