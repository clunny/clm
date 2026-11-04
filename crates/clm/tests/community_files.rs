//! The community files in `.github/` that tell an outsider how to take part
//! (ci/security-policy, ci/code-of-conduct, ci/issue-forms, ci/pull-request-template).

use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

fn github(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github")
        .join(name)
}

fn read(name: &str) -> String {
    fs::read_to_string(github(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn assert_holds(file: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(text.contains(needle), "{file} doesn't hold {needle}");
    }
}

#[test]
fn security_policy_says_where_to_report() {
    // req:ci/security-policy@5c07eba8
    assert_holds(
        "SECURITY.md",
        &read("SECURITY.md"),
        &[
            "https://github.com/clunny/clm/security/advisories/new",
            "public issue",
            "latest release",
            "`clm` binary",
            "plugin",
            "web app",
        ],
    );
}

#[test]
fn code_of_conduct_is_the_covenant_with_our_contact() {
    // req:ci/code-of-conduct@2d5843d4
    let text = read("CODE_OF_CONDUCT.md");
    assert_holds(
        "CODE_OF_CONDUCT.md",
        &text,
        &["Contributor Covenant", "version 2.1", "conduct@clm-app.dev"],
    );
    assert!(!text.contains("[INSERT"), "the contact was left unfilled");
}

#[test]
fn issue_forms_parse_and_say_what_is_asked() {
    // req:ci/issue-forms@1c64b676
    let parse = |name: &str| -> Value {
        serde_yaml::from_str(&read(&format!("ISSUE_TEMPLATE/{name}")))
            .unwrap_or_else(|e| panic!("{name} doesn't parse: {e}"))
    };
    for name in ["bug.yml", "idea.yml"] {
        let form = parse(name);
        for key in ["name", "description", "body"] {
            assert!(form.get(key).is_some(), "{name} lacks {key}");
        }
    }
    assert_holds(
        "idea.yml",
        &read("ISSUE_TEMPLATE/idea.yml"),
        &["one line", "accepted"],
    );
    let config = parse("config.yml");
    assert_eq!(config["blank_issues_enabled"], Value::Bool(false));
    assert_holds(
        "config.yml",
        &read("ISSUE_TEMPLATE/config.yml"),
        &["security/advisories/new", "CONTRIBUTING.md"],
    );
}

#[test]
fn pull_request_template_lists_the_spec_paths() {
    // req:ci/pull-request-template@4dbdf428
    assert_holds(
        "PULL_REQUEST_TEMPLATE.md",
        &read("PULL_REQUEST_TEMPLATE.md"),
        &[
            "specs/",
            "changes/",
            "backlog/",
            "docs/",
            "ARCHITECTURE.md",
            "CAPABILITIES.md",
            "CLAUDE.md",
            "CONTRIBUTING.md",
            "closed without review",
            "Which issue",
        ],
    );
}
