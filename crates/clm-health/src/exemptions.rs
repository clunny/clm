//! The `exemption` signal: what each component's exemptions still cover
//! (`exemptions-signal`).

use std::path::Path;

use clm_model::{ExemptRule, Graph};

use crate::{Failure, Section, finding};

/// The word CLM writes for `rule` in a component's frontmatter.
fn name(rule: ExemptRule) -> &'static str {
    match rule {
        ExemptRule::UndeclaredImports => "undeclared-imports",
        ExemptRule::TestsCiteTheirOwner => "tests-cite-their-owner",
        ExemptRule::SkippedTests => "skipped-tests",
    }
}

/// Each exemption a component holds, with how many tracked files the component owns that it
/// names. `Git` when git can't list the files.
pub(crate) fn section(root: &Path, graph: &Graph) -> Result<Section, Failure> {
    let files = clm_git::tracked_files(root).map_err(|e| Failure::Git(e.to_string()))?;
    let mut lines = Vec::new();
    for (id, node) in &graph.nodes {
        let owned: Vec<String> = files
            .iter()
            .filter(|f| clm_check::owner(graph, f) == Some(id.as_str()))
            .map(|f| f.to_string_lossy().into_owned())
            .collect();
        for e in &node.frontmatter.exemptions {
            let count = owned
                .iter()
                .filter(|f| e.files.iter().any(|g| clm_check::path_matches(g, f)))
                .count();
            let mut message = format!("{id}: {} until {}: {count} files", name(e.rule), e.until);
            let slug = e.until.strip_prefix("backlog/").unwrap_or(&e.until);
            if !root.join(format!("backlog/{slug}.md")).is_file() {
                message.push_str("; item gone");
            }
            if count == 0 {
                message.push_str("; matches nothing");
            }
            lines.push(finding(node.path.display(), "exemption", message));
        }
    }
    Ok(Section::of("exemption", "exemptions", lines))
}
