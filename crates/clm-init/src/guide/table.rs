//! The guide's table of pages, read from `docs/guide/` when `clm-init` builds
//! (`guide-table-from-files`). `build.rs` pulls this module in with `#[path]`, so it uses
//! `std` alone; the crate holds it only for its tests, since nothing at run time calls it.

use std::path::{Path, PathBuf};

/// The page `docs/spec-format.md` becomes, after every `docs/guide/` page.
const SPEC_FORMAT: &str = "spec-format";

/// The guide's pages as `(name, file)`: the `.md` files of `guide_dir` in the guide's
/// order, then `spec_format` as `spec-format`. A missing `README.md`, a page named
/// `spec-format.md`, or a page or `spec_format` that isn't UTF-8 is an error naming the
/// file.
pub fn pages(guide_dir: &Path, spec_format: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let shown = |p: &Path| p.display().to_string();
    let entries = std::fs::read_dir(guide_dir).map_err(|e| format!("{}: {e}", shown(guide_dir)))?;
    let mut stems = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", shown(guide_dir)))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = file.strip_suffix(".md") {
            stems.push(stem.to_owned());
        }
    }
    let readme_path = guide_dir.join("README.md");
    let readme = read(&readme_path)?;
    let mut out = Vec::new();
    let order = order(&stems, &readme).map_err(|file| {
        let file = shown(&guide_dir.join(file));
        format!("{file}: the page name {SPEC_FORMAT} is docs/spec-format.md's")
    })?;
    for stem in order {
        let file = guide_dir.join(format!("{stem}.md"));
        read(&file)?;
        let name = if stem == "README" { "index" } else { &stem };
        out.push((name.to_owned(), file));
    }
    read(spec_format)?;
    out.push((SPEC_FORMAT.to_owned(), spec_format.to_owned()));
    Ok(out)
}

/// A page's text, or an error naming its file when it can't be read or isn't UTF-8.
fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))
}

/// The file stems of the guide's pages in its order: `README`, then the stems `readme`
/// links as `](<stem>.md)` or `](<stem>.md#...)`, each at its first link, then the rest
/// by name, holding only the stems in `stems`. A stem `spec-format` is the error, as the
/// file name `spec-format.md`.
fn order(stems: &[String], readme: &str) -> Result<Vec<String>, String> {
    if stems.iter().any(|s| s == SPEC_FORMAT) {
        return Err(format!("{SPEC_FORMAT}.md"));
    }
    let mut sorted = stems.to_vec();
    sorted.sort();
    let mut out = vec!["README".to_owned()];
    for stem in linked(readme).into_iter().chain(sorted) {
        if stems.contains(&stem) && !out.contains(&stem) {
            out.push(stem);
        }
    }
    Ok(out)
}

/// Each `<stem>` of a link `](<stem>.md)` or `](<stem>.md#...)` in `readme`, in order,
/// `<stem>` being letters, digits, `_` and `-`.
fn linked(readme: &str) -> Vec<String> {
    readme
        .split("](")
        .skip(1)
        .filter_map(|rest| {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(rest.len());
            let (stem, after) = rest.split_at(end);
            let after = after.strip_prefix(".md")?;
            let closed =
                after.starts_with(')') || after.strip_prefix('#').is_some_and(|a| a.contains(')'));
            (!stem.is_empty() && closed).then(|| stem.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary directory holding `spec-format.md` and `guide/` with `files` in it.
    fn dir(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("spec-format.md"), "# S\n").unwrap();
        std::fs::create_dir(tmp.path().join("guide")).unwrap();
        for (name, text) in files {
            std::fs::write(tmp.path().join("guide").join(name), text).unwrap();
        }
        tmp
    }

    fn all(tmp: &tempfile::TempDir) -> Result<Vec<(String, PathBuf)>, String> {
        pages(
            &tmp.path().join("guide"),
            &tmp.path().join("spec-format.md"),
        )
    }

    fn names(tmp: &tempfile::TempDir) -> Result<Vec<String>, String> {
        all(tmp).map(|p| p.into_iter().map(|(n, _)| n).collect())
    }

    #[test]
    fn order_from_the_index_page() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[
            ("README.md", b"# G\n\n1. [B](b.md#x)\n2. [A](a.md)\n3. [B](b.md)\n[gone](gone.md) [s](../spec-format.md) [i](README.md)\n"),
            ("a.md", b"# A\n"),
            ("b.md", b"# B\n"),
            ("z.md", b"# Z\n"),
            ("notes.txt", b"x"),
        ]);
        assert_eq!(
            names(&tmp).unwrap(),
            ["index", "b", "a", "z", "spec-format"]
        );
        let all = all(&tmp).unwrap();
        assert_eq!(all[0].1, tmp.path().join("guide/README.md"));
        assert_eq!(all[1].1, tmp.path().join("guide/b.md"));
        assert_eq!(all[4].1, tmp.path().join("spec-format.md"));
    }

    #[test]
    fn unlinked_pages_by_name() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[
            ("README.md", b"# G\n1. [M](m.md)\n"),
            ("z.md", b"# Z\n"),
            ("m.md", b"# M\n"),
            ("c.md", b"# C\n"),
        ]);
        assert_eq!(
            names(&tmp).unwrap(),
            ["index", "m", "c", "z", "spec-format"]
        );
    }

    #[test]
    fn spec_format_that_is_not_utf8() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[("README.md", b"# G\n")]);
        std::fs::write(tmp.path().join("spec-format.md"), b"\xff").unwrap();
        let err = names(&tmp).unwrap_err();
        assert!(err.contains("spec-format.md"), "{err}");
    }

    #[test]
    fn a_page_named_spec_format() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[("README.md", b"# G\n"), ("spec-format.md", b"# S\n")]);
        let err = names(&tmp).unwrap_err();
        assert!(err.contains("spec-format.md"), "{err}");
    }

    #[test]
    fn missing_index_page() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[("a.md", b"# A\n")]);
        let err = names(&tmp).unwrap_err();
        assert!(err.contains("README.md"), "{err}");
    }

    #[test]
    fn page_that_is_not_utf8() {
        // req:init/guide-table-from-files@5c6bd405
        let tmp = dir(&[("README.md", b"# G\n"), ("bad.md", b"\xff\xfe")]);
        let err = names(&tmp).unwrap_err();
        assert!(err.contains("bad.md"), "{err}");
    }
}
