//! The record `<git common dir>/clm/machine`: when the builds were last measured, the largest
//! size any one has had, and each build's last size (`build-measure`).

use std::io::Write;
use std::path::Path;

use crate::time;

/// How long a measurement is reused.
pub const MEASURE_EVERY_SECONDS: u64 = 600;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// Unix seconds of the last measurement.
    pub measured: Option<u64>,
    /// The largest size any one build's worktree has been measured at in this clone.
    pub peak: Option<u64>,
    /// Each build measured that time, with its size.
    pub builds: Vec<(String, u64)>,
}

impl Record {
    /// The record at `path`; one that can't be read or parsed counts as none.
    pub fn load(path: &Path) -> Record {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| Record::parse(&text))
            .unwrap_or_default()
    }

    /// Parses a record, or `None` when a line isn't one.
    pub fn parse(text: &str) -> Option<Record> {
        let mut record = Record::default();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let (key, rest) = line.split_once(' ')?;
            match key {
                "measured" => record.measured = Some(time::parse(rest.trim())?),
                "peak" => record.peak = Some(rest.trim().parse().ok()?),
                "build" => {
                    let (name, bytes) = rest.rsplit_once(' ')?;
                    record.builds.push((name.to_string(), bytes.parse().ok()?));
                }
                _ => return None,
            }
        }
        // A record with no time is no record.
        record.measured?;
        Some(record)
    }

    /// The record after measuring `sizes` at `now`: its `peak` the larger of the old one and
    /// any size measured, so it never falls.
    pub fn measured_at(&self, now: u64, sizes: Vec<(String, u64)>) -> Record {
        let peak = sizes.iter().map(|(_, b)| *b).chain(self.peak).max();
        Record {
            measured: Some(now),
            peak,
            builds: sizes,
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        if let Some(at) = self.measured {
            out.push_str(&format!("measured {}\n", time::format(at)));
        }
        if let Some(peak) = self.peak {
            out.push_str(&format!("peak {peak}\n"));
        }
        for (name, bytes) in &self.builds {
            out.push_str(&format!("build {name} {bytes}\n"));
        }
        out
    }

    /// Writes the record through a temporary file and a rename, so two runs at once never
    /// leave a torn one. What can't be written is left unwritten.
    pub fn store(&self, path: &Path) {
        write_atomically(path, &self.render());
    }
}

/// Writes `text` to `path` through a temporary file beside it and a rename.
pub fn write_atomically(path: &Path, text: &str) {
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(mut file) = tempfile::NamedTempFile::new_in(dir) else {
        return;
    };
    if file.write_all(text.as_bytes()).is_ok() {
        let _ = file.persist(path);
    }
}
