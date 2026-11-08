//! CLM's layered settings: environment, the clone's `clm.local.toml`, the repository's
//! `clm.toml`, the user's `config.toml`, defaults.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, value::StrDeserializer};

/// What `clm defect` does with a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Defects {
    Ask,
    Backlog,
    Off,
}

impl fmt::Display for Defects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Defects::Ask => "ask",
            Defects::Backlog => "backlog",
            Defects::Off => "off",
        })
    }
}

/// Where the repository's remote lives, and which host adapter `clm` uses (`host-adapters`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Host {
    None,
    Git,
    Github,
    Azure,
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Host::None => "none",
            Host::Git => "git",
            Host::Github => "github",
            Host::Azure => "azure",
        })
    }
}

/// Whose approval of a diff counts (`approval-policy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Approval {
    /// Any writer's, the author's included.
    Any,
    /// Only a person's other than the branch's commit authors.
    Other,
}

impl fmt::Display for Approval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Approval::Any => "any",
            Approval::Other => "other",
        })
    }
}

/// Where a setting's value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// The named environment variable.
    Env(&'static str),
    /// The clone's `clm.local.toml`, the repository's `clm.toml` or the user's `config.toml`.
    File(PathBuf),
    Default,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Env(var) => write!(f, "env {var}"),
            Layer::File(path) => write!(f, "{}", path.display()),
            Layer::Default => f.write_str("default"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting<T> {
    pub value: T,
    pub layer: Layer,
}

/// A dead-code detector `clm check` runs: a `[[detectors]]` table in `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detector {
    /// Never empty.
    pub name: String,
    /// The program and its arguments; never empty.
    pub command: Vec<String>,
    /// How to install the program.
    pub hint: Option<String>,
    /// How many seconds `clm check` lets it run before stopping it (`detector-timeouts`); at
    /// least 1.
    pub timeout: u64,
    /// A path relative to the repository root; `clm check` skips the detector while nothing
    /// exists there (`detector-requires-manifest`). Never empty.
    pub requires: Option<String>,
}

/// A detector's `timeout` when its table sets none.
const DETECTOR_TIMEOUT: u64 = 600;

/// A `[[detectors]]` table as written, checked by `detectors`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDetector {
    /// A missing one reads as empty, so `detectors` reports it.
    #[serde(default)]
    name: String,
    command: Vec<String>,
    hint: Option<String>,
    /// Read raw so an invalid `timeout`'s error names `detectors`, which toml's own wouldn't.
    timeout: Option<toml::Value>,
    /// Read raw so an invalid `requires`'s error names `detectors`.
    requires: Option<toml::Value>,
}

/// The size in tokens above which `clm sync` files a split item for a node whose
/// frontmatter sets no `size_budget` (`node-size-budgets`): `[component_size]` in `clm.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeSize {
    pub spec_tokens: u64,
    pub code_tokens: u64,
}

impl Default for NodeSize {
    fn default() -> Self {
        NodeSize {
            spec_tokens: 12000,
            code_tokens: 40000,
        }
    }
}

impl fmt::Display for NodeSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "spec_tokens={},code_tokens={}",
            self.spec_tokens, self.code_tokens
        )
    }
}

/// What `clm health` flags and `clm sync` reconsiders (`health-advises`,
/// `lcov-coverage-signal`): `[health]` in `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// The score at which `clm sync` files a reconsider item for a requirement.
    pub reconsider: u64,
    /// The scenario count above which `clm health` flags a requirement.
    pub scenarios: u64,
    /// The exception-clause count above which `clm health` flags a requirement.
    pub exceptions: u64,
    /// The days after which an open `question` or `human` item is reported.
    pub stale_days: u64,
    /// The lcov file `clm health` reads, relative to the repository root.
    pub lcov: Option<String>,
}

impl Default for Health {
    fn default() -> Self {
        Health {
            reconsider: 3,
            scenarios: 6,
            exceptions: 4,
            stale_days: 30,
            lcov: None,
        }
    }
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "reconsider={},scenarios={},exceptions={},stale_days={}",
            self.reconsider, self.scenarios, self.exceptions, self.stale_days
        )?;
        if let Some(lcov) = &self.lcov {
            write!(f, ",lcov={lcov}")?;
        }
        Ok(())
    }
}

/// When `clm friction` stops a session to ask about a redesign (`redesign-before-workaround`):
/// `[effort]` in `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effort {
    /// The unsettled workaround flags on one capability at which `clm friction` exits 3.
    pub workaround_flag: u64,
}

impl Default for Effort {
    fn default() -> Self {
        Effort { workaround_flag: 3 }
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "workaround_flag={}", self.workaround_flag)
    }
}

/// What `clm land` runs in the tree it lands, before `clm check` (`land-locally`): `[land]`
/// in `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Land {
    /// A shell command; never empty.
    pub check: String,
    /// The size in gigabytes above which a land removes the shared target, when the table sets it.
    pub target_cap_gb: Option<u64>,
    /// The seconds a land's check aims to stay within, when the table sets it.
    pub check_target_seconds: Option<u64>,
}

/// The cap a land applies when `[land]` doesn't set `target_cap_gb`.
const DEFAULT_TARGET_CAP_GB: u64 = 15;

/// The check target a land applies when `[land]` doesn't set `check_target_seconds`.
const DEFAULT_CHECK_TARGET_SECONDS: u64 = 120;

impl Land {
    /// The target cap in gigabytes: the table's `target_cap_gb`, else 15.
    pub fn cap_gb(&self) -> u64 {
        self.target_cap_gb.unwrap_or(DEFAULT_TARGET_CAP_GB)
    }

    /// The check target in seconds: the table's `check_target_seconds`, else 120.
    pub fn target_seconds(&self) -> u64 {
        self.check_target_seconds
            .unwrap_or(DEFAULT_CHECK_TARGET_SECONDS)
    }
}

/// The changed files that make `clm affected` select every test although a node's `paths` hold
/// them (`graph-test-selection`): `[affected]` in `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affected {
    /// Globs written as in a node's `paths`; none is empty.
    pub all: Vec<String>,
}

/// Where the slow tests run and who records the result (`verify-after-land`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode {
    /// `clm verify` runs the command on this machine.
    Local,
    /// A host pipeline runs it and records the result.
    Pipeline,
    /// `clm verify` reads the runs of the pipeline `workflow` names.
    Import,
}

impl VerifyMode {
    fn name(self) -> &'static str {
        match self {
            VerifyMode::Local => "local",
            VerifyMode::Pipeline => "pipeline",
            VerifyMode::Import => "import",
        }
    }
}

/// A moment at which the slow tests run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyTrigger {
    ChangeDone,
    QueueDrained,
    /// Only with `VerifyMode::Local`.
    WorkDone,
}

impl VerifyTrigger {
    fn name(self) -> &'static str {
        match self {
            VerifyTrigger::ChangeDone => "change-done",
            VerifyTrigger::QueueDrained => "queue-drained",
            VerifyTrigger::WorkDone => "work-done",
        }
    }
}

/// The slow tests `clm verify` runs after the land (`verify-after-land`): `[verify]` in
/// `clm.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verify {
    /// The slow-test command; never empty.
    pub check: String,
    pub mode: VerifyMode,
    /// Distinct, in the order written.
    pub triggers: Vec<VerifyTrigger>,
    /// A run is due after this many lands, when set; at least 1.
    pub every_lands: Option<u64>,
    /// A run is due after this many days, when set; at least 1.
    pub every_days: Option<u64>,
    /// The host pipeline whose runs `clm verify` reads; set exactly with `VerifyMode::Import`.
    pub workflow: Option<String>,
}

impl fmt::Display for Verify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mode={},check={}", self.mode.name(), self.check)?;
        if !self.triggers.is_empty() {
            let names: Vec<&str> = self.triggers.iter().map(|t| t.name()).collect();
            write!(f, ",triggers={}", names.join("+"))?;
        }
        if let Some(lands) = self.every_lands {
            write!(f, ",every.lands={lands}")?;
        }
        if let Some(days) = self.every_days {
            write!(f, ",every.days={days}")?;
        }
        if let Some(workflow) = &self.workflow {
            write!(f, ",workflow={workflow}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub autosync: Setting<bool>,
    pub defects: Setting<Defects>,
    /// How many builds and drafts may hold a place on the clone at once (`build-places`).
    pub max_builds: Setting<u64>,
    /// The person part of the ledger writer identity (`per-writer-ledgers`). None: the caller
    /// derives it from git's `user.name`, so this crate runs no git.
    pub writer: Setting<Option<String>>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub detectors: Setting<Vec<Detector>>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub host: Setting<Host>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub approval: Setting<Approval>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone. None: `clm check` alone.
    pub land: Setting<Option<Land>>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone. None: slow tests stay in
    /// `[land] check`.
    pub verify: Setting<Option<Verify>>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub node_size: Setting<NodeSize>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone. None: no glob.
    pub affected: Setting<Option<Affected>>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub health: Setting<Health>,
    /// Repository-only: set by `clm.toml` or `clm.local.toml` alone.
    pub effort: Setting<Effort>,
    /// Repository-only, `clm.toml` alone (`upgrade-settings`): the last migration taken. None:
    /// unset.
    pub upgraded: Setting<Option<String>>,
    /// `clm.toml` alone: the advised migrations the repository declined. None: unset.
    pub declined_migrations: Setting<Option<Vec<String>>>,
    /// `clm.local.toml` alone: required migrations this clone runs past. None: unset.
    pub skip_migrations: Setting<Option<Vec<String>>>,
}

/// Every key `Settings::get` accepts, in the order `clm config list` prints them (`config-list`).
pub const KEYS: [&str; 16] = [
    "autosync",
    "defects",
    "max_builds",
    "writer",
    "host",
    "approval",
    "land",
    "verify",
    "affected",
    "detectors",
    "component_size",
    "health",
    "effort",
    "upgraded",
    "declined_migrations",
    "skip_migrations",
];

impl Settings {
    /// One line per key of `KEYS`: the key, then what `get` gives, tab-separated (`config-list`).
    pub fn list(&self) -> Result<Vec<String>, Error> {
        KEYS.iter()
            .map(|key| {
                let (value, layer) = self.get(key)?;
                Ok(format!("{key}\t{value}\t{layer}"))
            })
            .collect()
    }

    /// The key's value as its TOML value (for `detectors`, the names, comma-separated), and
    /// the layer it came from.
    pub fn get(&self, key: &str) -> Result<(String, &Layer), Error> {
        if !KEYS.contains(&key) {
            return Err(Error::new(None, format!("unknown key `{key}`")));
        }
        match key {
            "autosync" => Ok((self.autosync.value.to_string(), &self.autosync.layer)),
            "defects" => Ok((self.defects.value.to_string(), &self.defects.layer)),
            "max_builds" => Ok((self.max_builds.value.to_string(), &self.max_builds.layer)),
            "writer" => {
                let value = self.writer.value.as_deref().unwrap_or("(git user.name)");
                Ok((value.to_owned(), &self.writer.layer))
            }
            "detectors" => {
                let names: Vec<&str> = self.detectors.value.iter().map(|d| &*d.name).collect();
                Ok((names.join(","), &self.detectors.layer))
            }
            "host" => Ok((self.host.value.to_string(), &self.host.layer)),
            "approval" => Ok((self.approval.value.to_string(), &self.approval.layer)),
            "land" => {
                let value = match &self.land.value {
                    Some(land) => {
                        let mut text = format!("check={}", land.check);
                        if let Some(cap) = land.target_cap_gb {
                            text.push_str(&format!(",target_cap_gb={cap}"));
                        }
                        if let Some(seconds) = land.check_target_seconds {
                            text.push_str(&format!(",check_target_seconds={seconds}"));
                        }
                        text
                    }
                    None => String::new(),
                };
                Ok((value, &self.land.layer))
            }
            "verify" => {
                let value = self.verify.value.as_ref().map(Verify::to_string);
                Ok((value.unwrap_or_default(), &self.verify.layer))
            }
            "component_size" => Ok((self.node_size.value.to_string(), &self.node_size.layer)),
            "affected" => {
                let value = match &self.affected.value {
                    Some(affected) => format!("all={}", affected.all.join(",")),
                    None => String::new(),
                };
                Ok((value, &self.affected.layer))
            }
            "health" => Ok((self.health.value.to_string(), &self.health.layer)),
            "effort" => Ok((self.effort.value.to_string(), &self.effort.layer)),
            "upgraded" => {
                let value = self.upgraded.value.clone().unwrap_or_default();
                Ok((value, &self.upgraded.layer))
            }
            "declined_migrations" => {
                let value = self.declined_migrations.value.clone().unwrap_or_default();
                Ok((value.join(","), &self.declined_migrations.layer))
            }
            "skip_migrations" => {
                let value = self.skip_migrations.value.clone().unwrap_or_default();
                Ok((value.join(","), &self.skip_migrations.layer))
            }
            _ => Err(Error::new(None, format!("unknown key `{key}`"))),
        }
    }
}

/// A file or variable that can't be read, naming it and the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The file's path or `<VARIABLE>=<value>`; none when no layer is at fault.
    pub source: Option<String>,
    /// What's wrong, naming the key.
    pub message: String,
}

impl Error {
    fn new(source: Option<String>, message: String) -> Self {
        Error { source, message }
    }

    fn file(path: &Path, message: impl fmt::Display) -> Self {
        Error::new(Some(path.display().to_string()), message.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{source}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}

/// The user's `config.toml`: the keys anyone may set, so a repository-only key is unknown.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct UserFile {
    autosync: Option<bool>,
    defects: Option<Defects>,
    max_builds: Option<toml::Value>,
    writer: Option<String>,
}

/// The repository's `clm.toml`: the user's keys but `writer`, which names a person, plus the
/// repository-only ones. Not a flattened `UserFile`, since `deny_unknown_fields` doesn't work
/// with `flatten`.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoFile {
    autosync: Option<bool>,
    defects: Option<Defects>,
    max_builds: Option<toml::Value>,
    detectors: Option<Vec<RawDetector>>,
    host: Option<Host>,
    approval: Option<Approval>,
    land: Option<toml::Value>,
    /// Read raw, as `land` is.
    verify: Option<toml::Value>,
    component_size: Option<toml::Value>,
    /// The table's old name, read only to refuse it by name.
    node_size: Option<toml::Value>,
    affected: Option<toml::Value>,
    health: Option<toml::Value>,
    effort: Option<toml::Value>,
    /// Read raw so an invalid value's error names the key.
    upgraded: Option<toml::Value>,
    /// Read raw, as `upgraded` is.
    declined_migrations: Option<toml::Value>,
}

/// The clone's `clm.local.toml`, which may set every key; the other files are read into it.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalFile {
    autosync: Option<bool>,
    defects: Option<Defects>,
    /// Read raw, as `component_size` is.
    max_builds: Option<toml::Value>,
    writer: Option<String>,
    detectors: Option<Vec<RawDetector>>,
    host: Option<Host>,
    approval: Option<Approval>,
    /// Read raw, as `component_size` is.
    land: Option<toml::Value>,
    /// Read raw, as `component_size` is.
    verify: Option<toml::Value>,
    /// Read raw so every error `component_size` reports names it, which toml's own don't.
    component_size: Option<toml::Value>,
    /// The table's old name, read only to refuse it by name.
    node_size: Option<toml::Value>,
    /// Read raw, as `component_size` is.
    affected: Option<toml::Value>,
    /// Read raw, as `component_size` is.
    health: Option<toml::Value>,
    /// Read raw, as `component_size` is.
    effort: Option<toml::Value>,
    /// The repository's keys, filled from `clm.toml` only: skipped here, so `clm.local.toml`
    /// setting one is an unknown key (`upgrade-settings`).
    #[serde(skip)]
    upgraded: Option<toml::Value>,
    #[serde(skip)]
    declined_migrations: Option<toml::Value>,
    /// Read raw, as `component_size` is; from `clm.local.toml` alone, so `RepoFile` has none.
    skip_migrations: Option<toml::Value>,
}

impl From<UserFile> for LocalFile {
    fn from(f: UserFile) -> Self {
        LocalFile {
            autosync: f.autosync,
            defects: f.defects,
            max_builds: f.max_builds,
            writer: f.writer,
            ..LocalFile::default()
        }
    }
}

impl From<RepoFile> for LocalFile {
    fn from(f: RepoFile) -> Self {
        LocalFile {
            autosync: f.autosync,
            defects: f.defects,
            max_builds: f.max_builds,
            writer: None,
            detectors: f.detectors,
            host: f.host,
            approval: f.approval,
            land: f.land,
            verify: f.verify,
            component_size: f.component_size,
            node_size: f.node_size,
            affected: f.affected,
            health: f.health,
            effort: f.effort,
            upgraded: f.upgraded,
            declined_migrations: f.declined_migrations,
            skip_migrations: None,
        }
    }
}

/// Reads `key` from the process environment, for `load`'s `env`. Lossy, so a variable that
/// isn't UTF-8 is set to a value no key accepts rather than unset.
pub fn process_env(key: &str) -> Option<String> {
    std::env::var_os(key).map(|v| v.to_string_lossy().into_owned())
}

/// The `component_size` a repository `clm.toml` holding `text` gives when no `clm.local.toml` or
/// variable sets it, for the `web` Worker, which has the file's text but no clone. It reads the
/// whole text as `load` reads `clm.toml`, so it fails, naming `clm.toml`, wherever `load` would.
pub fn node_size_from(text: &str) -> Result<NodeSize, Error> {
    let path = Path::new("clm.toml");
    let file: RepoFile = parse(text, path)?;
    let mut settings = defaults();
    apply(&mut settings, file.into(), path)?;
    Ok(settings.node_size.value)
}

/// Every key at its default.
fn defaults() -> Settings {
    fn default<T>(value: T) -> Setting<T> {
        Setting {
            value,
            layer: Layer::Default,
        }
    }
    Settings {
        autosync: default(true),
        defects: default(Defects::Ask),
        max_builds: default(3),
        writer: default(None),
        detectors: default(Vec::new()),
        host: default(Host::Github),
        approval: default(Approval::Any),
        land: default(None),
        verify: default(None),
        node_size: default(NodeSize::default()),
        affected: default(None),
        health: default(Health::default()),
        effort: default(Effort::default()),
        upgraded: default(None),
        declined_migrations: default(None),
        skip_migrations: default(None),
    }
}

/// Loads the settings for the repository at `root` (or none), reading variables, including
/// `XDG_CONFIG_HOME` and `HOME`, only through `env`.
pub fn load(root: Option<&Path>, env: &dyn Fn(&str) -> Option<String>) -> Result<Settings, Error> {
    let mut settings = defaults();
    // Lowest layer first, so each file overrides the ones before it.
    if let Some(path) = user_file(env) {
        let file: UserFile = read(&path)?;
        apply(&mut settings, file.into(), &path)?;
    }
    if let Some(root) = root {
        let path = root.join("clm.toml");
        let file: RepoFile = read(&path)?;
        apply(&mut settings, file.into(), &path)?;
        if let Some(path) = local_file(root) {
            let file: LocalFile = read(&path)?;
            apply(&mut settings, file, &path)?;
        }
    }
    if let Some(v) = env("CLM_AUTOSYNC") {
        let value = match v.as_str() {
            "0" | "false" => false,
            "1" | "true" => true,
            _ => {
                return Err(Error::new(
                    Some(format!("CLM_AUTOSYNC={v}")),
                    "`autosync` expects 0, false, 1 or true".into(),
                ));
            }
        };
        set(
            &mut settings.autosync,
            Some(value),
            Layer::Env("CLM_AUTOSYNC"),
        );
    }
    if let Some(v) = env("CLM_DEFECTS") {
        // The file's spellings, through serde, so the two layers can't disagree.
        let value =
            Defects::deserialize(StrDeserializer::<de::value::Error>::new(&v)).map_err(|e| {
                Error::new(
                    Some(format!("CLM_DEFECTS={v}")),
                    format!("bad `defects`: {e}"),
                )
            })?;
        set(
            &mut settings.defects,
            Some(value),
            Layer::Env("CLM_DEFECTS"),
        );
    }
    if let Some(v) = env("CLM_MAX_BUILDS") {
        // Decimal digits only, so `+2` or ` 2` fail as they would in a file.
        let digits = !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit());
        let value = at_least_one(digits.then(|| v.parse().ok()).flatten()).ok_or_else(|| {
            Error::new(
                Some(format!("CLM_MAX_BUILDS={v}")),
                MAX_BUILDS_EXPECTED.into(),
            )
        })?;
        set(
            &mut settings.max_builds,
            Some(value),
            Layer::Env("CLM_MAX_BUILDS"),
        );
    }
    if let Some(v) = env("CLM_WRITER") {
        if !is_slug(&v) {
            return Err(Error::new(Some(format!("CLM_WRITER={v}")), not_a_slug(&v)));
        }
        set(
            &mut settings.writer,
            Some(Some(v)),
            Layer::Env("CLM_WRITER"),
        );
    }
    Ok(settings)
}

fn set<T>(setting: &mut Setting<T>, value: Option<T>, layer: Layer) {
    if let Some(value) = value {
        *setting = Setting { value, layer };
    }
}

/// Sets every key the file at `path` sets, each whole, failing on a value it doesn't accept.
fn apply(settings: &mut Settings, file: LocalFile, path: &Path) -> Result<(), Error> {
    let layer = || Layer::File(path.to_path_buf());
    if let Some(writer) = &file.writer
        && !is_slug(writer)
    {
        return Err(Error::file(path, not_a_slug(writer)));
    }
    let max_builds = file.max_builds.map(|v| max_builds(v, path)).transpose()?;
    let land = file.land.map(|v| land(v, path)).transpose()?;
    let verify = file.verify.map(|v| verify(v, path)).transpose()?;
    if file.node_size.is_some() {
        return Err(Error::file(
            path,
            "`node_size` is the old name of `component_size`; rename the table to `component_size`",
        ));
    }
    let node_size = file
        .component_size
        .map(|v| component_size(v, path))
        .transpose()?;
    let affected = file.affected.map(|v| affected(v, path)).transpose()?;
    let health = file.health.map(|v| health(v, path)).transpose()?;
    let effort = file.effort.map(|v| effort(v, path)).transpose()?;
    let detectors = file.detectors.map(|d| detectors(d, path)).transpose()?;
    let upgraded = file.upgraded.map(|v| upgraded(v, path)).transpose()?;
    let declined = file
        .declined_migrations
        .map(|v| migrations("declined_migrations", v, path))
        .transpose()?;
    let skipped = file
        .skip_migrations
        .map(|v| migrations("skip_migrations", v, path))
        .transpose()?;
    set(&mut settings.autosync, file.autosync, layer());
    set(&mut settings.defects, file.defects, layer());
    set(&mut settings.max_builds, max_builds, layer());
    set(&mut settings.writer, file.writer.map(Some), layer());
    set(&mut settings.detectors, detectors, layer());
    set(&mut settings.host, file.host, layer());
    set(&mut settings.approval, file.approval, layer());
    set(&mut settings.land, land.map(Some), layer());
    set(&mut settings.verify, verify.map(Some), layer());
    set(&mut settings.node_size, node_size, layer());
    set(&mut settings.affected, affected.map(Some), layer());
    set(&mut settings.health, health, layer());
    set(&mut settings.effort, effort, layer());
    set(&mut settings.upgraded, upgraded.map(Some), layer());
    set(
        &mut settings.declined_migrations,
        declined.map(Some),
        layer(),
    );
    set(&mut settings.skip_migrations, skipped.map(Some), layer());
    Ok(())
}

/// The path of the clone's `clm.local.toml` that loading the settings for `root` reads, whether
/// or not the file exists (`local-settings-path`). None when the clone has no main worktree, as
/// a bare repository's has none.
pub fn local_settings_path(root: &Path) -> Option<PathBuf> {
    local_file(root)
}

/// The clone's `clm.local.toml` for the repository at `root`: in the repository's directory in
/// the clone's main worktree, found from the files git keeps, without running git. None when
/// the clone's common git directory isn't a `.git`, as a bare repository's isn't.
fn local_file(root: &Path) -> Option<PathBuf> {
    const NAME: &str = "clm.local.toml";
    let beside = || Some(root.join(NAME));
    let Some(top) = root.ancestors().find(|dir| dir.join(".git").exists()) else {
        return beside();
    };
    let dot_git = top.join(".git");
    if dot_git.is_dir() {
        return beside();
    }
    let rel = root.strip_prefix(top).ok()?;
    let Some(gitdir) = std::fs::read_to_string(&dot_git).ok().and_then(|text| {
        let dir = text.lines().find_map(|l| l.strip_prefix("gitdir: "))?;
        Some(top.join(dir.trim()))
    }) else {
        return beside();
    };
    let Ok(commondir) = std::fs::read_to_string(gitdir.join("commondir")) else {
        return beside();
    };
    let Ok(common) = gitdir.join(commondir.trim()).canonicalize() else {
        return beside();
    };
    if common.file_name()? != ".git" {
        return None;
    }
    Some(common.parent()?.join(rel).join(NAME))
}

/// Lowercase letters, digits and `-`, and not empty.
fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn not_a_slug(value: &str) -> String {
    format!("`writer` expects a slug of lowercase letters, digits and `-`, not {value:?}")
}

/// The detectors a file sets, failing on one `clm check` couldn't name, run or time.
fn detectors(raw: Vec<RawDetector>, path: &Path) -> Result<Vec<Detector>, Error> {
    let mut detectors = Vec::with_capacity(raw.len());
    for (n, d) in raw.into_iter().enumerate() {
        let fail = |problem: String| {
            Error::file(path, format!("`detectors`: detector {} {problem}", n + 1))
        };
        if d.name.is_empty() {
            return Err(fail("has no name".into()));
        }
        if d.command.is_empty() {
            return Err(fail("has an empty command".into()));
        }
        let timeout = match d.timeout {
            None => DETECTOR_TIMEOUT,
            Some(value) => at_least_one(value.as_integer()).ok_or_else(|| {
                fail(format!(
                    "has `timeout` {value}: expected a whole number of seconds of at least 1"
                ))
            })?,
        };
        let requires = match d.requires {
            None => None,
            Some(toml::Value::String(s)) if !s.is_empty() => Some(s),
            Some(value) => {
                return Err(fail(format!(
                    "has `requires` {value}: expected a non-empty path"
                )));
            }
        };
        detectors.push(Detector {
            name: d.name,
            command: d.command,
            hint: d.hint,
            timeout,
            requires,
        });
    }
    Ok(detectors)
}

/// Reads `upgraded`: a string.
fn upgraded(value: toml::Value, path: &Path) -> Result<String, Error> {
    match value {
        toml::Value::String(s) => Ok(s),
        other => Err(Error::file(
            path,
            format!("`upgraded` must be a string, not {other}"),
        )),
    }
}

/// Reads a migration list, `declined_migrations` or `skip_migrations`: an array of strings.
fn migrations(key: &str, value: toml::Value, path: &Path) -> Result<Vec<String>, Error> {
    let list = value.as_array().and_then(|items| {
        items
            .iter()
            .map(|i| i.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
    });
    list.ok_or_else(|| {
        Error::file(
            path,
            format!("`{key}` must be an array of strings, not {value}"),
        )
    })
}

const MAX_BUILDS_EXPECTED: &str = "`max_builds` expects a whole number of at least 1";

/// Reads `max_builds`: an integer of at least 1.
fn max_builds(value: toml::Value, path: &Path) -> Result<u64, Error> {
    at_least_one(value.as_integer())
        .ok_or_else(|| Error::file(path, format!("{MAX_BUILDS_EXPECTED}, not {value}")))
}

/// A whole number of at least 1: the rule of `max_builds`, detector `timeout`s and the counts
/// of `[component_size]`, `[health]` and `[effort]`.
fn at_least_one(n: Option<i64>) -> Option<u64> {
    n.and_then(|n| u64::try_from(n).ok()).filter(|&n| n >= 1)
}

/// Reads `[land]`: a table with a non-empty string `check` and an optional positive integer
/// `target_cap_gb` and `check_target_seconds`.
fn land(value: toml::Value, path: &Path) -> Result<Land, Error> {
    let fail = |problem: String| Error::file(path, format!("`land`: {problem}"));
    let toml::Value::Table(table) = value else {
        return Err(fail(format!("must be a table, not {value}")));
    };
    if let Some(key) = table.keys().find(|k| {
        !matches!(
            k.as_str(),
            "check" | "target_cap_gb" | "check_target_seconds"
        )
    }) {
        return Err(fail(format!(
            "unknown key `{key}`, expected `check`, `target_cap_gb` or `check_target_seconds`"
        )));
    }
    let positive = |name: &str| match table.get(name) {
        None => Ok(None),
        Some(toml::Value::Integer(n)) if *n >= 1 => Ok(Some(*n as u64)),
        Some(other) => Err(fail(format!(
            "`{name}` must be a positive integer, not {other}"
        ))),
    };
    let target_cap_gb = positive("target_cap_gb")?;
    let check_target_seconds = positive("check_target_seconds")?;
    match table.get("check") {
        Some(toml::Value::String(check)) if !check.is_empty() => Ok(Land {
            check: check.clone(),
            target_cap_gb,
            check_target_seconds,
        }),
        Some(other) => Err(fail(format!(
            "`check` must be a non-empty string, not {other}"
        ))),
        None => Err(fail("`check` is missing".into())),
    }
}

/// Reads `[verify]`: a table with a non-empty string `check`, a `mode`, optional distinct
/// `triggers`, an optional `every` table of positive integers and a `workflow` that `import`
/// requires and the other modes refuse.
fn verify(value: toml::Value, path: &Path) -> Result<Verify, Error> {
    let fail = |problem: String| Error::file(path, format!("`verify`: {problem}"));
    let toml::Value::Table(table) = value else {
        return Err(fail(format!("must be a table, not {value}")));
    };
    if let Some(key) = table.keys().find(|k| {
        !matches!(
            k.as_str(),
            "check" | "mode" | "triggers" | "every" | "workflow"
        )
    }) {
        return Err(fail(format!(
            "unknown key `{key}`, expected `check`, `mode`, `triggers`, `every` or `workflow`"
        )));
    }
    let check = match table.get("check") {
        Some(toml::Value::String(check)) if !check.is_empty() => check.clone(),
        Some(other) => {
            return Err(fail(format!(
                "`check` must be a non-empty string, not {other}"
            )));
        }
        None => return Err(fail("`check` is missing".into())),
    };
    let mode = match table.get("mode").and_then(toml::Value::as_str) {
        Some("local") => VerifyMode::Local,
        Some("pipeline") => VerifyMode::Pipeline,
        Some("import") => VerifyMode::Import,
        _ => match table.get("mode") {
            Some(other) => {
                return Err(fail(format!(
                    "`mode` must be `local`, `pipeline` or `import`, not {other}"
                )));
            }
            None => return Err(fail("`mode` is missing".into())),
        },
    };
    let mut triggers = Vec::new();
    if let Some(value) = table.get("triggers") {
        let Some(items) = value.as_array() else {
            return Err(fail(format!("`triggers` must be an array, not {value}")));
        };
        for item in items {
            let trigger = match item.as_str() {
                Some("change-done") => VerifyTrigger::ChangeDone,
                Some("queue-drained") => VerifyTrigger::QueueDrained,
                Some("work-done") => VerifyTrigger::WorkDone,
                _ => {
                    return Err(fail(format!(
                        "unknown trigger {item}, expected `change-done`, `queue-drained` or \
                         `work-done`"
                    )));
                }
            };
            if triggers.contains(&trigger) {
                return Err(fail(format!("trigger {item} is repeated")));
            }
            if trigger == VerifyTrigger::WorkDone && mode != VerifyMode::Local {
                return Err(fail("trigger `work-done` needs `mode = \"local\"`".into()));
            }
            triggers.push(trigger);
        }
    }
    let (mut every_lands, mut every_days) = (None, None);
    if let Some(value) = table.get("every") {
        let toml::Value::Table(every) = value else {
            return Err(fail(format!("`every` must be a table, not {value}")));
        };
        for (key, value) in every {
            let slot = match key.as_str() {
                "lands" => &mut every_lands,
                "days" => &mut every_days,
                _ => {
                    return Err(fail(format!(
                        "`every`: unknown key `{key}`, expected `lands` or `days`"
                    )));
                }
            };
            *slot = Some(at_least_one(value.as_integer()).ok_or_else(|| {
                fail(format!(
                    "`every`: `{key}` must be a positive integer, not {value}"
                ))
            })?);
        }
    }
    let workflow = match (table.get("workflow"), mode) {
        (Some(toml::Value::String(w)), VerifyMode::Import) if !w.is_empty() => Some(w.clone()),
        (Some(other), VerifyMode::Import) => {
            return Err(fail(format!(
                "`workflow` must be a non-empty string, not {other}"
            )));
        }
        (None, VerifyMode::Import) => {
            return Err(fail("`mode = \"import\"` needs a `workflow`".into()));
        }
        (Some(_), _) => {
            return Err(fail("`workflow` is only for `mode = \"import\"`".into()));
        }
        (None, _) => None,
    };
    Ok(Verify {
        check,
        mode,
        triggers,
        every_lands,
        every_days,
        workflow,
    })
}

/// Reads `[affected]`: a table whose one key, `all`, is an array of non-empty strings.
fn affected(value: toml::Value, path: &Path) -> Result<Affected, Error> {
    let fail = |problem: String| Error::file(path, format!("`affected`: {problem}"));
    let toml::Value::Table(table) = value else {
        return Err(fail(format!("must be a table, not {value}")));
    };
    if let Some(key) = table.keys().find(|k| *k != "all") {
        return Err(fail(format!("unknown key `{key}`, expected `all`")));
    }
    let Some(all) = table.get("all") else {
        return Err(fail("`all` is missing".into()));
    };
    let globs = all.as_array().and_then(|globs| {
        globs
            .iter()
            .map(|g| g.as_str().filter(|g| !g.is_empty()).map(str::to_owned))
            .collect::<Option<Vec<_>>>()
    });
    globs.map(|all| Affected { all }).ok_or_else(|| {
        fail(format!(
            "`all` must be an array of non-empty strings, not {all}"
        ))
    })
}

/// Reads `[component_size]`: a table of optional positive integers, each defaulting.
fn component_size(value: toml::Value, path: &Path) -> Result<NodeSize, Error> {
    let name = "component_size";
    let table = match value {
        toml::Value::Table(table) => table,
        other => {
            return Err(Error::file(
                path,
                format!("`{name}` must be a table, not {other}"),
            ));
        }
    };
    let mut size = NodeSize::default();
    for (key, value) in table {
        let slot = match key.as_str() {
            "spec_tokens" => &mut size.spec_tokens,
            "code_tokens" => &mut size.code_tokens,
            _ => {
                return Err(Error::file(
                    path,
                    format!(
                        "`{name}`: unknown key `{key}`, expected `spec_tokens` or `code_tokens`"
                    ),
                ));
            }
        };
        *slot = at_least_one(value.as_integer()).ok_or_else(|| {
            Error::file(
                path,
                format!("`{name}`: `{key}` must be a positive integer, not {value}"),
            )
        })?;
    }
    Ok(size)
}

/// Whether `lcov` names a path relative to the repository root: at least one ordinary name,
/// otherwise only `.`, no root, prefix or `..`, and no `,`, `=` or control character, which
/// would break `clm config get health`'s `key=value,` line.
fn under_root(lcov: &str) -> bool {
    use std::path::Component;
    let mut named = false;
    for component in Path::new(lcov).components() {
        match component {
            Component::Normal(_) => named = true,
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => return false,
        }
    }
    named && !lcov.chars().any(|c| c == ',' || c == '=' || c.is_control())
}

/// Reads `[health]`: a table of optional positive integers, each defaulting, and an optional
/// `lcov` path under the repository root.
fn health(value: toml::Value, path: &Path) -> Result<Health, Error> {
    let fail = |problem: String| Error::file(path, format!("`health`: {problem}"));
    let toml::Value::Table(table) = value else {
        return Err(fail(format!("must be a table, not {value}")));
    };
    let mut health = Health::default();
    for (key, value) in table {
        let slot = match key.as_str() {
            "reconsider" => &mut health.reconsider,
            "scenarios" => &mut health.scenarios,
            "exceptions" => &mut health.exceptions,
            "stale_days" => &mut health.stale_days,
            "lcov" => {
                let toml::Value::String(lcov) = value else {
                    return Err(fail(format!("`lcov` must be a string, not {value}")));
                };
                if !under_root(&lcov) {
                    return Err(fail(format!(
                        "`lcov` must be a path relative to the repository root (at least one \
                         name, no `..`, no `,`, `=` or control character), not {}",
                        toml::Value::String(lcov)
                    )));
                }
                health.lcov = Some(lcov);
                continue;
            }
            _ => {
                return Err(fail(format!(
                    "unknown key `{key}`, expected `reconsider`, `scenarios`, `exceptions`, \
                     `stale_days` or `lcov`"
                )));
            }
        };
        *slot = at_least_one(value.as_integer())
            .ok_or_else(|| fail(format!("`{key}` must be a positive integer, not {value}")))?;
    }
    Ok(health)
}

/// Reads `[effort]`: a table whose one key, `workaround_flag`, is an optional positive integer.
fn effort(value: toml::Value, path: &Path) -> Result<Effort, Error> {
    let fail = |problem: String| Error::file(path, format!("`effort`: {problem}"));
    let toml::Value::Table(table) = value else {
        return Err(fail(format!("must be a table, not {value}")));
    };
    let mut effort = Effort::default();
    for (key, value) in table {
        if key != "workaround_flag" {
            return Err(fail(format!(
                "unknown key `{key}`, expected `workaround_flag`"
            )));
        }
        effort.workaround_flag = at_least_one(value.as_integer()).ok_or_else(|| {
            fail(format!(
                "`workaround_flag` must be a positive integer, not {value}"
            ))
        })?;
    }
    Ok(effort)
}

/// `${XDG_CONFIG_HOME:-$HOME/.config}/clm/config.toml`, or none when neither is set.
fn user_file(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = match env("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(env("HOME").filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("clm/config.toml"))
}

/// Reads a settings file; a missing one has no keys.
fn read<F: DeserializeOwned + Default>(path: &Path) -> Result<F, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(F::default()),
        Err(e) => return Err(Error::file(path, e)),
    };
    parse(&text, path)
}

/// Parses the text of the settings file at `path`.
fn parse<F: DeserializeOwned>(text: &str, path: &Path) -> Result<F, Error> {
    toml::from_str(text).map_err(|e| Error::file(path, e))
}
