//! `Journal`: what a run changed, and putting it back (`journal`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use clm_apply::{FileOp, Journal};
use tempfile::TempDir;

fn write(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn put(rel: &str, bytes: &[u8]) -> FileOp {
    FileOp::Write(rel.into(), bytes.to_vec())
}

#[test]
fn undo_after_writes_and_a_removal() {
    // req:apply/journal@e3e1076a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "x/f", b"1");
    write(d, "y/g", b"g bytes");
    let mut journal = Journal::new(d);
    journal.perform(&put("x/f", b"2")).unwrap();
    journal.perform(&put("z/new/h", b"h")).unwrap();
    journal.perform(&FileOp::Remove("y/g".into())).unwrap();
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"2");
    assert!(d.join("z/new/h").is_file());
    assert!(!d.join("y/g").exists());

    let not_back = journal.undo();
    assert!(not_back.is_empty(), "{not_back:?}");
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"1");
    assert_eq!(fs::read(d.join("y/g")).unwrap(), b"g bytes");
    assert!(!d.join("z").exists());
}

#[test]
fn one_remembered_state_per_path() {
    // req:apply/journal@e3e1076a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "x/f", b"1");
    let mut journal = Journal::new(d);
    journal.perform(&put("x/f", b"2")).unwrap();
    journal.perform(&put("x/f", b"3")).unwrap();
    assert!(journal.undo().is_empty());
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"1");
}

#[test]
fn a_directory_that_was_there_stays() {
    // req:apply/journal@e3e1076a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    fs::create_dir_all(d.join("x/empty")).unwrap();
    let mut journal = Journal::new(d);
    journal.perform(&put("x/empty/new", b"n")).unwrap();
    assert!(journal.undo().is_empty());
    assert!(!d.join("x/empty/new").exists());
    // Only the directories the journal's writes created go.
    assert!(d.join("x/empty").is_dir());
}

#[test]
fn no_op_on_a_path_it_cant_read() {
    // req:apply/journal@e3e1076a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "x/f", b"1");
    mode(&d.join("x/f"), 0o200);
    let mut journal = Journal::new(d);
    let refused = journal.perform(&put("x/f", b"2"));
    mode(&d.join("x/f"), 0o644);
    assert!(refused.is_err());
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"1");
    assert!(journal.undo().is_empty());
}

#[test]
fn a_path_already_as_it_was_is_left_alone() {
    // req:apply/journal@e3e1076a
    // The write is refused before it opens the file, which still holds what it held, so the
    // undo leaves it and names nothing, though it could not write it either.
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "x/f", b"1");
    mode(&d.join("x/f"), 0o444);
    let mut journal = Journal::new(d);
    assert!(journal.perform(&put("x/f", b"2")).is_err());
    let not_back = journal.undo();
    mode(&d.join("x/f"), 0o644);
    assert!(not_back.is_empty(), "{not_back:?}");
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"1");
}

#[test]
fn a_path_it_cant_put_back_is_returned() {
    // req:apply/journal@e3e1076a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    fs::create_dir_all(d.join("locked")).unwrap();
    write(d, "x/f", b"1");
    let mut journal = Journal::new(d);
    journal.perform(&put("locked/new", b"n")).unwrap();
    journal.perform(&put("x/f", b"2")).unwrap();
    // The new file can't be removed from a directory that no longer takes changes.
    mode(&d.join("locked"), 0o555);
    let not_back = journal.undo();
    mode(&d.join("locked"), 0o755);
    let paths: Vec<_> = not_back.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(paths, [Path::new("locked/new")]);
    // The others are still put back.
    assert_eq!(fs::read(d.join("x/f")).unwrap(), b"1");
}
