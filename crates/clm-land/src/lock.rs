//! The clone's two advisory locks, exclusive locks of the operating system released when the
//! file is dropped, or by the system when the process dies: one check at a time on the shared
//! `.clm/land` (`land/land-target-lock`, `.clm/land.lock`) and one `[land] check` at a time
//! on the clone (`land/check-lock`, `.clm/check.lock`). The system wakes the waiters of a
//! lock in no stated order, so an ordered queue stands in front of each (`land/lock-queue`):
//! the lock keeps the exclusion, the queue decides who tries it next.

use std::fs::{File, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clm_model::Priority;

use crate::Error;
use crate::stderr::Stderr;

/// A lock's or a queue's file that can't be made, read, written or locked: a failure naming
/// its path, exit 2 (`land/lock-queue`).
fn at(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| Error::Git(format!("{}: {e}", path.display()))
}

/// How long a waiter leaves between two reads of the queue.
const POLL: Duration = Duration::from_millis(250);

/// How many priority-lane runs in a row below `P1` may pass an ordinary one that waits.
const STREAK: u32 = 6;

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

/// What a ticket carries: its lane and its priority (`land/lock-queue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rank {
    pub(crate) lane: Lane,
    pub(crate) priority: Priority,
}

impl Rank {
    /// The rank of every run that isn't a land with a priority of its own.
    pub(crate) const ORDINARY: Rank = Rank {
        lane: Lane::Ordinary,
        priority: Priority::P5,
    };
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
            let _ = std::fs::remove_file(alone_mark(ticket));
            let _ = std::fs::remove_file(ticket);
        }
    }
}

impl Held {
    /// The file name of this run's ticket, none for a run that took the lock with no ticket.
    pub(crate) fn ticket_name(&self) -> Option<String> {
        let ticket = self.ticket.as_ref()?;
        Some(ticket.file_name()?.to_string_lossy().into_owned())
    }

    /// Whether this run's ticket is marked to land alone (`land/failed-batch`).
    pub(crate) fn alone(&self) -> bool {
        self.ticket
            .as_ref()
            .is_some_and(|ticket| alone_mark(ticket).exists())
    }
}

/// The file beside a ticket that marks its land to land alone (`land/failed-batch`).
fn alone_mark(ticket: &Path) -> PathBuf {
    let mut name = ticket.as_os_str().to_owned();
    name.push(".alone");
    PathBuf::from(name)
}

/// A live ticket: its file name, `<20 digits>-<lane>-p<n>`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ticket {
    name: String,
    rank: Rank,
}

impl Ticket {
    fn priority_lane(&self) -> bool {
        self.rank.lane == Lane::Priority
    }
}

fn parse(name: &str) -> Option<Ticket> {
    let mut parts = name.split('-');
    let (digits, lane, priority) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || digits.len() != 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let lane = match lane {
        "priority" => Lane::Priority,
        "ordinary" => Lane::Ordinary,
        _ => return None,
    };
    let priority = match priority {
        "p1" => Priority::P1,
        "p2" => Priority::P2,
        "p3" => Priority::P3,
        "p4" => Priority::P4,
        "p5" => Priority::P5,
        _ => return None,
    };
    Some(Ticket {
        name: name.to_string(),
        rank: Rank { lane, priority },
    })
}

fn ticket_name(counter: u64, rank: Rank) -> String {
    format!(
        "{counter:020}-{}-p{}",
        rank.lane.name(),
        rank.priority.to_string().trim_start_matches('P')
    )
}

/// The index in `left` of the ticket whose turn comes first among `pool`: a priority-lane
/// one before an ordinary one, then the highest priority, then the oldest.
fn best(pool: &[usize], left: &[Ticket]) -> Option<usize> {
    pool.iter().copied().min_by_key(|&i| {
        let t = &left[i];
        (!t.priority_lane(), t.rank.priority, i)
    })
}

/// The tickets in the order their turns come, given the queue's `streak`: the `P1` tickets
/// first, a priority-lane one before an ordinary one and otherwise the oldest; then the
/// other priority-lane tickets, the highest priority first and then the oldest; then the
/// ordinary ones the same way. After `STREAK` priority-lane runs in a row below `P1`, an
/// ordinary ticket that waits goes first, unless a `P1` ticket waits.
fn turn_order(waiting: &[Ticket], mut streak: u32) -> Vec<Ticket> {
    let mut left = waiting.to_vec();
    left.sort_by(|a, b| a.name.cmp(&b.name));
    let mut order = Vec::new();
    while !left.is_empty() {
        let all: Vec<usize> = (0..left.len()).collect();
        let ones: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| left[i].rank.priority == Priority::P1)
            .collect();
        let ordinary: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| !left[i].priority_lane())
            .collect();
        let at = if !ones.is_empty() {
            best(&ones, &left)
        } else if streak >= STREAK && !ordinary.is_empty() {
            best(&ordinary, &left)
        } else {
            best(&all, &left)
        }
        .expect("left is not empty");
        let taken = left.remove(at);
        if taken.rank.priority != Priority::P1 {
            streak = if taken.priority_lane() && left.iter().any(|t| !t.priority_lane()) {
                streak + 1
            } else {
                0
            };
        }
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
        std::fs::create_dir_all(&dir).map_err(at(&dir))?;
        Ok(Queue { dir })
    }

    /// The short exclusive lock every read and write of the queue's files happens under.
    fn guard(&self) -> Result<File, Error> {
        let path = self.dir.join("next.lock");
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(at(&path))?;
        file.lock().map_err(at(&path))?;
        Ok(file)
    }

    /// The text of the queue's file `name`, empty when it is missing; any other failure to
    /// read it names it.
    fn read(&self, name: &str) -> Result<String, Error> {
        let path = self.dir.join(name);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(at(&path)(e)),
        }
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Error> {
        let path = self.dir.join(name);
        std::fs::write(&path, text).map_err(at(&path))
    }

    fn streak(&self) -> Result<u32, Error> {
        Ok(self.read("streak")?.trim().parse().unwrap_or(0))
    }

    /// The live tickets, removing those of runs that have ended (their lock is free). A ticket
    /// that can't be opened, even to read, holds no turn in this reading and stays where it
    /// is: the lock, not the queue, keeps two runs apart. Call under the guard.
    fn live(&self) -> Result<Vec<Ticket>, Error> {
        let mut live = Vec::new();
        for entry in std::fs::read_dir(&self.dir).map_err(at(&self.dir))? {
            let entry = entry.map_err(at(&self.dir))?;
            let Some(ticket) = entry.file_name().to_str().and_then(parse) else {
                continue;
            };
            // Read-only: a lock on a file needs no right to write it.
            let dead = match File::open(entry.path()) {
                Ok(file) => file.try_lock().is_ok(),
                Err(_) => continue,
            };
            if dead {
                let _ = std::fs::remove_file(alone_mark(&entry.path()));
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
/// nothing; otherwise it takes a ticket of `rank` and, while it waits, passes
/// `<prefix>: <n> ahead` to `say` when it begins to wait and each time `<n>` changes. Waits
/// without a time limit.
pub(crate) fn take(
    path: &Path,
    rank: Rank,
    prefix: &str,
    say: impl FnMut(&str),
) -> Result<Held, Error> {
    take_named(path, rank, prefix, "", say)
}

/// As `take`, the ticket holding `content` as the file's text, written before the ticket is
/// locked and before the waiting line is said (`land/batched-lands`).
pub(crate) fn take_named(
    path: &Path,
    rank: Rank,
    prefix: &str,
    content: &str,
    mut say: impl FnMut(&str),
) -> Result<Held, Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(at(parent))?;
    }
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(at(path))?;
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
                Err(TryLockError::Error(e)) => return Err(at(path)(e)),
            }
        }
        // A lost counter must not hand out a live ticket's name: skip names that exist.
        let mut counter: u64 = queue.read("next")?.trim().parse().unwrap_or(0);
        let (name, ticket_path, file) = loop {
            let name = ticket_name(counter, rank);
            let ticket_path = queue.dir.join(&name);
            let digits = format!("{counter:020}-");
            let taken = std::fs::read_dir(&queue.dir)
                .map_err(at(&queue.dir))?
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with(&digits));
            counter += 1;
            if taken {
                continue;
            }
            let mut file = File::options()
                .create_new(true)
                .write(true)
                .open(&ticket_path)
                .map_err(at(&ticket_path))?;
            file.write_all(content.as_bytes())
                .map_err(at(&ticket_path))?;
            break (name, ticket_path, file);
        };
        queue.write("next", &counter.to_string())?;
        file.lock().map_err(at(&ticket_path))?;
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
            let holder = queue.read("holder")?;
            let holder = holder.trim();
            let holding = live.iter().any(|t| t.name == holder);
            let waiting: Vec<Ticket> = live.into_iter().filter(|t| t.name != holder).collect();
            let order = turn_order(&waiting, queue.streak()?);
            let before = order
                .iter()
                .position(|t| t.name == name)
                .unwrap_or(order.len());
            let free = match held.lock.try_lock() {
                Ok(()) => true,
                Err(TryLockError::WouldBlock) => false,
                Err(TryLockError::Error(e)) => return Err(at(path)(e)),
            };
            if free && before == 0 {
                let streak = if rank.priority == Priority::P1 {
                    queue.streak()?
                } else if rank.lane == Lane::Priority && order.iter().any(|t| !t.priority_lane()) {
                    queue.streak()? + 1
                } else {
                    0
                };
                queue.write("streak", &streak.to_string())?;
                queue.write("holder", &name)?;
                return Ok(held);
            }
            if free {
                held.lock.unlock().map_err(at(path))?;
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

/// `<target>.lock`, the land lock.
fn land_lock(target: &Path) -> PathBuf {
    let mut path = target.as_os_str().to_owned();
    path.push(".lock");
    PathBuf::from(path)
}

/// Locks `<target>.lock`, the land lock, its ticket naming `branch` (`land/batched-lands`),
/// saying each waiting line on `err`.
pub(crate) fn land(target: &Path, rank: Rank, branch: &str, err: Stderr) -> Result<Held, Error> {
    let prefix = format!("waiting for another land's check on {}", target.display());
    take_named(&land_lock(target), rank, &prefix, branch, |l| err.line(l))
}

/// A land waiting for the land lock whose ticket names its branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Waiting {
    pub(crate) ticket: String,
    pub(crate) branch: String,
}

/// The lands waiting for the land lock besides the holder's, whose tickets name a branch, in
/// the order their turns would have come; a ticket that names none is left out, and so is one
/// marked to land alone (`land/batched-lands`, `land/failed-batch`).
pub(crate) fn waiting_lands(target: &Path, held: &Held) -> Result<Vec<Waiting>, Error> {
    let queue = Queue::at(&land_lock(target))?;
    let _guard = queue.guard()?;
    let own = held.ticket_name();
    let live: Vec<Ticket> = queue
        .live()?
        .into_iter()
        .filter(|t| Some(&t.name) != own.as_ref())
        .collect();
    let mut out = Vec::new();
    for ticket in turn_order(&live, queue.streak()?) {
        if alone_mark(&queue.dir.join(&ticket.name)).exists() {
            continue;
        }
        let branch = queue.read(&ticket.name)?.trim().to_string();
        if !branch.is_empty() {
            out.push(Waiting {
                ticket: ticket.name,
                branch,
            });
        }
    }
    Ok(out)
}

/// Marks the land waiting with `ticket` to land alone: the file goes with the ticket
/// (`land/failed-batch`).
pub(crate) fn mark_alone(target: &Path, ticket: &str) -> Result<(), Error> {
    let queue = Queue::at(&land_lock(target))?;
    let _guard = queue.guard()?;
    let ticket = queue.dir.join(ticket);
    // A ticket that ended meanwhile leaves no mark behind.
    if ticket.exists() {
        let mark = alone_mark(&ticket);
        std::fs::write(&mark, "").map_err(at(&mark))?;
    }
    Ok(())
}

/// Locks `push.lock` in `places`, the push turn (`land/push-turn`): ordinary tickets, so the
/// turns go in the order they were asked; each waiting line said on `err`.
pub(crate) fn push(places: &Path, err: Stderr) -> Result<Held, Error> {
    let path = places.join("push.lock");
    take(
        &path,
        Rank::ORDINARY,
        "waiting for another land's push",
        |l| err.line(l),
    )
}

/// Locks `check.lock` in `places`, the check lock, passing each waiting line to `say`.
pub(crate) fn check(places: &Path, rank: Rank, say: impl FnMut(&str)) -> Result<Held, Error> {
    let path = places.join("check.lock");
    let prefix = format!("waiting for another check on {}", path.display());
    take(&path, rank, &prefix, say)
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
    fn a_missing_queue_file_reads_as_its_default_and_an_unreadable_one_names_its_path() {
        // req:land/lock-queue@32c5dce4
        let dir = tempfile::tempdir().unwrap();
        let queue = Queue::at(&lock_path(&dir)).unwrap();
        assert_eq!(queue.read("holder").unwrap(), "");
        assert_eq!(queue.streak().unwrap(), 0);
        // A directory where the file would be can't be read as one.
        std::fs::create_dir(queue.dir.join("streak")).unwrap();
        match queue.streak() {
            Err(Error::Git(why)) => assert!(why.contains("check.queue/streak"), "{why}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_free_lock_is_taken_at_once_and_leaves_no_ticket() {
        // req:land/lock-queue@32c5dce4
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let held = take(&path, Rank::ORDINARY, "p", |_| panic!("said")).unwrap();
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
        // req:land/lock-queue@32c5dce4
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let held = outsider(&path);
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            drop(held);
        });
        let mut said = Vec::new();
        let start = Instant::now();
        let got = take(&path, Rank::ORDINARY, "waiting", |l| {
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

    fn rank(lane: Lane, priority: Priority) -> Rank {
        Rank { lane, priority }
    }

    /// Starts a waiter on `path` with `rank` that sends `id` once it holds the lock, and
    /// then releases it.
    fn waiter(
        path: &Path,
        rank: Rank,
        id: &'static str,
        order: &mpsc::Sender<&'static str>,
        lines: &Lines,
    ) -> std::thread::JoinHandle<()> {
        let (path, order, lines) = (path.to_path_buf(), order.clone(), lines.clone());
        std::thread::spawn(move || {
            let held = take(&path, rank, "w", |l| {
                lines.send((id, l.to_string())).unwrap();
            })
            .unwrap();
            order.send(id).unwrap();
            drop(held);
        })
    }

    /// Starts the waiters one after another, each once its first line is said, behind a lock
    /// held outside the queue (and a `streak` already in the queue, if given), releases that
    /// lock and returns the order they held it in and their first lines. A waiter already
    /// started may say a new count while the next one starts, when the newcomer goes before
    /// it: the next start waits for the newcomer's own line, whatever comes first.
    fn run_in_order_after(
        ids: &[(&'static str, Rank)],
        streak: Option<u32>,
    ) -> (Vec<&'static str>, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let outside = outsider(&path);
        if let Some(streak) = streak {
            let queue = path.with_extension("queue");
            std::fs::create_dir_all(&queue).unwrap();
            std::fs::write(queue.join("streak"), streak.to_string()).unwrap();
        }
        let (order_tx, order_rx) = mpsc::channel();
        let (lines_tx, lines_rx) = mpsc::channel();
        let mut handles = Vec::new();
        let mut first = Vec::new();
        for (id, rank) in ids {
            handles.push(waiter(&path, *rank, id, &order_tx, &lines_tx));
            let line = loop {
                let (who, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                if who == *id {
                    break line;
                }
            };
            first.push(line);
        }
        drop(outside);
        let got = ids
            .iter()
            .map(|_| order_rx.recv_timeout(Duration::from_secs(30)).unwrap())
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        (got, first)
    }

    fn run_in_order(ids: &[(&'static str, Rank)]) -> (Vec<&'static str>, Vec<String>) {
        run_in_order_after(ids, None)
    }

    #[test]
    fn turns_go_in_arrival_order_and_each_line_counts_the_runs_ahead() {
        // req:land/lock-queue@32c5dce4
        let (got, first) = run_in_order(&[
            ("a", Rank::ORDINARY),
            ("b", Rank::ORDINARY),
            ("c", Rank::ORDINARY),
        ]);
        assert_eq!(got, ["a", "b", "c"]);
        assert_eq!(first, ["w: 1 ahead", "w: 2 ahead", "w: 3 ahead"]);
    }

    #[test]
    fn priority_lane_runs_go_first_but_cannot_starve_the_rest() {
        // req:land/lock-queue@32c5dce4
        let p = rank(Lane::Priority, Priority::P5);
        let (got, _) = run_in_order(&[
            ("o", Rank::ORDINARY),
            ("p1", p),
            ("p2", p),
            ("p3", p),
            ("p4", p),
            ("p5", p),
            ("p6", p),
            ("p7", p),
            ("p8", p),
        ]);
        assert_eq!(got, ["p1", "p2", "p3", "p4", "p5", "p6", "o", "p7", "p8"]);
    }

    #[test]
    fn p1_goes_first_priority_lane_before_ordinary() {
        // req:land/lock-queue@32c5dce4
        let (got, _) = run_in_order(&[
            ("pri5", rank(Lane::Priority, Priority::P5)),
            ("ord1", rank(Lane::Ordinary, Priority::P1)),
            ("pri1", rank(Lane::Priority, Priority::P1)),
        ]);
        assert_eq!(got, ["pri1", "ord1", "pri5"]);
    }

    #[test]
    fn below_p1_the_priority_lane_goes_first_then_the_highest_priority() {
        // req:land/lock-queue@32c5dce4
        let (got, _) = run_in_order(&[
            ("ord2", rank(Lane::Ordinary, Priority::P2)),
            ("pri4", rank(Lane::Priority, Priority::P4)),
            ("ord3", rank(Lane::Ordinary, Priority::P3)),
            ("pri3", rank(Lane::Priority, Priority::P3)),
        ]);
        assert_eq!(got, ["pri3", "pri4", "ord2", "ord3"]);
    }

    #[test]
    fn p1_does_not_wait_for_the_cap() {
        // req:land/lock-queue@32c5dce4
        let (got, _) = run_in_order_after(
            &[
                ("ord5", Rank::ORDINARY),
                ("ord1", rank(Lane::Ordinary, Priority::P1)),
            ],
            Some(STREAK),
        );
        assert_eq!(got, ["ord1", "ord5"]);
    }

    #[test]
    fn a_priority_lane_p1_run_goes_before_the_cap_runs() {
        // req:land/lock-queue@32c5dce4
        let (got, _) = run_in_order_after(
            &[
                ("ord5", Rank::ORDINARY),
                ("pri1", rank(Lane::Priority, Priority::P1)),
            ],
            Some(STREAK),
        );
        assert_eq!(got, ["pri1", "ord5"]);
    }

    #[test]
    fn the_cap_lets_an_ordinary_run_pass_after_six_below_p1() {
        // req:land/lock-queue@32c5dce4
        let (got, _) = run_in_order_after(
            &[
                ("ord5", Rank::ORDINARY),
                ("pri5", rank(Lane::Priority, Priority::P5)),
            ],
            Some(STREAK),
        );
        assert_eq!(got, ["ord5", "pri5"]);
    }

    #[test]
    fn the_ticket_of_an_ended_run_holds_no_turn() {
        // req:land/lock-queue@32c5dce4
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let outside = outsider(&path);
        // A waiter whose process ends keeps its ticket file but not the lock on it.
        let queue = path.with_extension("queue");
        std::fs::create_dir_all(&queue).unwrap();
        std::fs::write(queue.join("next"), "1").unwrap();
        let dead_name = queue.join(ticket_name(0, Rank::ORDINARY));
        let dead = File::create(&dead_name).unwrap();
        dead.lock().unwrap();
        let (order_tx, order_rx) = mpsc::channel();
        let (lines_tx, lines_rx) = mpsc::channel();
        let handle = waiter(&path, Rank::ORDINARY, "b", &order_tx, &lines_tx);
        let (_, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(line, "w: 2 ahead");
        drop(dead);
        let (_, line) = lines_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(line, "w: 1 ahead");
        drop(outside);
        order_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        handle.join().unwrap();
        assert!(!dead_name.exists());
        assert!(!queue.join(ticket_name(1, Rank::ORDINARY)).exists());
    }
}
