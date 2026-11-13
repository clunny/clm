//! A fixture repository with an `origin`, and a fake `clm` for the loop's subprocesses.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use clm_agent_harness::AgentHarness;
use clm_loop::{Kind, Options, Run, RunState};

pub struct Fixture {
    pub tmp: tempfile::TempDir,
    pub work: PathBuf,
    pub fake: PathBuf,
}

const FAKE: &str = r#"#!/bin/sh
F='@FAKE@'
echo "$*" >> "$F/calls"
case "$1" in
next) cat "$F/next" 2>/dev/null; exit 0 ;;
triage) cat "$F/triage" 2>/dev/null; exit 0 ;;
view) if [ -f "$F/flight.json" ]; then cat "$F/flight.json"; else echo '{"nodes":[]}'; fi ;;
wrap-up) pwd > "$F/wrap-up-cwd"; cat "$F/wrap-up" 2>/dev/null; echo "wrap-up ok"; exit 0 ;;
run)
  if [ "$2" = "--finish" ]; then cat "$F/finish-$3" 2>/dev/null; exit 0; fi
  if [ -f "$F/run-exit-$2" ]; then exit "$(cat "$F/run-exit-$2")"; fi
  if [ -f "$F/lock" ]; then
    if flock -n "$(cat "$F/lock")" true; then echo free > "$F/lock-result"; else echo held > "$F/lock-result"; fi
  fi
  echo "launcher for $2" ;;
draft)
  if [ "$2" = "--finish" ]; then cat "$F/finish-$3" 2>/dev/null; exit 0; fi
  if [ "$2" = "--resume" ]; then echo "resume launcher for $3"; echo "draft $3" >&2; exit 0; fi
  n=$(cat "$F/draft-n" 2>/dev/null || echo 0); n=$((n+1)); echo $n > "$F/draft-n"
  echo "draft launcher for $*"; echo "draft d$n" >&2 ;;
esac
"#;

impl Fixture {
    pub fn new() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let origin = base.join("origin.git");
        let work = base.join("work");
        let fake = base.join("fake");
        fs::create_dir_all(&fake).unwrap();
        clm_git::git(&base, &["init", "-q", "--bare", "-b", "main", "origin.git"]).unwrap();
        clm_git::git(&base, &["clone", "-q", origin.to_str().unwrap(), "work"]).unwrap();
        let git = |args: &[&str]| clm_git::git(&work, args).unwrap();
        git(&["config", "user.email", "t@example.com"]);
        git(&["config", "user.name", "t"]);
        git(&["switch", "-q", "-c", "main"]);
        fs::write(work.join("README"), "x\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "init"]);
        git(&["push", "-q", "origin", "main"]);
        let script = fake.join("clm");
        fs::write(&script, FAKE.replace("@FAKE@", fake.to_str().unwrap())).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        Fixture { tmp, work, fake }
    }

    pub fn options(&self) -> Options {
        Options {
            clm: self.fake.join("clm"),
            root: self.work.clone(),
            cwd: self.work.clone(),
            max_builds: 3,
            agent_harness: AgentHarness::ClaudeCode,
        }
    }

    /// Sets what the fake `clm` reads: `next`, `triage`, `flight.json`, `wrap-up`,
    /// `finish-<name>` or `run-exit-<component>`.
    pub fn set(&self, name: &str, text: &str) {
        fs::write(self.fake.join(name), text).unwrap();
    }

    /// The commands the fake `clm` ran, one line each.
    pub fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake.join("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// How many times the fake ran exactly `args`.
    pub fn ran(&self, args: &str) -> usize {
        self.calls().iter().filter(|c| *c == args).count()
    }

    /// `<git common dir>/clm`.
    pub fn loop_dir(&self) -> PathBuf {
        self.work.join(".git/clm")
    }

    /// Adds a run to the loop file, with the lease renewed `renewed_minutes_ago`.
    pub fn add_run(&self, run: Run) {
        clm_loop::update(&self.work, |f| {
            f.runs.push(run);
            Ok(())
        })
        .unwrap();
    }

    pub fn set_lease(&self, renewed_minutes_ago: u64) {
        clm_loop::update(&self.work, |f| {
            f.lease = Some(clm_git::now() - renewed_minutes_ago * 60);
            Ok(())
        })
        .unwrap();
    }

    pub fn file(&self) -> clm_loop::LoopFile {
        clm_loop::read(&self.work).unwrap()
    }
}

pub fn run(name: &str, kind: Kind, model: &str, state: RunState, task: Option<&str>) -> Run {
    Run {
        name: name.to_string(),
        kind: Some(kind),
        model: model.to_string(),
        state,
        task_id: task.map(str::to_string),
        started: clm_git::now(),
        round: 1,
        stops: BTreeMap::new(),
        orphan: false,
        items: Vec::new(),
        prompt: None,
        command: None,
        landed: false,
    }
}

/// The first line of `lines` that starts with `prefix`.
pub fn line<'a>(lines: &'a [String], prefix: &str) -> &'a str {
    lines
        .iter()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no `{prefix}` line in {lines:?}"))
}
