//! `clm ledger verify` (`ledger/verify`): `main`'s history audited against the records.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::{Body, Error, Fold, LandKind, Record, read};

/// One commit the ledger doesn't account for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// `ledger/unrecorded-land`, `ledger/unchecked-land`, `ledger/unsigned-land`,
    /// `ledger/understated-land`, `ledger/unrated-land` or `ledger/unapproved-land`.
    pub rule: &'static str,
    pub commit: String,
    pub message: String,
}

/// The release workflow's pushes start so (`host-ci-for-releases`): a merge commit when
/// `main` moved during the release, or the version bump itself when it fast-forwards.
const RELEASE: &str = "Release v";

/// The trailer of a commit made by hand under a decision (`verify-by-hand-merge`).
const BY_HAND: &str = "Clm-By-Hand";

/// The trailer naming the version of the gate's rules a land was judged by
/// (`land/gate-rules-trailer`).
const GATE_RULES: &str = "Gate-Rules";

/// Audits the first-parent chain of `tip` after `since` (by default, from the oldest commit
/// on it a `landed` record names): each commit no `landed` record names, merge or not
/// (`clm land` always merges, so a single-parent commit there was pushed around it), except
/// a release or a commit made by hand under an accepted decision, and each landed commit
/// whose land's code key no `checked` record names.
/// With `review`, each signed land's kind and whether it needed review are also recomputed
/// with the gate, under the rules in force when it landed, rather than trusted
/// (`verify-recomputes-review`). Doesn't fetch.
pub fn verify(
    root: &Path,
    tip: &str,
    since: Option<&str>,
    review: bool,
) -> Result<Vec<Finding>, Error> {
    let fold = crate::fold(root)?;
    let range = match since {
        Some(since) => format!("{since}..{tip}"),
        None => tip.to_string(),
    };
    let format =
        format!("--format=%H%x1f%P%x1f%(trailers:key={BY_HAND},valueonly,separator=%x1e)%x1f%s");
    let log = clm_git::git(root, &["log", "--first-parent", &format, &range])?;
    let mut chain: Vec<(&str, &str, &str, &str)> = log
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(4, '\x1f');
            Some((
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
            ))
        })
        .collect();
    if since.is_none() {
        // Newest first: keep up to and including the oldest landed commit, or none.
        match chain
            .iter()
            .rposition(|(sha, ..)| fold.lands.contains_key(*sha))
        {
            Some(oldest) => chain.truncate(oldest + 1),
            None => chain.clear(),
        }
    }
    let rules = if review {
        rules_in_force(root, &fold, tip)?
    } else {
        HashMap::new()
    };
    let mut findings = Vec::new();
    for (sha, parents, by_hand, subject) in chain.into_iter().rev() {
        if !by_hand.is_empty() && by_hand_allowed(root, sha, parents, by_hand)? {
            continue;
        }
        if let Some(lands) = fold.lands.get(sha) {
            // A land's code key is its record's, or, only for a record written before the
            // field, its legacy key, computed from the commit (`legacy-code-key`).
            let mut keys = BTreeSet::new();
            let mut keyless = false;
            for land in lands {
                match &land.body {
                    Body::Landed {
                        code_key: Some(key),
                        ..
                    } => {
                        keys.insert(key.clone());
                    }
                    _ => keyless = true,
                }
            }
            if keyless {
                keys.insert(crate::code_key::legacy_code_key(root, sha)?);
            }
            let keys: Vec<String> = keys.into_iter().collect();
            if !keys.iter().any(|key| is_checked(&fold, key)) {
                findings.push(Finding {
                    rule: "ledger/unchecked-land",
                    commit: sha.to_string(),
                    message: format!(
                        "landed {sha}: no `checked` record names its code key {}",
                        keys.join(", ")
                    ),
                });
            }
            // A commit of the audited range is on the whole chain, so it has its rules; were
            // one missing, the strictest rules this gate knows, never laxer ones.
            let rules = review.then(|| rules.get(sha).copied().unwrap_or(clm_gate::RULES));
            findings.extend(audit_signed(root, &fold, sha, parents, lands, rules)?);
        } else if !subject.starts_with(RELEASE) {
            findings.push(Finding {
                rule: "ledger/unrecorded-land",
                commit: sha.to_string(),
                message: format!(
                    "commit {sha} ({subject}) has no `landed` record: it is on {tip} \
                     without `clm land`"
                ),
            });
        }
    }
    Ok(findings)
}

/// The rules in force for each commit on the first-parent chain of `tip`, the whole chain
/// whatever `--since` says (`verify-recomputes-review`, `audit-by-the-rules-in-force`): the
/// higher of its own version and its floor, the highest own version, capped at the gate's
/// `RULES`, of an earlier commit whose first parent is signed and that a `landed` record
/// signed under that first parent's `docs/allowed_signers` names. A lander can't take older,
/// laxer rules by writing a lower trailer than `main` has already landed under, and one land
/// above `RULES` is one unrated land, not every land after it.
fn rules_in_force(root: &Path, fold: &Fold, tip: &str) -> Result<HashMap<String, u32>, Error> {
    // One NUL-ended entry per commit, its trailers unfolded, so no message text can split an
    // entry or forge another.
    let format = format!(
        "--format=%H%x1f%P%x1f%(trailers:key={GATE_RULES},valueonly,unfold,separator=%x1e)"
    );
    let log = clm_git::git(root, &["log", "-z", "--first-parent", &format, tip])?;
    let mut rules = HashMap::new();
    let mut floor = 1;
    for entry in log.split('\0').collect::<Vec<_>>().into_iter().rev() {
        let mut fields = entry.trim_start_matches('\n').splitn(3, '\x1f');
        let (Some(sha), Some(parents)) = (fields.next(), fields.next()) else {
            continue;
        };
        let own = own_rules(fields.next().unwrap_or(""));
        rules.insert(sha.to_string(), own.max(floor));
        let capped = own.min(clm_gate::RULES);
        if capped > floor && signed_land(root, fold, sha, parents)? {
            floor = capped;
        }
    }
    Ok(rules)
}

/// A commit's own version of the rules: the highest of its `Gate-Rules` trailers (separated
/// by `\x1e`) that, trimmed, is a whole number of at least 1, or 1 when there is none.
fn own_rules(trailers: &str) -> u32 {
    trailers
        .split('\x1e')
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
        .map(|value| value.parse::<u32>().unwrap_or(u32::MAX))
        .filter(|&version| version >= 1)
        .max()
        .unwrap_or(1)
}

/// Whether `sha`'s first parent is signed and a `landed` record naming `sha` is signed under
/// it, as `ledger/unsigned-land` judges a land.
fn signed_land(root: &Path, fold: &Fold, sha: &str, parents: &str) -> Result<bool, Error> {
    let (Some(lands), Some(first)) = (fold.lands.get(sha), parents.split_whitespace().next())
    else {
        return Ok(false);
    };
    if !crate::signed(root, first)? {
        return Ok(false);
    }
    for land in lands {
        if Rated::recorded(land).is_some() && signed_by(root, first, fold, land, "clm-land")? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether some `checked` record counts for `key`: one that checked the tree whole, or a
/// composed one whose `head` and `base` keys each have a `checked` record with its command
/// (`composed-check`).
fn is_checked(fold: &Fold, key: &str) -> bool {
    let ran = |key: &str, command: &str| {
        fold.checks.get(key).is_some_and(|records| {
            records
                .iter()
                .any(|r| matches!(&r.body, Body::Checked { command: c, .. } if c == command))
        })
    };
    fold.checks.get(key).is_some_and(|records| {
        records.iter().any(|r| match &r.body {
            Body::Checked {
                command,
                basis: Some(basis),
                ..
            } => ran(&basis.head, command) && ran(&basis.base, command),
            Body::Checked { basis: None, .. } => true,
            _ => false,
        })
    })
}

/// Whether a commit whose `Clm-By-Hand` trailers are `by_hand` is exempt: one names an
/// accepted decision and the first parent isn't signed, since in a signed repository anyone
/// who can push could name one and skip the signature audit.
fn by_hand_allowed(root: &Path, sha: &str, parents: &str, by_hand: &str) -> Result<bool, Error> {
    if let Some(first) = parents.split_whitespace().next()
        && crate::signed(root, first)?
    {
        return Ok(false);
    }
    Ok(by_hand
        .split('\x1e')
        .any(|name| accepted_decision(root, sha, name.trim())))
}

/// Whether `<sha>:docs/decisions/<name>.md` exists with `status: accepted` in its
/// frontmatter. A name that isn't a decision's file name counts for nothing.
fn accepted_decision(root: &Path, sha: &str, name: &str) -> bool {
    let named = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !named {
        return false;
    }
    let blob = format!("{sha}:docs/decisions/{name}.md");
    let Ok(text) = clm_git::git(root, &["cat-file", "blob", &blob]) else {
        return false;
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return false;
    }
    // Only a frontmatter that closes counts.
    let mut accepted = false;
    for line in lines {
        if line.trim_end() == "---" {
            return accepted;
        }
        if let Some(status) = line.strip_prefix("status:") {
            accepted |= status.trim().trim_matches(['"', '\'']) == "accepted";
        }
    }
    false
}

/// A land's kind and whether it needed review, ranked `code` = `spec` < `spec review` <
/// `mixed` (`verify-recomputes-review`): a code land and a spec land that needs no review both
/// land on checks alone, and the gate blocks a mixed diff outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rated {
    Code,
    /// Recorded as a land of a branch the gate read as `light` (`landed-record-fields`): only
    /// whether it needed review is recomputed. `review` is what the record says, which `clm
    /// land` writes false.
    Light {
        review: bool,
    },
    Spec {
        review: bool,
    },
    Mixed,
}

impl Rated {
    fn rank(self) -> u8 {
        match self {
            Rated::Code | Rated::Light { review: false } | Rated::Spec { review: false } => 0,
            Rated::Light { review: true } | Rated::Spec { review: true } => 1,
            Rated::Mixed => 2,
        }
    }

    fn needs_review(self) -> bool {
        matches!(
            self,
            Rated::Spec { review: true } | Rated::Light { review: true }
        )
    }

    /// As a `landed` record states it; `None` for any other record. `review` on a code land,
    /// which `clm land` never writes, still counts, so a record claiming review is audited
    /// for its approval whatever its kind.
    fn recorded(record: &Record) -> Option<Self> {
        match &record.body {
            Body::Landed {
                light: true,
                review,
                ..
            } => Some(Rated::Light { review: *review }),
            Body::Landed {
                kind: LandKind::Code,
                review: false,
                ..
            } => Some(Rated::Code),
            Body::Landed { review, .. } => Some(Rated::Spec { review: *review }),
            _ => None,
        }
    }

    /// As the gate gives it.
    fn gate(outcome: &clm_gate::Outcome) -> Self {
        match outcome.kind {
            clm_gate::Kind::Code => Rated::Code,
            clm_gate::Kind::Mixed => Rated::Mixed,
            // Every other kind rates as a spec branch, so a kind the gate gains later
            // compiles here.
            _ => Rated::Spec {
                review: outcome.review,
            },
        }
    }
}

/// As `clm land` prints a kind: `code`, `spec`, `spec review`, `mixed`, or `light` for a
/// land recorded light.
impl std::fmt::Display for Rated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Rated::Code => "code",
            Rated::Light { .. } => "light",
            Rated::Mixed => "mixed",
            Rated::Spec { review: false } => "spec",
            Rated::Spec { review: true } => "spec review",
        })
    }
}

/// A land of `sha` whose first parent is signed (`signed-ledger-records`): no `landed`
/// record naming it signed under `clm-land` is `ledger/unsigned-land`. The highest-ranked
/// signed record (the first in the fold's order among equals) is the recorded one; with
/// `review`, the rules in force when it landed, the gate's kind and review under them ranking
/// above it is `ledger/understated-land`. A land recorded or recomputed as needing review
/// whose diff, recomputed from the parents, no counted approval names is
/// `ledger/unapproved-land`. A land whose first parent isn't signed is judged as
/// before.
fn audit_signed(
    root: &Path,
    fold: &Fold,
    sha: &str,
    parents: &str,
    lands: &[Record],
    review: Option<u32>,
) -> Result<Vec<Finding>, Error> {
    let mut parents = parents.split_whitespace();
    let (Some(first), second) = (parents.next(), parents.next()) else {
        return Ok(Vec::new());
    };
    if !crate::signed(root, first)? {
        return Ok(Vec::new());
    }
    let finding = |rule, message| Finding {
        rule,
        commit: sha.to_string(),
        message,
    };
    let mut recorded: Option<(Rated, &Record)> = None;
    for land in lands {
        let Some(rated) = Rated::recorded(land) else {
            continue;
        };
        if recorded.is_none_or(|(best, _)| rated.rank() > best.rank())
            && signed_by(root, first, fold, land, "clm-land")?
        {
            recorded = Some((rated, land));
        }
    }
    let Some((rated, record)) = recorded else {
        return Ok(vec![finding(
            "ledger/unsigned-land",
            format!("landed {sha}: no `landed` record naming it is signed by its writer"),
        )]);
    };
    let mut findings = Vec::new();
    let mut needs_review = rated.needs_review();
    if let Some(rules) = review {
        // As `clm land` rated it: the merge commit against the `main` it merged into, under
        // the rules in force. A land the gate can't rate, a version above its `RULES`
        // included, is named and judged by its record, and the walk goes on
        // (`verify-unrated-land`).
        match clm_gate::evaluate_by(root, first, sha, false, rules) {
            // A light land is judged only by whether it needed review, as the gate's rules
            // version decides it, whatever kind the gate gives: the gate's `review` for a spec
            // branch, and whether its spec part would need review for any other.
            Ok(outcome) if matches!(rated, Rated::Light { .. }) => {
                let review = match outcome.kind {
                    clm_gate::Kind::Spec => outcome.review,
                    _ => outcome.spec_part_review,
                };
                if review && !rated.needs_review() {
                    findings.push(finding(
                        "ledger/understated-land",
                        format!("landed {sha}: recorded light, the gate gives spec review"),
                    ));
                    needs_review = true;
                }
            }
            Ok(outcome) => {
                let gate = Rated::gate(&outcome);
                if gate.rank() > rated.rank() {
                    findings.push(finding(
                        "ledger/understated-land",
                        format!("landed {sha}: recorded {rated}, the gate gives {gate}"),
                    ));
                }
                needs_review |= gate.needs_review();
            }
            Err(why) => findings.push(finding(
                "ledger/unrated-land",
                format!("landed {sha}: the gate can't rate it: {why}"),
            )),
        }
    }
    if needs_review {
        let Body::Landed { fingerprint, .. } = &record.body else {
            unreachable!("a rated record is a land");
        };
        let lander = crate::person(&record.writer);
        findings.extend(audit_approval(
            root,
            fold,
            sha,
            first,
            second,
            lander,
            fingerprint,
        )?);
    }
    Ok(findings)
}

/// A land of `sha` that needed review: `ledger/unapproved-land` unless an approval of the diff its parents
/// give, signed by its writer and, under `approval = "other"`, by someone other than the
/// lander and the land's authors, names the fingerprint its record names.
fn audit_approval(
    root: &Path,
    fold: &Fold,
    sha: &str,
    first: &str,
    second: Option<&str>,
    lander: &str,
    recorded: &str,
) -> Result<Option<Finding>, Error> {
    let finding = |rule, message| {
        Some(Finding {
            rule,
            commit: sha.to_string(),
            message,
        })
    };
    let Some(second) = second else {
        return Ok(finding(
            "ledger/unapproved-land",
            format!("landed {sha}: it needed review, but isn't a merge, so no diff was approved"),
        ));
    };
    // Unrelated histories have no merge base: nothing was diffed against `main`.
    let base = match clm_git::git(root, &["merge-base", first, second]) {
        Ok(base) => base,
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => {
            return Ok(finding(
                "ledger/unapproved-land",
                format!("landed {sha}: its parents share no history, so no diff was approved"),
            ));
        }
        Err(e) => return Err(e.into()),
    };
    let fingerprint = clm_gate::fingerprint_of(&listing(root, &base)?, &listing(root, second)?);
    if fingerprint != *recorded {
        return Ok(finding(
            "ledger/unapproved-land",
            format!(
                "landed {sha}: its `landed` record names fingerprint {recorded}, but its \
                 parents give {fingerprint}, so any approval was for another diff"
            ),
        ));
    }
    // Under `approval = "other"`, neither the lander nor an author of the land's commits.
    let mut excluded = BTreeSet::new();
    if approval_other(root, first)? {
        excluded.insert(lander.to_string());
        let authors = clm_git::git(
            root,
            &["log", "--format=%an", &format!("{first}..{second}")],
        )?;
        excluded.extend(authors.lines().map(crate::slug));
    }
    for approval in fold.approvals.get(&fingerprint).into_iter().flatten() {
        if !excluded.contains(crate::person(&approval.writer))
            && signed_by(root, first, fold, approval, "clm-approve")?
        {
            return Ok(None);
        }
    }
    Ok(finding(
        "ledger/unapproved-land",
        format!(
            "landed {sha}: it needed review, but no `approved` record for its fingerprint \
             {fingerprint} is signed by its writer{}",
            if excluded.is_empty() {
                String::new()
            } else {
                " and by someone other than the lander and the authors".into()
            }
        ),
    ))
}

/// A commit's first parent, or `None` for a root commit.
pub(crate) fn first_parent(root: &Path, sha: &str) -> Result<Option<String>, Error> {
    let line = clm_git::git(root, &["rev-list", "--parents", "-n", "1", sha])?;
    Ok(line.split_whitespace().nth(1).map(str::to_string))
}

/// Whether the record's line, as read, is signed under `namespace` by its writer.
fn signed_by(
    root: &Path,
    rev: &str,
    fold: &Fold,
    record: &Record,
    namespace: &str,
) -> Result<bool, Error> {
    match fold.lines.get(&record.id) {
        Some(line) => crate::signed_by(root, rev, line, namespace),
        None => Ok(false),
    }
}

fn listing(root: &Path, rev: &str) -> Result<Vec<clm_gate::Entry>, Error> {
    let args = ["ls-tree", "-r", "-z", rev];
    let out = clm_git::run(root, "git", &args, clm_git::Options::default())?;
    Ok(clm_gate::parse_ls_tree(&out))
}

/// Whether `rev`'s `clm.toml` sets `approval = "other"`; a missing file or key is `any`.
fn approval_other(root: &Path, rev: &str) -> Result<bool, Error> {
    let Some(text) = crate::sign::file_at(root, rev, "clm.toml")? else {
        return Ok(false);
    };
    let table: toml::Table = String::from_utf8_lossy(&text)
        .parse()
        .map_err(|e| Error::Settings(format!("{rev}:clm.toml: {e}")))?;
    Ok(table.get("approval").and_then(toml::Value::as_str) == Some("other"))
}

/// Whether the repository has a ledger branch, local or on `origin`.
pub(crate) fn has_ledger(root: &Path) -> Result<bool, Error> {
    Ok(!read::list(root, "refs/heads/clm/ledger/")?.is_empty()
        || !read::list(root, &read::remote_ledgers())?.is_empty())
}
