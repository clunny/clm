//! The clone's two advisory locks, exclusive locks of the operating system released when the
//! file is dropped, or by the system when the process dies: one check at a time on the shared
//! `.clm/land` (`land/land-target-lock`, `.clm/land.lock`) and one `[land] check` at a time
//! on the clone (`land/check-lock`, `.clm/check.lock`). The system wakes the waiters of a
//! lock in no stated order, so an ordered queue stands in front of each (`land/lock-queue`):
//! the lock keeps the exclusion, the queue decides who tries it next.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Error;

/// How long a waiter leaves between two reads of the queue.
const POLL: Duration = Duration::from_millis(250);

/// How many priority runs in a row may pass an ordinary one that waits.
const STREAK: u32 = 3;

/// Which ticket a run takes: a priority one goes before every ordinary one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    Priority,
    Ordinary,
}

impl Lane {
    fn name(self) -> &'static str {
        match self {
            Lane::Priority => "priority",
            Lane::Ordinary => "ordinary",
        }
    }
}

/// A held lock. Dropping it removes the run's ticket, if it has one, and releases the lock.
#[derive(Debug)]
pub(crate) struct Held {
    ticket: Option<PathBuf>,
    /// Whether the run had to wait for the lock.
    pub(crate) waited: bool,
    lock: File,
    _ticket_lock: Option<File>,
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(ticket) = &self.ticket {
            let _ = std::fs::remove_file(ticket);
        }
    }
}

/// A live ticket: its file name, `<20 digits>-<lane>`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ticket {
    name: String,
    priority: bool,
}

fn parse(name: &str) -> Option<Ticket> {
    let (digits, lane) = name.split_once('-')?;
    if digits.len() != 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let priority = match lane {
        "priority" => true,
        "ordinary" => false,
        _ => return None,
    };
    Some(Ticket {
        name: name.to_string(),
        priority,
    })
}

/// The tickets in the order their turns come, given the queue's `streak`: the oldest
/// priority ticket, else the oldest ordinary one, except that after `STREAK` priority runs
/// in a row an ordinary ticket that waits goes first.
fn turn_order(waiting: &[Ticket], mut streak: u32) -> Vec<Ticket> {
    let mut left = waiting.to_vec();
    left.sort_by(|a, b| a.name.cmp(&b.name));
    let mut order = Vec::new();
    while !left.is_empty() {
        let ordinary = left.iter().position(|t| !t.priority);
        let priority = left.iter().position(|t| t.priority);
        let at = match (priority, ordinary) {
            (Some(p), Some(o)) => {
                if streak >= STREAK {
                    o
                } else {
                    p
                }
            }
            (Some(i), None) | (None, Some(i)) => i,
            (None, None) => unreachable!("left is not empty"),
        };
        let taken = left.remove(at);
        streak = if taken.priority && left.iter().any(|t| !t.priority) {
            streak + 1
        } else {
            0
        };
        order.push(taken);
    }
    order
}

/// The queue beside a lock: `<name>.queue/` next to `<name>.lock`.
struct Queue {
    dir: PathBuf,
}

impl Queue {
    fn at(lock: &Path) -> Result<Queue, Error> {
        let dir = lock.with_extension("queue");
        std::fs::create_dir_all(&dir)?;
        Ok(Queue { dir })
    }

    /// The short exclusive lock every read and write of the queue's files happens under.
    fn guard(&self) -> Result<File, Error> {
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.dir.join("next.lock"))?;
        file.lock()?;
        Ok(file)
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.join(name)).unwrap_or_default()
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Error> {
        std::fs::write(self.dir.join(name), text)?;
        Ok(())
    }

    fn streak(&self) -> u32 {
        self.read("streak").trim().parse().unwrap_or(0)
    }

    /// The live tickets, removing those of runs that have ended (their lock is free). Call
    /// under the guard.
    fn live(&self) -> Result<Vec<Ticket>, Error> {
        let mut live = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let Some(ticket) = entry.file_name().to_str().and_then(parse) else {
                continue;
            };
            let dead = match File::options().write(true).open(entry.path()) {
                Ok(file) => file.try_lock().is_ok(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                // A ticket we can't open is taken as live: safer than failing every run.
                Err(_) => false,
            };
            if dead {
                let _ = std::fs::remove_file(entry.path());
            } else {
                live.push(ticket);
            }
        }
        Ok(live)
    }
}

/// Takes the lock at `path` (creating it and its directory when missing) in the order the
/// queue sets. A run that finds the lock free and no live ticket takes it at once, saying
/// nothing; otherwise it takes a ticket of `lane` and, while it waits, passes
/// `<prefix>: <n> ahead` to `say` when it begins to wait and each time `<n>` changes. Waits
/// without a time limit.
pub(crate) fn take(
    path: &Path,
    lane: Lane,
    prefix: &str,
    mut say: impl FnMut(&str),
) -> Result<Held, Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    let queue = Queue::at(path)?;

    // At once when nothing is in the way; else a ticket, made under the guard.
    let (name, ticket_path, ticket_file) = {
        let _guard = queue.guard()?;
        if queue.live()?.is_empty() {
            match lock.try_lock() {
                Ok(()) => {
                    queue.write("holder", "")?;
                    queue.write("streak", "0")?;
                    return Ok(Held {
                        ticket: None,
                        waited: false,
                        lock,
                        _ticket_lock: None,
                    });
                }
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(e)) => return Err(e.into()),
            }
        }
        // A lost counter must not hand out a live ticket's name: skip names that exist.
        let mut counter: u64 = queue.read("next").trim().parse().unwrap_or(0);
        let (name, ticket_path, file) = loop {
            let name = format!("{counter:020}-{}", lane.name());
            let ticket_path = queue.dir.join(&name);
            let taken = ["priority", "ordinary"]
                .iter()
                .any(|l| queue.dir.join(format!("{counter:020}-{l}")).exists());
            counter += 1;
            if taken {
                continue;
            }
            let file = File::options()
                .create_new(true)
                .write(true)
                .open(&ticket_path)?;
            break (name, ticket_path, file);
        };
        queue.write("next", &counter.to_string())?;
        file.lock()?;
        (name, ticket_path, file)
    };
    // From here the ticket is removed on every path out.
    let held = Held {
        ticket: Some(ticket_path),
        waited: true,
        lock,
        _ticket_lock: Some(ticket_file),
    };

    let mut said = None;
    loop {
        {
            let _guard = queue.guard()?;
            let live = queue.live()?;
            let holder = queue.read("holder");
            let holder = holder.trim();
            let holding = live.iter().any(|t| t.name == holder);
            let waiting: Vec<Ticket> = live.into_iter().filter(|t| t.name != holder).collect();
            let order = turn_order(&waiting, queue.streak());
            let before = order
                .iter()
                .position(|t| t.name == name)
                .unwrap_or(order.len());
            let free = match held.lock.try_lock() {
                Ok(()) => true,
                Err(TryLockError::WouldBlock) => false,
                Err(TryLockError::Error(e)) => return Err(e.into()),
            };
            if free && before == 0 {
                let streak = if lane == Lane::Priority && order.iter().any(|t| !t.priority) {
                    queue.streak() + 1
                } else {
                    0
                };
                queue.write("streak", &streak.to_string())?;
                queue.write("holder", &name)?;
                return Ok(held);
            }
            if free {
                held.lock.unlock()?;
            }
            // The run holding the lock counts, whether it has a ticket or not.
            let ahead = before + usize::from(holding || !free);
            if said != Some(ahead) {
                said = Some(ahead);
                say(&format!("{prefix}: {ahead} ahead"));
            }
        }
        std::thread::sleep(POLL);
    }
}

/// Locks `<target>.lock`, the land lock.
pub(crate) fn land(target: &Path, lane: Lane) -> Result<Held, Error> {
    let mut path = target.as_os_str().to_owned();
    path.push(".lock");
    let prefix = format!("waiting for another land's check on {}", target.display());
    take(Path::new(&path), lane, &prefix, |line| eprintln!("{line}"))
}

/// Locks `check.lock` in `places`, the check lock, passing each waiting line to `say`.
pub(crate) fn check(places: &Path, lane: Lane, say: impl FnMut(&str)) -> Result<Held, Error> {
    let path = places.join("check.lock");
    let prefix = format!("waiting for another check on {}", path.display());
    take(&path, lane, &prefix, say)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    fn lock_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join(".clm/check.lock")
    }

    /// Holds the lock without a ticket, as an older `clm` would.
    fn outsider(path: &Path) -> File {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = File::create(path).unwrap();
        file.lock().unwrap();
        file
    }

    #[test]
    fn a_free_lock_is_taken_at_once_and_leaves_no_ticket() {
        // req:land/lock-queue@ea656239
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let held = take(&path, Lane::Ordinary, "p", |_| panic!("said")).unwrap();
        assert!(!held.waited);
        drop(held);
        let tickets = std::fs::read_dir(path.with_extension("queue"))
            .unwrap()
            .filter_map(|e| parse(e.unwrap().file_name().to_str().unwrap()))
            .count();
        assert_eq!(tickets, 0);
    }

    #[test]
    fn a_run_outside_the_queue_counts_one_ahead_and_is_waited_for() {
        // req:land/check-lock@a6608578
        // req:land/lock-queue@ea656239
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let held = outsider(&path);
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            drop(held);
        });
        let mut said = Vec::new();
        let start = Instant::now();
        let got = take(&path, Lane::Ordinary, "waiting", |l| {
            said.push(l.to_string())
        })
        .unwrap();
        assert!(got.waited);
        assert!(start.elapsed() >= Duration::from_millis(400));
        assert_eq!(said, ["waiting: 1 ahead"]);
        releaser.join().unwrap();
        drop(got);
        assert!(
            std::fs::read_dir(path.with_extension("queue"))
                .unwrap()
                .all(|e| parse(e.unwrap().file_name().to_str().unwrap()).is_none())
        );
    }

    type Lines = mpsc::Sender<(&'static str, String)>;

    /// Starts a waiter on `path` with `lane` that sends `id` once it holds the lock, and
    /// then releases it.
    fn waiter(
        path: &Path,
        lane: Lane,
        id: &'static str,
        order: &mpsc::Sender<&'static str>,
        lines: &Lines,
    ) -> std::thread::JoinHandle<()> {
        let (path, order, lines) = (path.to_path_buf(), order.clone(), lines.clone());
        std::thread::spawn(move || {
            let held = take(&path, lane, "w", |l| {
                lines.send((id, l.to_string())).unwrap();
            })
            .unwrap();
            order.send(id).unwrap();
            drop(held);
        })
    }

    /// Starts the waiters one after another, each once its first line is said, behind a lock
    /// held outside the queue, releases that lock and returns the order they held it in and
    /// their first lines.
    fn run_in_order(ids: &[(&'static str, Lane)]) -> (Vec<&'static str>, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let outside = outsider(&path);
        let (order_tx, order_rx) = mpsc::channel();
        let (lines_tx, lines_rx) = mpsc::channel();
        let mut handles = Vec::new();
        let mut first = Vec::new();
        for (id, lane) in ids {
            handles.push(waiter(&path, *lane, id, &order_tx, &lines_tx));
            let (who, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            assert_eq!(who, *id);
            first.push(line);
        }
        drop(outside);
        let got = ids
            .iter()
            .map(|_| order_rx.recv_timeout(Duration::from_secs(20)).unwrap())
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        (got, first)
    }

    #[test]
    fn turns_go_in_arrival_order_and_each_line_counts_the_runs_ahead() {
        // req:land/lock-queue@ea656239
        let (got, first) = run_in_order(&[
            ("a", Lane::Ordinary),
            ("b", Lane::Ordinary),
            ("c", Lane::Ordinary),
        ]);
        assert_eq!(got, ["a", "b", "c"]);
        assert_eq!(first, ["w: 1 ahead", "w: 2 ahead", "w: 3 ahead"]);
    }

    #[test]
    fn a_priority_run_goes_first_but_cannot_starve_the_rest() {
        // req:land/lock-queue@ea656239
        let (got, _) = run_in_order(&[
            ("o", Lane::Ordinary),
            ("p1", Lane::Priority),
            ("p2", Lane::Priority),
            ("p3", Lane::Priority),
            ("p4", Lane::Priority),
            ("p5", Lane::Priority),
        ]);
        assert_eq!(got, ["p1", "p2", "p3", "o", "p4", "p5"]);
    }

    #[test]
    fn the_ticket_of_an_ended_run_holds_no_turn() {
        // req:land/lock-queue@ea656239
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let outside = outsider(&path);
        // A waiter whose process ends keeps its ticket file but not the lock on it.
        let queue = path.with_extension("queue");
        std::fs::create_dir_all(&queue).unwrap();
        std::fs::write(queue.join("next"), "1").unwrap();
        let dead_name = queue.join(format!("{:020}-ordinary", 0));
        let dead = File::create(&dead_name).unwrap();
        dead.lock().unwrap();
        let (order_tx, order_rx) = mpsc::channel();
        let (lines_tx, lines_rx) = mpsc::channel();
        let handle = waiter(&path, Lane::Ordinary, "b", &order_tx, &lines_tx);
        let (_, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(line, "w: 2 ahead");
        drop(dead);
        let (_, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(line, "w: 1 ahead");
        drop(outside);
        order_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        handle.join().unwrap();
        assert!(!dead_name.exists());
        assert!(!queue.join(format!("{:020}-ordinary", 1)).exists());
    }
}
