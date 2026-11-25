//! The human's notes for every launch (`notes-reach-every-launch`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use clm_git::Failure;
use clm_loop::{
    Kind, RunState, StopMode, approved, cleared, release, returned, start, start_with_notes, step,
    stop,
};
use common::{Fixture, line, run};

const SECTION: &str = "\n## The human's notes for this loop\n\n";

/// The text of the prompt file the `launch <name>` line among `actions` names.
fn prompt(actions: &[String], name: &str) -> String {
    let path = line(actions, &format!("launch {name} "))
        .rsplit(' ')
        .next()
        .unwrap()
        .to_string();
    fs::read_to_string(path).unwrap()
}

/// The run's copy of the notes, `.clm/prompts/loop/notes/<name>.md` with `:` written `-`.
fn copy_of(fx: &Fixture, name: &str) -> PathBuf {
    std::path::absolute(&fx.work)
        .unwrap()
        .join(".clm/prompts/loop/notes")
        .join(format!("{}.md", name.replace(':', "-")))
}

/// Asserts that `text`, a prompt, ends with the notes' section holding one line that names the
/// absolute path of `copy`, that `copy` holds `notes` unchanged, and that the prompt doesn't.
fn names_the_copy(text: &str, copy: &Path, notes: &str) {
    let (_, section) = text
        .rsplit_once(SECTION)
        .unwrap_or_else(|| panic!("no notes section: {text}"));
    assert_eq!(section.lines().count(), 1, "{section:?}");
    assert!(section.ends_with('\n'), "{section:?}");
    let path = copy.display().to_string();
    assert!(copy.is_absolute(), "{path}");
    assert!(section.contains(&path), "{section:?} doesn't name {path}");
    assert_eq!(fs::read_to_string(copy).unwrap(), notes);
    assert!(!text.contains(notes.trim()), "{text}");
}

fn unreachable_origin(fx: &Fixture) {
    let missing = fx.tmp.path().join("no-such-origin");
    clm_git::git(
        &fx.work,
        &["remote", "set-url", "origin", missing.to_str().unwrap()],
    )
    .unwrap();
}

fn reachable_origin(fx: &Fixture) {
    let origin = fx.tmp.path().canonicalize().unwrap().join("origin.git");
    clm_git::git(
        &fx.work,
        &["remote", "set-url", "origin", origin.to_str().unwrap()],
    )
    .unwrap();
}

#[test]
fn notes_reach_every_launch() {
    // req:orchestrator.loop/notes-reach-every-launch@74d6b3af
    let fx = Fixture::new();
    fx.set("next", "a\n");
    fs::write(fx.work.join("notes.md"), "Keep the old tests.\n").unwrap();
    let notes = Path::new("notes.md");
    let first = start_with_notes(&fx.options(), "opus", false, notes).unwrap();
    let copy = copy_of(&fx, "build:a");
    names_the_copy(&prompt(&first, "build:a"), &copy, "Keep the old tests.\n");

    // After the lease lapses, a start without notes relaunches `build:a` with them, writing the
    // copy again with the prompt, replacing what the file held.
    fx.set_lease(20);
    fs::write(&copy, "Stale.").unwrap();
    let adopted = start(&fx.options(), "opus", false).unwrap();
    names_the_copy(&prompt(&adopted, "build:a"), &copy, "Keep the old tests.\n");
    let runs_of_a = fx
        .calls()
        .iter()
        .filter(|c| c.starts_with("run a "))
        .count();
    assert_eq!(runs_of_a, 2, "the relaunch ran `clm run a` again");

    // Notes that can't be read are refused before the fetch, changing nothing.
    let loop_file = fx.loop_dir().join("loop");
    let notes_file = fx.loop_dir().join("loop.notes");
    unreachable_origin(&fx);
    fx.set_lease(20);
    let file_before = fs::read(&loop_file).unwrap();
    let notes_before = fs::read(&notes_file).unwrap();
    let Err(Failure::Refused(why)) =
        start_with_notes(&fx.options(), "opus", false, Path::new("missing.md"))
    else {
        panic!("missing notes weren't refused before the fetch");
    };
    assert!(why.contains("missing.md"), "{why}");
    assert_eq!(fs::read(&loop_file).unwrap(), file_before);
    assert_eq!(fs::read(&notes_file).unwrap(), notes_before);
    reachable_origin(&fx);

    // A drain's `end` removes them: the next start launches with no section.
    fx.change_run("build:a", |r| r.state = RunState::Returned);
    stop(&fx.options(), StopMode::Drain).unwrap();
    let last = step(&fx.options()).unwrap();
    assert_eq!(last.last().map(String::as_str), Some("end"), "{last:?}");
    assert!(!notes_file.exists());
    let fresh = start(&fx.options(), "opus", false).unwrap();
    let text = prompt(&fresh, "build:a");
    assert!(!text.contains("## The human's notes"), "{text}");
    assert_eq!(text, "loop run build:a\nlauncher for a\n");
}

#[test]
fn unreadable_notes_stop_every_launch() {
    // req:orchestrator.loop/notes-reach-every-launch@74d6b3af
    let fx = Fixture::new();
    fx.set("next", "a\n");
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set_lease(0);
    let notes_file = fx.loop_dir().join("loop.notes");
    fs::write(&notes_file, [0xff, 0xfe, b'x']).unwrap();
    let loop_file = fx.loop_dir().join("loop");
    let before = fs::read(&loop_file).unwrap();

    // A step that would launch `build:a` fails naming the file, and claims nothing.
    let Err(Failure::Git(why)) = step(&fx.options()) else {
        panic!("a step read notes that aren't UTF-8");
    };
    assert!(why.contains("loop.notes"), "{why}");
    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run a ")),
        "{:?}",
        fx.calls()
    );

    // A return is refused the same way before it records anything, so a retry counts once.
    let Err(Failure::Git(why)) = returned(
        &fx.options(),
        "build:b",
        Some("t-1"),
        "landed abc",
        None,
        None,
    ) else {
        panic!("a return read notes that aren't UTF-8");
    };
    assert!(why.contains("loop.notes"), "{why}");
    assert_eq!(fs::read(&loop_file).unwrap(), before);
    assert_eq!(fx.finishes("b"), 0);

    // Every other command that may write a prompt reads them first too.
    let options = fx.options();
    let others: [(&str, Result<Vec<String>, Failure>); 4] = [
        ("release", release(&options, "b")),
        ("approved", approved(&options, "b1")),
        ("cleared", cleared(&options, "disk")),
        ("start", start(&options, "opus", true)),
    ];
    for (command, got) in others {
        let Err(Failure::Git(why)) = got else {
            panic!("{command} read notes that aren't UTF-8");
        };
        assert!(why.contains("loop.notes"), "{command}: {why}");
        assert_eq!(fs::read(&loop_file).unwrap(), before, "{command}");
    }
    assert_eq!(fx.ran("branch delete impl/b"), 0);
    assert_eq!(fx.finishes("b"), 0);

    // `start --notes` replaces them without reading the old file.
    fs::write(fx.work.join("notes.md"), "New notes.").unwrap();
    let actions = start_with_notes(&fx.options(), "opus", true, Path::new("notes.md")).unwrap();
    names_the_copy(
        &prompt(&actions, "build:a"),
        &copy_of(&fx, "build:a"),
        "New notes.",
    );
    assert_eq!(fs::read_to_string(&notes_file).unwrap(), "New notes.");
}

#[test]
fn notes_of_white_space_add_no_section() {
    // req:orchestrator.loop/notes-reach-every-launch@74d6b3af
    let fx = Fixture::new();
    fx.set("next", "a\n");
    fs::write(fx.work.join("blank.md"), "  \n\t\n").unwrap();
    let actions = start_with_notes(&fx.options(), "opus", false, Path::new("blank.md")).unwrap();
    assert_eq!(
        prompt(&actions, "build:a"),
        "loop run build:a\nlauncher for a\n"
    );
    assert!(!copy_of(&fx, "build:a").exists());
}
