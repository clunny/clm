//! The defect paragraph `clm hook session-start` adds after the claims (`defect-rule`).

use std::fs;
use std::path::Path;

use clm_view_repo::Budgets;

/// What the call of `clm hook session-start` prints for an event whose `cwd` is `root`, with
/// no user settings; it must exit 0.
fn start(root: &Path) -> String {
    let config = tempfile::tempdir().unwrap();
    let home = config.path().display().to_string();
    let lookup = |key: &str| (key == "XDG_CONFIG_HOME").then(|| home.clone());
    let budgets = |_: &Path| {
        Ok(Budgets {
            spec_tokens: 1_000_000,
            code_tokens: 1_000_000,
        })
    };
    let event = serde_json::json!({ "cwd": root }).to_string();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_hooks::session_start_command(
        std::iter::empty::<&str>(),
        None,
        root,
        &event,
        &lookup,
        &mut out,
        &mut err,
        &budgets,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    String::from_utf8(out).unwrap()
}

#[test]
fn the_paragraph_says_component() {
    // req:plugin.hooks/defect-rule@c1b0da79
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
    // `backlog`, then no settings (`ask`).
    fs::write(dir.path().join("clm.toml"), "defects = \"backlog\"\n").unwrap();
    let backlog = start(dir.path());
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    let ask = start(dir.path());
    for (out, names, not) in [
        (&backlog, "backlog/", "<<'EOF'"),
        (&ask, "<<'EOF'", "backlog/"),
    ] {
        let claims = out.find("\nclaims\n").expect("the claims section");
        let after = &out[claims..];
        // One paragraph names `clm defect`, after the claims: the text's last.
        let paragraphs: Vec<&str> = out
            .split("\n\n")
            .filter(|p| p.contains("clm defect"))
            .collect();
        assert_eq!(paragraphs.len(), 1, "{out}");
        let paragraph = paragraphs[0];
        assert!(after.contains(paragraph), "{out}");
        assert!(!paragraph.trim_end().contains('\n'), "one paragraph: {out}");
        assert!(paragraph.contains(names), "{names}: {paragraph}");
        assert!(!out.contains(not), "{not}: {out}");
    }
}
