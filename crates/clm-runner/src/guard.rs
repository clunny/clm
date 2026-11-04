//! The runner's refusal to start sessions that wouldn't load the guard
//! (`runner-needs-guard`): `claude plugin list --json`, run in the clone's main worktree, must
//! show the `clm` plugin enabled there.

use std::path::Path;
use std::process::{Command, Stdio};

/// Ok when the `clm` plugin is enabled in the main worktree of the clone at `root`; otherwise
/// the message the runner prints before it exits 1.
pub(crate) fn check(claude: &Path, root: &Path) -> Result<(), String> {
    let listed = clm_git::worktrees(root).map_err(|e| format!("git failed: {e}"))?;
    let main = &listed.first().ok_or("git failed: no worktree listed")?.path;
    let out = Command::new(claude)
        .args(["plugin", "list", "--json"])
        .current_dir(main)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{} failed: {e}", claude.display()))?;
    if !out.status.success() {
        return Err(format!(
            "`claude plugin list` failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let enabled = clm_enabled(&out.stdout, main)
        .map_err(|e| format!("`claude plugin list --json` printed no plugin list: {e}"))?;
    if enabled {
        Ok(())
    } else {
        Err(format!(
            "the `clm` plugin isn't enabled in {}, so its sessions would run without the guard; \
             install and enable it there",
            main.display()
        ))
    }
}

/// Whether `json`, the output of `claude plugin list --json`, holds a plugin whose `id`'s name
/// part, before `@`, is `clm`, enabled for `main`: `enabled`, and `projectEnabled` when its
/// `projectPath` is `main`.
fn clm_enabled(json: &[u8], main: &Path) -> Result<bool, serde_json::Error> {
    let plugins: Vec<serde_json::Value> = serde_json::from_slice(json)?;
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let main = canonical(main);
    let flag = |p: &serde_json::Value, key: &str| p[key].as_bool() == Some(true);
    Ok(plugins.iter().any(|p| {
        let named = p["id"]
            .as_str()
            .is_some_and(|id| id.split('@').next() == Some("clm"));
        let here = p["projectPath"]
            .as_str()
            .is_some_and(|path| canonical(Path::new(path)) == main);
        named && flag(p, "enabled") && (!here || flag(p, "projectEnabled"))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(json: &str) -> bool {
        clm_enabled(json.as_bytes(), Path::new("/repo")).unwrap()
    }

    #[test]
    fn name_part_before_at() {
        // req:orchestrator.runner/runner-needs-guard@df4cfbe1
        assert!(enabled(r#"[{"id":"clm@clm","enabled":true}]"#));
        assert!(enabled(r#"[{"id":"clm@elsewhere","enabled":true}]"#));
        assert!(!enabled(r#"[{"id":"clm-panel@clm","enabled":true}]"#));
        assert!(!enabled(r#"[{"id":"other@clm","enabled":true}]"#));
        assert!(!enabled("[]"));
    }

    #[test]
    fn enabled_and_project_enabled_here() {
        // req:orchestrator.runner/runner-needs-guard@df4cfbe1
        assert!(!enabled(r#"[{"id":"clm@clm","enabled":false}]"#));
        assert!(!enabled(r#"[{"id":"clm@clm"}]"#));
        assert!(!enabled(
            r#"[{"id":"clm@clm","enabled":true,"projectPath":"/repo","projectEnabled":false}]"#
        ));
        assert!(enabled(
            r#"[{"id":"clm@clm","enabled":true,"projectPath":"/repo","projectEnabled":true}]"#
        ));
        assert!(enabled(
            r#"[{"id":"clm@clm","enabled":true,"projectPath":"/other","projectEnabled":false}]"#
        ));
        assert!(enabled(
            r#"[{"id":"clm@clm","enabled":false},{"id":"clm@clm","enabled":true}]"#
        ));
    }

    #[test]
    fn not_a_list() {
        // req:orchestrator.runner/runner-needs-guard@df4cfbe1
        assert!(clm_enabled(b"not json", Path::new("/repo")).is_err());
    }
}
