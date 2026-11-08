//! The line `session-start` prints when the project has migrations pending
//! (`session-upgrade-reminder`).

use clm_upgrade::{Level, Migration, TooNew};

/// The one line telling a session what `upgrade` says is pending, or none when nothing is.
/// A `required` migration in `skipped` is counted as skipped, not as blocking.
pub(crate) fn upgrade_line(
    pending: Result<Vec<&Migration>, TooNew>,
    skipped: &[String],
) -> Option<String> {
    let pending = match pending {
        Ok(pending) => pending,
        Err(TooNew(id)) => {
            return Some(format!(
                "CLM upgrade: this project was upgraded by a newer `clm` (migration `{id}`) and \
needs a newer `clm`; commands are blocked until one is installed."
            ));
        }
    };
    if pending.is_empty() {
        return None;
    }
    let advised = pending.iter().filter(|m| m.level == Level::Advised).count();
    let required: Vec<&&Migration> = pending
        .iter()
        .filter(|m| m.level == Level::Required)
        .collect();
    let blocking = required
        .iter()
        .filter(|m| !skipped.iter().any(|s| s == m.id))
        .count();
    let mut counts = Vec::new();
    if !required.is_empty() {
        counts.push(format!("{} required", required.len()));
    }
    if advised > 0 {
        counts.push(format!("{advised} advised"));
    }
    let mut line = format!("CLM upgrade: {} migration(s) pending", counts.join(", "));
    if blocking > 0 {
        line.push_str("; commands are blocked");
    } else if !required.is_empty() {
        line.push_str("; the required migration(s) are skipped on this clone");
    }
    line.push_str("; run `/clm:upgrade`.");
    Some(line)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clm_upgrade::Kind;

    use super::*;

    fn migration(id: &'static str, level: Level) -> Migration {
        Migration {
            id,
            level,
            done: |_| false,
            kind: Kind::Apply(|_: &Path| Ok(()), ""),
        }
    }

    fn line(list: &[Migration], skipped: &[&str]) -> Option<String> {
        let skipped: Vec<String> = skipped.iter().map(|s| s.to_string()).collect();
        upgrade_line(Ok(list.iter().collect()), &skipped)
    }

    #[test]
    fn advised_drift_is_not_blocking() {
        // req:plugin.hooks/session-upgrade-reminder@5a5a1830
        let text = line(&[migration("a", Level::Advised)], &[]).unwrap();
        assert!(
            text.contains("1 advised") && text.contains("/clm:upgrade"),
            "{text}"
        );
        assert!(
            !text.contains("blocked") && !text.contains("required"),
            "{text}"
        );
    }

    #[test]
    fn required_pending_blocks() {
        // req:plugin.hooks/session-upgrade-reminder@5a5a1830
        let text = line(&[migration("a", Level::Required)], &[]).unwrap();
        for needle in ["1 required", "blocked", "/clm:upgrade"] {
            assert!(text.contains(needle), "{needle}: {text}");
        }
        assert!(!text.contains("skipped"), "{text}");
    }

    #[test]
    fn required_but_skipped_does_not_block() {
        // req:plugin.hooks/session-upgrade-reminder@5a5a1830
        let list = [
            migration("a", Level::Required),
            migration("b", Level::Advised),
        ];
        let text = line(&list, &["a"]).unwrap();
        assert!(
            text.contains("skipped") && !text.contains("blocked"),
            "{text}"
        );
        assert!(text.contains("1 required, 1 advised"), "{text}");
        // One skipped, one not: still blocked.
        let list = [
            migration("a", Level::Required),
            migration("c", Level::Required),
        ];
        assert!(line(&list, &["a"]).unwrap().contains("blocked"));
    }

    #[test]
    fn nothing_pending_is_no_line() {
        // req:plugin.hooks/session-upgrade-reminder@5a5a1830
        assert_eq!(line(&[], &[]), None);
    }

    #[test]
    fn too_new_says_a_newer_clm() {
        // req:plugin.hooks/session-upgrade-reminder@5a5a1830
        let text = upgrade_line(Err(TooNew("x".into())), &[]).unwrap();
        assert!(text.contains("needs a newer `clm`"), "{text}");
    }
}
