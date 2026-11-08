//! The lands of `main` and what the ledger says of them: the last verified commit, the last
//! green one and the unverified lands (`verify`'s Design, "Since when").

use clm_ledger::{Body, Fold, Record};

use crate::{Ctx, Error};

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
        self.newest(commit).map(|r| {
            matches!(r.body, Body::Verified { passed: true, .. })
        })
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
