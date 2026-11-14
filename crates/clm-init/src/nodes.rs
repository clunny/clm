//! `clm survey --components`: checks a proposed component map against the surveyed code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_model::Violation;
use serde::Deserialize;

use crate::survey;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeMap {
    #[serde(default, deserialize_with = "list")]
    components: Option<Vec<Node>>,
    /// The key `20261102-rename-node-to-component` kept beside `components`: refused
    /// (`clm/node-names-retired`).
    #[serde(default, deserialize_with = "present")]
    nodes: Option<serde_yaml::Value>,
    /// The code left for later, as the root's `outside` (`incremental-adoption`).
    /// Read as YAML, since a plain `String` field would take `1` as `"1"`; a null value is
    /// present (`Some(Null)`), so it is refused rather than read as absent.
    #[serde(default, deserialize_with = "present")]
    outside: Option<serde_yaml::Value>,
}

/// A key's list; a null value is present and refused, not read as an absent key.
fn list<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Vec<Node>>, D::Error> {
    Vec::<Node>::deserialize(d).map(Some)
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<serde_yaml::Value>, D::Error> {
    serde_yaml::Value::deserialize(d).map(Some)
}

/// The map's `outside` globs: none when absent; an error unless a list of non-empty strings.
fn outside_globs(value: Option<&serde_yaml::Value>) -> Option<Vec<String>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    value
        .as_sequence()?
        .iter()
        .map(|glob| glob.as_str().filter(|g| !g.is_empty()).map(str::to_owned))
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    id: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    /// Globs the component owns as its tests, whatever `paths` match
    /// (`model/test-ownership-keys`).
    #[serde(default)]
    tests: Vec<String>,
    /// Rules its files are temporarily exempt from (`model/exemptions`), read as YAML and
    /// judged by `exemption_globs`; a null value is present, so it is refused.
    #[serde(default, deserialize_with = "present")]
    exemptions: Option<serde_yaml::Value>,
}

/// The `files` globs of `node`'s exemptions from `undeclared-imports`; an error holding
/// the model's own messages for entries the model refuses (`model/exemptions`). The
/// entries are judged by the model's frontmatter check, as the `exemptions` of the
/// component the map proposes, so this holds no rule of its own to drift from it.
fn exemption_globs(file: &Path, node: &Node) -> Result<Vec<String>, String> {
    let Some(value) = &node.exemptions else {
        return Ok(Vec::new());
    };
    // The keys every frontmatter needs, and the map's `exemptions` as written.
    let fm: serde_yaml::Mapping = [
        ("title", node.id.as_str().into()),
        ("status", "unbuilt".into()),
        ("owners", serde_yaml::Value::Sequence(Vec::new())),
        ("exemptions", value.clone()),
    ]
    .into_iter()
    .map(|(key, value)| (key.into(), value))
    .collect();
    let fm = serde_yaml::to_string(&fm).map_err(|e| e.to_string())?;
    match clm_model::check_node_frontmatter(&node.id, file, &fm, &BTreeSet::new()) {
        Ok((fm, _)) => Ok(fm
            .exemptions
            .into_iter()
            .filter(|e| e.rule == clm_model::ExemptRule::UndeclaredImports)
            .flat_map(|e| e.files)
            .collect()),
        Err(violations) => {
            // A YAML type error ends with its place in the text built above, which is no
            // line of the map's.
            let messages: Vec<&str> = violations
                .iter()
                .map(|v| v.message.split(" at line ").next().unwrap_or_default())
                .collect();
            Err(format!("`exemptions`: {}", messages.join("; ")))
        }
    }
}

/// What's wrong with the node map `yaml` (read from `file`) for the repository at `root`,
/// sorted; an error when the map itself can't be used.
pub fn check_nodes(root: &Path, file: &Path, yaml: &str) -> Result<Vec<Violation>, String> {
    let map: NodeMap =
        serde_yaml::from_str(yaml).map_err(|e| format!("{}: {e}", file.display()))?;
    if map.nodes.is_some() {
        return Ok(vec![Violation {
            path: file.to_path_buf(),
            rule: "survey/old-key",
            message: "sets `nodes`, which is retired; list the components under `components`"
                .to_owned(),
        }]);
    }
    let Some(list) = &map.components else {
        return Err(format!("{}: missing field `components`", file.display()));
    };
    let mut ids = BTreeSet::new();
    for node in list {
        if !ids.insert(node.id.as_str()) {
            return Ok(vec![Violation {
                path: file.to_path_buf(),
                rule: "survey/duplicate-component",
                message: format!("component `{}` appears twice", node.id),
            }]);
        }
    }
    for node in list {
        if let Some(dep) = node.depends_on.iter().find(|d| !ids.contains(d.as_str())) {
            return Err(format!(
                "{}: component `{}` depends on unknown component `{dep}`",
                file.display(),
                node.id
            ));
        }
    }

    let Some(outside) = outside_globs(map.outside.as_ref()) else {
        return Err(format!(
            "{}: `outside` must be a list of non-empty globs",
            file.display()
        ));
    };

    // Each code file's nodes: coverage is over the files `check/unowned-files` reads, so a
    // map this accepts passes that rule once written as the spec graph.
    let code = survey::code_files(root)?;
    let mut owners: BTreeMap<&str, Vec<&str>> =
        code.iter().map(|f| (f.as_str(), Vec::new())).collect();
    // A file a component's `tests` match is that component's whatever `paths` match it; two
    // components' `tests` matching it overlap (`components-own-their-tests`).
    let mut test_owners: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for node in list.iter().filter(|n| !n.tests.is_empty()) {
        for path in matching(root, &node.tests)? {
            test_owners.entry(path).or_default().push(&node.id);
        }
    }
    for node in list {
        for path in matching(root, &node.paths)? {
            if test_owners.contains_key(&path) {
                continue;
            }
            if let Some(list) = owners.get_mut(path.as_str()) {
                list.push(&node.id);
            }
        }
    }
    for (path, ids) in &test_owners {
        if let Some(list) = owners.get_mut(path.as_str()) {
            list.extend(ids);
        }
    }
    // The code files no node covers.
    let unowned: BTreeSet<&str> = owners
        .iter()
        .filter(|(_, nodes)| nodes.is_empty())
        .map(|(path, _)| *path)
        .collect();

    let mut lines = BTreeSet::new();
    let mut exempt = BTreeSet::new();
    for glob in &outside {
        let matched = matching(root, std::slice::from_ref(glob))?;
        let mut exempts = false;
        for path in matched {
            if let Some(path) = unowned.get(path.as_str()) {
                exempt.insert(*path);
                exempts = true;
            }
        }
        if !exempts {
            lines.insert((
                "survey/stale-outside",
                format!("outside glob {glob} matches no file outside every component"),
            ));
        }
    }
    for (path, nodes) in &owners {
        match nodes.as_slice() {
            [] if exempt.contains(path) => false,
            [] => lines.insert((
                "survey/uncovered",
                format!("{path} is in no component's paths"),
            )),
            [_] => false,
            many => {
                let names: Vec<String> = many.iter().map(|n| format!("`{n}`")).collect();
                lines.insert((
                    "survey/overlap",
                    format!("{path} is in the paths of {}", names.join(" and ")),
                ))
            }
        };
    }

    let surveyed = survey::map(root)?;
    let deps: BTreeMap<&str, &[String]> = list
        .iter()
        .map(|n| (n.id.as_str(), n.depends_on.as_slice()))
        .collect();
    // Each component's files an exemption names for `undeclared-imports`.
    let mut exempt_files: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for node in list {
        let globs = exemption_globs(file, node)
            .map_err(|e| format!("{}: {}: {e}", file.display(), node.id))?;
        exempt_files.insert(&node.id, matching(root, &globs)?);
    }
    for (from, to) in surveyed.edges.keys() {
        let (Some([a]), Some([b])) = (
            owners.get(from.as_str()).map(Vec::as_slice),
            owners.get(to.as_str()).map(Vec::as_slice),
        ) else {
            continue;
        };
        let allowed = deps[a].iter().any(|d| d == b) || exempt_files[a].contains(from);
        if a != b && !allowed {
            lines.insert((
                "survey/undeclared-edge",
                format!(
                    "{from} (component `{a}`) imports {to} (component `{b}`), and `{a}` doesn't depend on `{b}`"
                ),
            ));
        }
    }
    for cycle in cycles(&deps) {
        lines.insert((
            "survey/cycle",
            format!("depends_on cycle: {} -> {}", cycle.join(" -> "), cycle[0]),
        ));
    }
    Ok(lines
        .into_iter()
        .map(|(rule, message)| Violation {
            path: file.to_path_buf(),
            rule,
            message,
        })
        .collect())
}

/// The tracked files under `root` that any of the globs match, by git's glob rules: the
/// files `clm size` counts as a node's (`tracked-files-in-git-layer`).
fn matching(root: &Path, globs: &[String]) -> Result<BTreeSet<String>, String> {
    let paths = clm_git::tracked_matching(root, globs).map_err(|e| e.to_string())?;
    Ok(survey::lossy(paths).into_iter().collect())
}

/// Each elementary `depends_on` cycle once, starting at its smallest ID.
fn cycles<'a>(deps: &BTreeMap<&'a str, &'a [String]>) -> BTreeSet<Vec<&'a str>> {
    /// Extends `path` (from `start`) through `reach`, recording each way back to `start`.
    fn walk<'a>(
        start: &'a str,
        deps: &BTreeMap<&'a str, &'a [String]>,
        reach: &BTreeSet<&'a str>,
        path: &mut Vec<&'a str>,
        found: &mut BTreeSet<Vec<&'a str>>,
    ) {
        let at = path[path.len() - 1];
        for dep in deps[at].iter().map(String::as_str) {
            if dep == start {
                found.insert(path.clone());
            } else if reach.contains(dep) && !path.contains(&dep) {
                path.push(dep);
                walk(start, deps, reach, path, found);
                path.pop();
            }
        }
    }
    let mut found = BTreeSet::new();
    for &start in deps.keys() {
        // The nodes after `start` that lead back to it: a cycle whose smallest ID is `start`
        // passes through no others, which keeps the walk inside its strongly connected part.
        let mut reach = BTreeSet::from([start]);
        let mut grew = true;
        while grew {
            grew = false;
            for (&id, ds) in deps {
                if id > start && !reach.contains(id) && ds.iter().any(|d| reach.contains(&**d)) {
                    reach.insert(id);
                    grew = true;
                }
            }
        }
        walk(start, deps, &reach, &mut vec![start], &mut found);
    }
    found
}
