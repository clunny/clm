//! The guide's entry pages stay written for a newcomer (`guide-entry-pages`): they name the
//! workflow skills and link the pages that say how it works, and name no `clm` command.

use super::links::{outside_fences, past_code_span};

/// What a page must name and link: its name, the skills it names, the pages it links.
const RULES: [(&str, &[&str], &[&str]); 2] = [
    (
        "quickstart",
        &[
            "/plugin marketplace add clunny/clm",
            "/plugin install clm@clm",
            "/clm:init",
            "/clm:build",
            "/clm:propose",
            "/clm:triage",
            "/clm:health",
        ],
        &["using-clm.md", "how-it-works.md"],
    ),
    (
        "using-clm",
        &["/clm:propose", "/clm:build", "/clm:triage", "/clm:health"],
        &["how-it-works.md"],
    ),
];

/// What the entry page `page` (`quickstart` or `using-clm`) lacks or holds, each as
/// `<page>: lacks <thing>` or `<page>: names <command>`. A page that isn't an entry page
/// has no rule.
fn problems(page: &str, markdown: &str) -> Vec<String> {
    let Some((_, names, links)) = RULES.iter().find(|(p, ..)| *p == page) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for name in *names {
        if !markdown.contains(name) {
            out.push(format!("{page}: lacks {name}"));
        }
    }
    for link in *links {
        let linked = markdown
            .match_indices(&format!("]({link}"))
            .any(|(i, m)| matches!(markdown.as_bytes().get(i + m.len()), Some(b')' | b'#')));
        if !linked {
            out.push(format!("{page}: lacks a link to {link}"));
        }
    }
    for span in code_spans(&outside_fences(markdown)) {
        if span.starts_with("clm ") {
            out.push(format!("{page}: names {span}"));
        }
    }
    out
}

/// The text of each code span in `text`, as written.
fn code_spans(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let n = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            let end = past_code_span(bytes, i);
            if end - i > n {
                spans.push(&text[i + n..end - n]);
            }
            i = end;
        } else {
            i += 1;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_USING: &str =
        "/clm:propose /clm:build /clm:triage /clm:health [How](how-it-works.md#health)";
    const GOOD_QUICK: &str = "/plugin marketplace add clunny/clm /plugin install clm@clm \
        /clm:init /clm:build /clm:propose /clm:triage /clm:health \
        [U](using-clm.md) [H](how-it-works.md)";

    fn page(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/guide")
            .join(format!("{name}.md"));
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn this_repositorys_entry_pages_meet_the_rule() {
        // req:init/guide-entry-pages@55895485
        for name in ["quickstart", "using-clm"] {
            let out = problems(name, &page(name));
            assert!(out.is_empty(), "entry pages:\n{}", out.join("\n"));
        }
    }

    #[test]
    fn a_command_on_an_entry_page_is_reported() {
        // req:init/guide-entry-pages@55895485
        let md = format!("{GOOD_USING}\nrun `clm land <branch>`");
        assert_eq!(
            problems("using-clm", &md),
            ["using-clm: names clm land <branch>"]
        );
    }

    #[test]
    fn a_missing_workflow_is_reported() {
        // req:init/guide-entry-pages@55895485
        let md = GOOD_QUICK.replace("/clm:triage", "");
        assert_eq!(
            problems("quickstart", &md),
            ["quickstart: lacks /clm:triage"]
        );
    }

    #[test]
    fn a_missing_link_is_reported() {
        // req:init/guide-entry-pages@55895485
        let md = GOOD_USING.replace("how-it-works.md", "elsewhere.md");
        assert_eq!(
            problems("using-clm", &md),
            ["using-clm: lacks a link to how-it-works.md"]
        );
    }

    #[test]
    fn a_command_in_a_fenced_block_is_allowed() {
        // req:init/guide-entry-pages@55895485
        let md =
            format!("{GOOD_QUICK}\n```\nclm status\n```\n\nthe skill `/clm:build` and `clm` alone");
        assert!(problems("quickstart", &md).is_empty());
    }
}
