//! The human's queue: what waits on the person, oldest first (`humans-queue`). It never calls
//! the host, so intake isn't in it.

use crate::plan::Snapshot;
use crate::state::StoredHuman;
use crate::summary::UNSAFE_WORK;

fn entry(name: &str, kind: &str, what: String, since: u64, action: String) -> StoredHuman {
    StoredHuman {
        name: name.to_string(),
        kind: kind.to_string(),
        what,
        since,
        action,
    }
}

/// What the person does about a line of `clm wrap-up`.
fn unsafe_action(line: &str) -> &'static str {
    if line.starts_with("unpushed ") {
        "push it"
    } else if line.starts_with("dirty ") {
        "commit it or set it aside"
    } else {
        "audit it with clm ledger verify"
    }
}

/// The queue for a snapshot, oldest first.
pub(crate) fn queue(s: &Snapshot) -> Vec<StoredHuman> {
    let file = &s.file;
    let mut out = Vec::new();
    for batch in file.batches.iter().filter(|b| !b.approved) {
        out.push(entry(
            &batch.id,
            "fold",
            "fold batch waiting for approval".to_string(),
            batch.since,
            format!("say \"approve {}\"", batch.id),
        ));
    }
    for item in s
        .backlog
        .iter()
        .filter(|i| matches!(i.kind.as_str(), "question" | "human-task") && !i.blocks.is_empty())
    {
        out.push(entry(
            &item.slug,
            &item.kind,
            format!("blocks {}", item.blocks.join(", ")),
            item.since,
            format!("answer backlog/{}", item.slug),
        ));
    }
    for esc in &file.escalations {
        out.push(entry(
            &esc.slug,
            "escalation",
            "the loop couldn't land its question item".to_string(),
            esc.since,
            format!("fix the land of loop/{}", esc.slug),
        ));
    }
    for red in &file.red_verifies {
        let commit = red.split_whitespace().nth(2).unwrap_or_default();
        let taken = s.backlog.iter().any(|i| {
            i.verify_failures
                .iter()
                .any(|f| !commit.is_empty() && f.starts_with(commit))
        });
        if !taken {
            out.push(entry(
                "verify",
                "verify",
                red.clone(),
                s.now,
                "take in with /clm:triage".to_string(),
            ));
        }
    }
    for line in &s.unsafe_work {
        out.push(entry(
            line,
            UNSAFE_WORK,
            line.clone(),
            s.now,
            unsafe_action(line).to_string(),
        ));
    }
    for id in &s.upgrades {
        out.push(entry(
            id,
            "upgrade",
            "a migration is pending".to_string(),
            s.now,
            "run /clm:upgrade".to_string(),
        ));
    }
    if file.conditions.disk {
        out.push(entry(
            "disk",
            "condition",
            "the disk is full".to_string(),
            s.now,
            "free disk, then say so".to_string(),
        ));
    }
    out.sort_by_key(|e| e.since);
    out
}
