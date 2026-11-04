//! Build places (`build-places`): the files under `<git common dir>/clm/builds/` that
//! bound how many builds and drafts the clone runs at once.
//!
//! [`places`] reads them and judges each on the remote-tracking refs as they are, without
//! fetching; [`take_place`], [`rename_place`], [`free_place`] and [`free_stale_places`]
//! change them under an exclusive lock on `<git common dir>/clm/builds.lock`.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clm_git::git;

use crate::{Error, REMOTE, lapsed};
use clm_git::now;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    Build,
    Draft,
}

/// What a place file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceFile {
    pub kind: PlaceKind,
    /// When the place was taken, in Unix seconds.
    pub taken: u64,
    /// `impl/<node>` for a build; `spec/<id>` then each `propose/<slug>` for a draft.
    pub branches: Vec<String>,
}

/// A place on the clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The node ID for a build, the change ID (or a provisional name) for a draft.
    pub name: String,
    /// The file, absolute.
    pub path: PathBuf,
    /// The file's contents, or why it can't be read or parsed: such a place counts.
    pub file: Result<PlaceFile, String>,
    /// Whether it holds one of the clone's `max_builds` places.
    pub counts: bool,
}

/// What [`take_place`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placed {
    /// It took a new place.
    New,
    /// The place of that name counted, so it rewrote it.
    Reused,
    /// The clone is full: `held` places count, of `max_builds`.
    Full { held: usize, max_builds: u64 },
}

/// Every place of the clone, sorted by name, with whether it counts. Never fetches.
pub fn places(root: &Path, expiry_hours: u64) -> Result<Vec<Place>, Error> {
    read(&Dirs::of(root)?, root, expiry_hours)
}

/// Takes place `name`, of `kind` and belonging to `branches`: reuses it when a place of
/// that name counts, takes a new one when fewer than `max_builds` count, and otherwise
/// takes nothing. The places that don't count are deleted first.
pub fn take_place(
    root: &Path,
    name: &str,
    kind: PlaceKind,
    branches: &[String],
    max_builds: u64,
    expiry_hours: u64,
) -> Result<Placed, Error> {
    place_name(name)?;
    let dirs = Dirs::of(root)?;
    let _lock = dirs.lock()?;
    let mut held = 0;
    let mut own = false;
    for place in read(&dirs, root, expiry_hours)? {
        if !place.counts {
            remove(&place.path)?;
        } else if place.name == name {
            own = true;
        } else {
            held += 1;
        }
    }
    let placed = if own {
        Placed::Reused
    } else if (held as u64) < max_builds {
        Placed::New
    } else {
        return Ok(Placed::Full { held, max_builds });
    };
    let file = PlaceFile {
        kind,
        taken: now(),
        branches: branches.to_vec(),
    };
    dirs.write(name, &format(&file))?;
    Ok(placed)
}

/// Renames place `from` to `to`, refusing when `to` exists or `from` doesn't.
pub fn rename_place(root: &Path, from: &str, to: &str) -> Result<(), Error> {
    place_name(from)?;
    place_name(to)?;
    let dirs = Dirs::of(root)?;
    let _lock = dirs.lock()?;
    let (from_path, to_path) = (dirs.builds.join(from), dirs.builds.join(to));
    if to_path.symlink_metadata().is_ok() {
        return Err(Error::Refused(format!("place `{to}` exists")));
    }
    if from_path.symlink_metadata().is_err() {
        return Err(Error::Refused(format!("no place `{from}`")));
    }
    fs::rename(&from_path, &to_path).map_err(|e| io(&from_path, e))
}

/// What [`free_stale_places`] did with one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweepOutcome {
    /// It counts, so it stays.
    Kept,
    /// It didn't count, and its file is deleted.
    Freed,
    /// It didn't count, but its file couldn't be deleted: why.
    Failed(String),
}

/// One place as [`free_stale_places`] judged it under the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swept {
    pub place: Place,
    pub outcome: SweepOutcome,
}

/// Under the places lock, reads the places, deletes each that doesn't count, and returns
/// every place read, sorted by name, with what became of it. A failed delete is that
/// place's outcome, and the sweep goes on; only finding the places or taking the lock
/// fails the call. Never fetches.
pub fn free_stale_places(root: &Path, expiry_hours: u64) -> Result<Vec<Swept>, Error> {
    let dirs = Dirs::of(root)?;
    let _lock = dirs.lock()?;
    Ok(read(&dirs, root, expiry_hours)?
        .into_iter()
        .map(|place| {
            let outcome = if place.counts {
                SweepOutcome::Kept
            } else {
                match delete(&place.path) {
                    Ok(()) => SweepOutcome::Freed,
                    Err(e) => SweepOutcome::Failed(format!("{}: {e}", place.path.display())),
                }
            };
            Swept { place, outcome }
        })
        .collect())
}

/// Deletes place `name`'s file; succeeds when there is none.
pub fn free_place(root: &Path, name: &str) -> Result<(), Error> {
    place_name(name)?;
    let dirs = Dirs::of(root)?;
    let _lock = dirs.lock()?;
    remove(&dirs.builds.join(name))
}

/// The clone's `clm/` directory in its git common directory, and its `builds/`.
struct Dirs {
    clm: PathBuf,
    builds: PathBuf,
}

impl Dirs {
    fn of(root: &Path) -> Result<Dirs, Error> {
        let common = git(
            root,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let common = PathBuf::from(common);
        // Canonical, so every worktree names the same files the same way.
        let common = common.canonicalize().unwrap_or(common);
        let clm = common.join("clm");
        Ok(Dirs {
            builds: clm.join("builds"),
            clm,
        })
    }

    /// The exclusive lock on `builds.lock`, held until the file is dropped. An advisory
    /// lock of the operating system, so a killed command releases it.
    fn lock(&self) -> Result<File, Error> {
        fs::create_dir_all(&self.builds).map_err(|e| io(&self.builds, e))?;
        let path = self.clm.join("builds.lock");
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| io(&path, e))?;
        file.lock().map_err(|e| io(&path, e))?;
        Ok(file)
    }

    /// Writes place `name` through a temporary file and a rename, so no reader sees half
    /// a file.
    fn write(&self, name: &str, text: &str) -> Result<(), Error> {
        let mut tmp = tempfile::NamedTempFile::new_in(&self.clm).map_err(|e| io(&self.clm, e))?;
        tmp.write_all(text.as_bytes())
            .map_err(|e| io(tmp.path(), e))?;
        let path = self.builds.join(name);
        tmp.persist(&path).map_err(|e| io(&path, e.error))?;
        Ok(())
    }
}

fn read(dirs: &Dirs, root: &Path, expiry_hours: u64) -> Result<Vec<Place>, Error> {
    let entries = match fs::read_dir(&dirs.builds) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(&dirs.builds, e)),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io(&dirs.builds, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let file = match fs::read_to_string(&path) {
            // Deleted or renamed since it was listed, by a command holding the lock.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            read => read.map_err(|e| e.to_string()).and_then(|t| parse(&t)),
        };
        files.push((name, path, file));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let refs = Refs::read(root)?;
    let now = now();
    Ok(files
        .into_iter()
        .map(|(name, path, file)| {
            let counts = match &file {
                Ok(f) => refs.counts(root, f, now, expiry_hours),
                Err(_) => true,
            };
            Place {
                name,
                path,
                file,
                counts,
            }
        })
        .collect())
}

/// The tips of the local branches and of `origin`'s tracking refs, by full ref name: each
/// its SHA and committer time.
struct Refs(BTreeMap<String, (String, u64)>);

impl Refs {
    fn read(root: &Path) -> Result<Refs, Error> {
        let out = git(
            root,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname) %(committerdate:unix)",
                "refs/heads",
                &format!("refs/remotes/{REMOTE}"),
            ],
        )?;
        Ok(Refs(
            out.lines()
                .filter_map(|l| {
                    let mut parts = l.split(' ');
                    let name = parts.next()?.to_string();
                    let sha = parts.next()?.to_string();
                    let time = parts.next()?.parse().ok()?;
                    Some((name, (sha, time)))
                })
                .collect(),
        ))
    }

    fn remote(&self, branch: &str) -> Option<&(String, u64)> {
        self.0.get(&format!("refs/remotes/{REMOTE}/{branch}"))
    }

    fn on_main(&self, root: &Path, sha: &str) -> bool {
        self.remote("main").is_some()
            && clm_git::is_ancestor(root, sha, &format!("refs/remotes/{REMOTE}/main"))
                .unwrap_or(false)
    }

    /// Whether `branch` on `origin` is a live claim: its tip committed within the expiry
    /// and, when `mergeable`, not on `origin/main`.
    fn live(&self, root: &Path, branch: &str, mergeable: bool, now: u64, expiry: u64) -> bool {
        self.remote(branch).is_some_and(|(sha, time)| {
            !lapsed(*time, now, expiry) && !(mergeable && self.on_main(root, sha))
        })
    }

    /// A place counts unless it has landed, or its claim isn't live and it was taken
    /// longer ago than the expiry.
    fn counts(&self, root: &Path, f: &PlaceFile, now: u64, expiry: u64) -> bool {
        let (landed, live) = match f.kind {
            PlaceKind::Build => (
                f.branches.iter().any(|b| self.landed_build(root, b)),
                f.branches
                    .iter()
                    .any(|b| self.live(root, b, true, now, expiry)),
            ),
            PlaceKind::Draft => {
                let id = f.branches.iter().find_map(|b| b.strip_prefix("spec/"));
                let landed = id.is_some_and(|id| {
                    ["changes", "changes/archive"].iter().any(|dir| {
                        git(
                            root,
                            &[
                                "rev-parse",
                                "--verify",
                                "-q",
                                &format!("refs/remotes/{REMOTE}/main:{dir}/{id}"),
                            ],
                        )
                        .is_ok()
                    })
                });
                let live = f.branches.iter().any(|b| match b.strip_prefix("spec/") {
                    Some(id) => [b.clone(), format!("autosync/spec/{id}")]
                        .iter()
                        .any(|r| self.live(root, r, false, now, expiry)),
                    None => self.live(root, b, false, now, expiry),
                });
                (landed, live)
            }
        };
        !landed && (live || !lapsed(f.taken, now, expiry))
    }

    /// A build's branch has landed when at least one of `origin/<branch>` and the local
    /// `<branch>` exists and each that exists is on `origin/main`.
    fn landed_build(&self, root: &Path, branch: &str) -> bool {
        let tips: Vec<&String> = [
            self.remote(branch),
            self.0.get(&format!("refs/heads/{branch}")),
        ]
        .into_iter()
        .flatten()
        .map(|(sha, _)| sha)
        .collect();
        !tips.is_empty() && tips.iter().all(|sha| self.on_main(root, sha))
    }
}

/// Refuses a name that isn't one file name.
fn place_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err(Error::Refused(format!("`{name}` isn't a place name")));
    }
    Ok(())
}

fn remove(path: &Path) -> Result<(), Error> {
    delete(path).map_err(|e| io(path, e))
}

/// Deletes a file, succeeding when there is none.
fn delete(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn io(path: &Path, e: std::io::Error) -> Error {
    Error::Git(format!("{}: {e}", path.display()))
}

fn format(f: &PlaceFile) -> String {
    let kind = match f.kind {
        PlaceKind::Build => "build",
        PlaceKind::Draft => "draft",
    };
    let mut text = format!("kind {kind}\ntaken {}\n", rfc3339(f.taken));
    for b in &f.branches {
        text.push_str(&format!("branch {b}\n"));
    }
    text
}

fn parse(text: &str) -> Result<PlaceFile, String> {
    let (mut kind, mut taken, mut branches) = (None, None, Vec::new());
    for line in text.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        let (key, value) = line
            .split_once(' ')
            .ok_or_else(|| format!("`{line}` isn't a `key value` line"))?;
        match key {
            "kind" => {
                kind = Some(match value {
                    "build" => PlaceKind::Build,
                    "draft" => PlaceKind::Draft,
                    _ => return Err(format!("unknown kind `{value}`")),
                })
            }
            "taken" => taken = Some(unix_time(value).ok_or_else(|| format!("bad time `{value}`"))?),
            "branch" => branches.push(value.to_string()),
            _ => return Err(format!("unknown key `{key}`")),
        }
    }
    Ok(PlaceFile {
        kind: kind.ok_or("no `kind` line")?,
        taken: taken.ok_or("no `taken` line")?,
        branches,
    })
}

/// `YYYY-MM-DDTHH:MM:SSZ` for Unix seconds.
fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil from days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Unix seconds for an RFC 3339 time: `YYYY-MM-DDTHH:MM:SS`, optional fractional seconds,
/// then `Z` or an offset `+HH:MM` / `-HH:MM`.
fn unix_time(t: &str) -> Option<u64> {
    if !t.is_ascii() {
        return None;
    }
    let b = t.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't') {
        return None;
    }
    let num = |r: std::ops::Range<usize>| {
        let s = t.get(r)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse::<i64>().ok())?
    };
    let mut rest = &t[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &frac[digits..];
    }
    let offset = match rest.as_bytes() {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), _, _, b':', _, _] => {
            let (oh, om) = (num(t.len() - 5..t.len() - 3)?, num(t.len() - 2..t.len())?);
            if oh > 23 || om > 59 {
                return None;
            }
            let secs = oh * 3600 + om * 60;
            if *sign == b'+' { secs } else { -secs }
        }
        _ => return None,
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // Days from civil (Howard Hinnant).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss - offset).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_round_trip() {
        for secs in [0, 951_782_400, 1_793_059_200, 1_793_145_599] {
            assert_eq!(unix_time(&rfc3339(secs)), Some(secs), "{secs}");
        }
        assert_eq!(unix_time("2026-10-27T00:00:00.123Z"), Some(1_793_059_200));
        assert_eq!(unix_time("2026-10-27t01:30:00+01:30"), Some(1_793_059_200));
        assert_eq!(
            unix_time("2026-10-26T23:00:00.5-01:00"),
            Some(1_793_059_200)
        );
        assert_eq!(unix_time("2026-10-27T00:00:00z"), Some(1_793_059_200));
        for bad in [
            "2026-10-27",
            "2026-13-01T00:00:00Z",
            "2026-10-27T00:00:00",
            "2026-10-27T00:00:0éZ",
            "2026-10-27T00:00:00.Z",
            "2026-10-27T00:00:00+1:00",
            "x",
        ] {
            assert_eq!(unix_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn file_round_trips() {
        let f = PlaceFile {
            kind: PlaceKind::Draft,
            taken: 1_793_059_200,
            branches: vec!["spec/x".into(), "propose/q".into()],
        };
        assert_eq!(
            format(&f),
            "kind draft\ntaken 2026-10-27T00:00:00Z\nbranch spec/x\nbranch propose/q\n"
        );
        assert_eq!(parse(&format(&f)), Ok(f.clone()));
        assert_eq!(parse(&format(&f).replace('\n', "\r\n")), Ok(f));
    }
}
