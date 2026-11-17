//! Fixtures shared by the tests: spec texts, a chart with its colour removed, and the
//! commands called as `crates/clm` calls them. Each test file uses a part of them.
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use clm_check::{Budgets, Violation};

/// A `specd` node with no owners, depending on `deps` (comma-separated), with requirements
/// `reqs`, each `It SHALL.`.
pub fn specd(deps: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// `s` with its ANSI escapes removed.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The exit status a call returned, read as a process's.
#[derive(Debug)]
pub struct Status(u8);

impl Status {
    pub fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

/// What a call wrote to its two streams, and what it returned.
#[derive(Debug)]
pub struct Output {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// `[component_size]` of a `clm.toml` holding `text`, as `config` reads it: 12000 spec and
/// 40000 code tokens where it sets none.
fn budgets_in(text: Option<&str>) -> Result<Budgets, String> {
    let table: toml::Table = text
        .unwrap_or_default()
        .parse()
        .map_err(|e| format!("{e}"))?;
    let size = table.get("component_size");
    let budget = |key: &str, default: u64| match size.and_then(|s| s.get(key)) {
        None => Ok(default),
        Some(v) => v
            .as_integer()
            .map(|n| n as u64)
            .ok_or_else(|| format!("`component_size.{key}` is no number")),
    };
    Ok(Budgets {
        spec_tokens: budget("spec_tokens", 12000)?,
        code_tokens: budget("code_tokens", 40000)?,
    })
}

/// The working tree's settings for `root`, as `crates/clm` passes them: its `clm.toml`.
fn repository_budgets(root: &Path) -> Result<Budgets, Violation> {
    let text = fs::read_to_string(root.join("clm.toml")).ok();
    budgets_in(text.as_deref()).map_err(|message| Violation {
        path: root.join("clm.toml"),
        rule: "check/node-size",
        message,
    })
}

/// `clm <args> --root <root>` through `clm-view-repo`'s call of the command `args` names.
pub fn clm(root: &Path, args: &[&str]) -> Output {
    let (name, rest) = args.split_first().unwrap();
    let mut argv: Vec<std::ffi::OsString> = rest.iter().map(Into::into).collect();
    argv.push("--root".into());
    argv.push(root.into());
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let here = Path::new(".");
    let code = match *name {
        "status" => clm_view_repo::status_command(
            &argv,
            here,
            &|_| None,
            &mut stdout,
            &mut stderr,
            &repository_budgets,
        ),
        "view" => clm_view_repo::view_command(
            &argv,
            here,
            &|_| None,
            &mut stdout,
            &mut stderr,
            &repository_budgets,
            &budgets_in,
        ),
        "pending" => {
            clm_view_repo::pending_command(&argv, here, &|_| None, &mut stdout, &mut stderr)
        }
        other => panic!("no call for `clm {other}`"),
    };
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
}
