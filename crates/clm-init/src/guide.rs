//! `clm guide`: CLM's guide, the pages of `docs/guide/` and the spec format compiled into
//! the binary (`guide-in-the-binary`, `spec-format-in-the-guide`), so it reads offline and
//! matches the version that runs.

// `const PAGES: [(&str, &str); N]`, each page as `(name, markdown)`: `README.md` first as
// `index`, then the pages in the order it links them, then those it doesn't by name, then
// `docs/spec-format.md` as `spec-format`. `build.rs` generates it from the files
// (`guide-table-from-files`).
include!(concat!(env!("OUT_DIR"), "/guide_pages.rs"));

/// With no page, the index: one `<name>  <title>` line per page. With one, that page's
/// Markdown unchanged. An unknown name is an error holding `no guide page <name>` and the
/// index, for stderr.
pub fn guide(page: Option<&str>) -> Result<String, String> {
    let Some(name) = page else {
        return Ok(index());
    };
    match PAGES.iter().find(|(n, _)| *n == name) {
        Some((_, markdown)) => Ok((*markdown).to_owned()),
        None => Err(format!("no guide page {name}\n{}", index())),
    }
}

fn index() -> String {
    PAGES
        .iter()
        .map(|(name, markdown)| format!("{name}  {}\n", title(markdown)))
        .collect()
}

/// The page's first `# ` heading.
fn title(markdown: &str) -> &str {
    markdown
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("")
        .trim()
}

#[cfg(test)]
mod links;

#[cfg(test)]
mod table;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn guide_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide")
    }

    /// A page's file, from the repository's root.
    fn file_of(name: &str) -> String {
        match name {
            "index" => "docs/guide/README.md".to_owned(),
            "spec-format" => "docs/spec-format.md".to_owned(),
            name => format!("docs/guide/{name}.md"),
        }
    }

    /// The text of a page's file at the repository's root, `docs/spec-format.md` for
    /// `spec-format` and `docs/guide/<file>` for the rest.
    fn file_text(name: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::read_to_string(root.join(file_of(name))).unwrap()
    }

    /// The page name of a file in `docs/guide/`.
    fn name_of(file: &str) -> &str {
        match file.strip_suffix(".md").unwrap_or(file) {
            "README" => "index",
            name => name,
        }
    }

    #[test]
    fn every_page_is_compiled_in() {
        // req:init/guide-command@32520cc6
        // req:init/guide-table-from-files@5c6bd405
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(guide_dir()).unwrap() {
            let file = entry.unwrap().file_name().to_string_lossy().into_owned();
            if file.ends_with(".md") && !PAGES.iter().any(|(n, _)| *n == name_of(&file)) {
                missing.push(file);
            }
        }
        assert!(
            missing.is_empty(),
            "docs/guide pages not in the binary: {missing:?}"
        );
    }

    #[test]
    fn no_page_named_in_source() {
        // req:init/guide-table-from-files@5c6bd405
        // Built from parts so this file doesn't match itself.
        let needle = concat!("docs/", "guide/");
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut named = Vec::new();
        let mut dirs = vec![src];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if std::fs::read_to_string(&path)
                    .unwrap()
                    .lines()
                    .any(|l| l.contains("include_str!") && l.contains(needle))
                {
                    named.push(path);
                }
            }
        }
        assert!(named.is_empty(), "a guide page named by hand in {named:?}");
    }

    #[test]
    fn pages_in_the_index_order() {
        // req:init/guide-command@32520cc6
        // req:init/guide-table-from-files@5c6bd405
        // `README.md`'s numbered list: lines `<n>. [Title](file.md)...`.
        let readme = std::fs::read_to_string(guide_dir().join("README.md")).unwrap();
        let listed: Vec<&str> = readme
            .lines()
            .filter(|l| {
                l.split_once(". ")
                    .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .filter_map(|l| l.split_once("](")?.1.split_once(".md").map(|(f, _)| f))
            .fold(Vec::new(), |mut seen, name| {
                if !seen.contains(&name) {
                    seen.push(name);
                }
                seen
            });
        let names: Vec<&str> = PAGES.iter().map(|(n, _)| *n).collect();
        assert_eq!(names[0], "index");
        assert_eq!(names[1..=listed.len()], listed[..], "{readme}");
        // Pages the list doesn't name come after it, by name, before `spec-format`.
        assert!(
            names[listed.len() + 1..names.len() - 1].is_sorted(),
            "{names:?}"
        );
        assert_eq!(names.last(), Some(&"spec-format"));
    }

    #[test]
    fn pages_as_written() {
        // req:init/guide-command@32520cc6
        for (name, markdown) in PAGES {
            let text = file_text(name);
            assert_eq!(guide(Some(name)).unwrap(), text, "{name}");
            assert_eq!(markdown, text, "{name}");
        }
    }

    #[test]
    fn links_stay_inside_the_guide() {
        // req:init/guide-links-inside@863332b6
        let files: Vec<String> = PAGES.iter().map(|(name, _)| file_of(name)).collect();
        let files: Vec<&str> = files.iter().map(String::as_str).collect();
        let out: Vec<String> = PAGES
            .iter()
            .zip(&files)
            .flat_map(|((_, markdown), file)| links::links_out(file, markdown, &files))
            .collect();
        assert!(
            out.is_empty(),
            "links out of the guide:\n{}",
            out.join("\n")
        );
        // The scanner reads `reference`'s links to the spec format and `using-clm`: with no
        // pages to name, it reports them.
        let reference = guide(Some("reference")).unwrap();
        let read = links::links_out("docs/guide/reference.md", &reference, &[]);
        for link in ["reference: ../spec-format.md", "reference: using-clm.md"] {
            assert!(read.iter().any(|l| l == link), "{link} not read: {read:?}");
        }
    }

    #[test]
    fn index_lines() {
        // req:init/guide-command@32520cc6
        let index = guide(None).unwrap();
        assert!(index.starts_with("index  CLM guide\n"), "{index}");
        assert!(
            index
                .lines()
                .any(|l| l == "shaping-the-graph  Shaping the graph"),
            "{index}"
        );
        assert!(
            index.ends_with("\nspec-format  Spec format (v0, provisional)\n"),
            "{index}"
        );
        assert_eq!(index.lines().count(), PAGES.len());
        let err = guide(Some("no-such-page")).unwrap_err();
        assert_eq!(err, format!("no guide page no-such-page\n{index}"));
    }
}
