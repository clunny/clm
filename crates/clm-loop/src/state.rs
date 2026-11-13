//! The loop file: the loop's state in `<git common dir>/clm/loop` (`loop-file`).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use clm_git::Failure;
use serde::{Deserialize, Serialize};

/// The loop file's format version, which the loader checks.
const VERSION: u32 = 1;

/// How long a lease stays live after its last renewal.
pub(crate) const LEASE_SECONDS: u64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Build,
    Draft,
    Fold,
    Archive,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Build => "build",
            Kind::Draft => "draft",
            Kind::Fold => "fold",
            Kind::Archive => "archive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Prepared, its `launch` printed, not yet reported with `launched`.
    Planned,
    Running,
    Returned,
    Stopped,
}

/// One run of the loop: a build, a draft, a fold, an archive, or a command the session runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// `build:<component>`, `draft:<change-id>`, `fold:<first-id>`, `archive:<id>`,
    /// `land:<batch>` or `verify`.
    pub name: String,
    /// None for a command the session runs itself (`land:<batch>`, `verify`).
    pub kind: Option<Kind>,
    pub model: String,
    pub state: RunState,
    pub task_id: Option<String>,
    /// Unix seconds.
    pub started: u64,
    pub round: u32,
    /// Stops counted by cause.
    #[serde(default)]
    pub stops: BTreeMap<String, u32>,
    /// Left by a session that is gone, for the next step to relaunch.
    #[serde(default)]
    pub orphan: bool,
    /// A draft's backlog items.
    #[serde(default)]
    pub items: Vec<String>,
    /// The prompt file the run's `launch` names.
    #[serde(default)]
    pub prompt: Option<String>,
    /// The command a `run` action names.
    #[serde(default)]
    pub command: Option<String>,
    /// Whether a returned run landed.
    #[serde(default)]
    pub landed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopMode {
    Drain,
    Now,
}

/// A fold batch waiting for the human's approval, or approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    pub id: String,
    pub approved: bool,
    /// When the batch went to the human, Unix seconds.
    #[serde(default)]
    pub since: u64,
}

/// A `question` item the loop couldn't land yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escalation {
    pub slug: String,
    pub body: String,
    pub since: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conditions {
    pub disk: bool,
    /// Unix seconds at which a `limit` lifts.
    pub limit_until: Option<u64>,
}

/// What an entry of the plan's `next` section stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredNext {
    pub name: String,
    pub kind: Kind,
    pub priority: String,
    pub reason: String,
}

/// An entry of the human's queue as the loop file stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredHuman {
    pub name: String,
    pub kind: String,
    pub what: String,
    /// Unix seconds.
    pub since: u64,
    pub action: String,
}

/// The plan the last step acted on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPlan {
    /// The `origin/main` commit it was computed from.
    pub main: String,
    pub at: u64,
    pub next: Vec<StoredNext>,
    /// The human's queue, oldest first.
    #[serde(default)]
    pub humans: Vec<StoredHuman>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopFile {
    pub version: u32,
    /// The lease's last renewal, Unix seconds.
    pub lease: Option<u64>,
    /// The `origin/main` commit when the loop started.
    pub started_main: Option<String>,
    pub session_model: String,
    pub runs: Vec<Run>,
    /// Backlog items drafts of the loop left.
    pub drafted: Vec<String>,
    /// Changes drafts of the loop left for review: the fold list.
    pub review: Vec<String>,
    pub batches: Vec<Batch>,
    /// The red results of the loop's verify runs.
    pub red_verifies: Vec<String>,
    pub escalations: Vec<Escalation>,
    pub conditions: Conditions,
    pub stop: Option<StopMode>,
    pub plan: Option<StoredPlan>,
    /// The `left` lines of `clm wrap-up` already said.
    #[serde(default)]
    pub said: Vec<String>,
}

impl Default for LoopFile {
    fn default() -> Self {
        LoopFile {
            version: VERSION,
            lease: None,
            started_main: None,
            session_model: String::new(),
            runs: Vec::new(),
            drafted: Vec::new(),
            review: Vec::new(),
            batches: Vec::new(),
            red_verifies: Vec::new(),
            escalations: Vec::new(),
            conditions: Conditions::default(),
            stop: None,
            plan: None,
            said: Vec::new(),
        }
    }
}

impl LoopFile {
    /// Whether the lease is live at `now`.
    pub fn held(&self, now: u64) -> bool {
        self.lease
            .is_some_and(|renewed| now.saturating_sub(renewed) < LEASE_SECONDS)
    }

    pub fn run(&self, name: &str) -> Option<&Run> {
        self.runs.iter().find(|r| r.name == name)
    }

    pub(crate) fn run_mut(&mut self, name: &str) -> Option<&mut Run> {
        self.runs.iter_mut().find(|r| r.name == name)
    }
}

/// Where the loop file and its lock live: `<git common dir>/clm/`, not created.
fn dir_path(root: &Path) -> Result<PathBuf, Failure> {
    Ok(clm_git::common_dir(root)?.join("clm"))
}

/// `dir_path`, created: only what writes the loop file creates it.
fn dir(root: &Path) -> Result<PathBuf, Failure> {
    let dir = dir_path(root)?;
    fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    Ok(dir)
}

fn parse(bytes: &[u8]) -> Result<LoopFile, String> {
    let file: LoopFile = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if file.version == VERSION {
        Ok(file)
    } else {
        Err(format!("version {}", file.version))
    }
}

/// The loop file as it stands, read without a lock or a write: none when the clone has none
/// or it can't be read.
pub fn read(root: &Path) -> Option<LoopFile> {
    let path = dir_path(root).ok()?.join("loop");
    parse(&fs::read(path).ok()?).ok()
}

/// Reads the loop file under its lock, hands it to `change`, and, when that succeeds, writes
/// it back whole through a temporary file and a rename. The lock is held only for this call,
/// never while a subprocess runs. A file that can't be read or parsed is renamed
/// `loop.damaged` and the loop starts afresh, saying so on stderr.
pub fn update<T>(
    root: &Path,
    change: impl FnOnce(&mut LoopFile) -> Result<T, Failure>,
) -> Result<T, Failure> {
    let dir = dir(root)?;
    let io = |path: &Path, e: std::io::Error| Failure::Git(format!("{}: {e}", path.display()));
    let lock_path = dir.join("loop.lock");
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| io(&lock_path, e))?;
    lock.lock().map_err(|e| io(&lock_path, e))?;
    let path = dir.join("loop");
    let mut file = match fs::read(&path) {
        Ok(bytes) => match parse(&bytes) {
            Ok(file) => file,
            Err(why) => {
                let damaged = dir.join("loop.damaged");
                fs::rename(&path, &damaged).map_err(|e| io(&damaged, e))?;
                eprintln!(
                    "clm build: the loop file can't be read ({why}); set aside as {}, started afresh",
                    damaged.display()
                );
                LoopFile::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => LoopFile::default(),
        Err(e) => return Err(io(&path, e)),
    };
    let out = change(&mut file)?;
    let tmp = dir.join("loop.tmp");
    let mut tmp_file = File::create(&tmp).map_err(|e| io(&tmp, e))?;
    let text = serde_json::to_vec_pretty(&file).map_err(|e| Failure::Git(e.to_string()))?;
    tmp_file.write_all(&text).map_err(|e| io(&tmp, e))?;
    tmp_file.sync_all().map_err(|e| io(&tmp, e))?;
    fs::rename(&tmp, &path).map_err(|e| io(&path, e))?;
    Ok(out)
}
