// clm:reads ARCHITECTURE.md
//! The human layers: the entries of `ARCHITECTURE.md` and `CAPABILITIES.md`, and the
//! component IDs they reserve.

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Layer, Violation, content_hash, load, load_files};
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

const GIT_IS_TRUTH: &str = "### Constraint: single-source\nGit SHALL be the source of truth.\n\n#### Scenario: no server state\n- GIVEN a repo\n- THEN git holds it\n";

const APPROVE: &str = "### Capability: approve\nA reviewer can approve a change.\n\n#### Why\nOnly a human decides.\n\n#### Scenario: approve\n- GIVEN a change\n- THEN it lands\n";

fn architecture(entries: &str) -> String {
    format!("# Architecture\n\nPreamble.\n\n## Constraints\n\n{entries}")
}

fn capabilities(entries: &str) -> String {
    format!("# Capabilities\n\nPreamble.\n\n## Capabilities\n\n{entries}")
}

/// Root `clm`, plus `extra` files.
fn files(extra: &[(&str, &str)]) -> Files {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    for (path, text) in extra {
        files.insert(PathBuf::from(path), text.as_bytes().to_vec());
    }
    files
}

fn clean(extra: &[(&str, &str)]) -> Graph {
    let (graph, violations) = load_files(&files(extra));
    assert!(violations.is_empty(), "unexpected: {violations:?}");
    graph
}

/// The one violation, which is `rule` on `path` and names each of `names`.
fn one_violation(extra: &[(&str, &str)], rule: &str, path: &str, names: &[&str]) -> Graph {
    let (graph, violations) = load_files(&files(extra));
    assert_eq!(violations.len(), 1, "{violations:?}");
    let v: &Violation = &violations[0];
    assert_eq!(v.rule, rule, "{v:?}");
    assert_eq!(v.path, Path::new(path), "{v:?}");
    for name in names {
        assert!(v.message.contains(name), "{name}: {v:?}");
    }
    graph
}

fn ids(graph: &Graph, layer: Layer) -> Vec<&str> {
    graph
        .layers
        .get(&layer)
        .map(|e| e.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default()
}

#[test]
fn entries_load_with_ids_and_versions() {
    // req:model/human-layers@c4f90bbc
    let graph = clean(&[
        ("ARCHITECTURE.md", &architecture(GIT_IS_TRUTH)),
        ("CAPABILITIES.md", &capabilities(APPROVE)),
    ]);
    assert_eq!(
        ids(&graph, Layer::Architecture),
        ["architecture/single-source"]
    );
    assert_eq!(ids(&graph, Layer::Capability), ["capability/approve"]);
    let arch = graph.requirement("architecture/single-source").unwrap();
    assert_eq!(arch.hash, content_hash(GIT_IS_TRUTH));
    assert_eq!(arch.version(), &content_hash(GIT_IS_TRUTH)[..8]);
    assert_eq!(arch.scenarios, 1);
    let cap = graph.requirement("capability/approve").unwrap();
    assert_eq!(cap.hash, content_hash(APPROVE));
    assert!(cap.needs.is_empty() && cap.serves.is_empty());
    assert_eq!(graph.entry("capability/approve"), Some(cap));
    assert!(graph.entry("clm/none").is_none());
    assert!(graph.requirement("capability/nope").is_none());
}

#[test]
fn entries_end_at_the_section_and_skip_fences() {
    // req:model/human-layers@c4f90bbc
    let fenced = format!("{APPROVE}\n```markdown\n### Capability: inside\n```\n");
    let text = format!("{}\n## Later\nprose\n", capabilities(&fenced));
    let graph = clean(&[("CAPABILITIES.md", &text)]);
    assert_eq!(ids(&graph, Layer::Capability), ["capability/approve"]);
    let cap = graph.requirement("capability/approve").unwrap();
    assert!(!cap.text.contains("## Later"), "{}", cap.text);
}

#[test]
fn free_form_architecture_has_no_entries() {
    // req:model/human-layers@c4f90bbc
    let old = "# Architecture\n\n## Layer 1 invariants\n\n- **`single-source`**: git holds it.\n- **`no-dead-code`**: remove it.\n\n## Glossary\n\n### Requirement\nA rule.\n";
    let graph = clean(&[("ARCHITECTURE.md", old)]);
    assert!(ids(&graph, Layer::Architecture).is_empty());
}

#[test]
fn absent_files_have_no_entries() {
    // req:model/human-layers@c4f90bbc
    let graph = clean(&[]);
    assert!(graph.layers.values().all(Vec::is_empty));
    assert_eq!(graph.requirements().count(), 0);
}

#[test]
fn entry_rules() {
    // req:model/human-layers@c4f90bbc
    const RULE: &str = "model/human-layers";
    let no_scenario = "### Constraint: x\nIt SHALL hold.\n";
    let no_shall = "### Constraint: x\nIt holds.\n\n#### Scenario: s\n- ok\n";
    let no_why = "### Capability: x\nA person can.\n\n#### Scenario: s\n- ok\n";
    let two_whys = "### Capability: x\nA person can.\n\n#### Why\na\n\n#### Why\nb\n\n#### Scenario: s\n- ok\n";
    let needs = "### Constraint: x\nIt SHALL.\n\n#### Needs\n- a/y\n\n#### Scenario: s\n- ok\n";
    let serves = "### Capability: x\nA person can.\n\n#### Why\na\n\n#### Serves\n- capability/y\n\n#### Scenario: s\n- ok\n";
    let duplicate = format!("{APPROVE}\n{}", APPROVE);
    let outside = format!(
        "# Capabilities\n\n## Why we exist\n\n{}\n## Capabilities\n\n",
        APPROVE.replace("approve", "x")
    );
    let no_section = format!("# Capabilities\n\n{APPROVE}");
    let cases: [(&str, String, &str); 9] = [
        ("CAPABILITIES.md", no_section, "approve"),
        ("ARCHITECTURE.md", architecture(no_scenario), "x"),
        ("ARCHITECTURE.md", architecture(no_shall), "x"),
        ("CAPABILITIES.md", capabilities(no_why), "x"),
        ("CAPABILITIES.md", capabilities(two_whys), "x"),
        ("ARCHITECTURE.md", architecture(needs), "x"),
        ("CAPABILITIES.md", capabilities(serves), "x"),
        ("CAPABILITIES.md", capabilities(&duplicate), "approve"),
        ("CAPABILITIES.md", outside, "x"),
    ];
    for (i, (file, text, slug)) in cases.into_iter().enumerate() {
        let graph = one_violation(&[(file, &text)], RULE, file, &[&format!("`{slug}`")]);
        if i == 0 {
            // An entry heading with no section gives no entries.
            assert!(ids(&graph, Layer::Capability).is_empty());
        }
    }
    for slug in ["Approve", "approve intent", "a/b", "a`b", ""] {
        let text = capabilities(&APPROVE.replace("approve\n", &format!("{slug}\n")));
        let (graph, violations) = load_files(&files(&[("CAPABILITIES.md", &text)]));
        assert_eq!(violations.len(), 1, "{slug}: {violations:?}");
        assert_eq!(violations[0].rule, RULE);
        assert!(
            violations[0].message.contains(&format!("`{slug}`")),
            "{violations:?}"
        );
        assert!(ids(&graph, Layer::Capability).is_empty(), "{slug}");
    }
    clean(&[(
        "CAPABILITIES.md",
        &capabilities(&APPROVE.replace("approve\n", "approve-2-ways\n")),
    )]);
    // MUST counts as SHALL does.
    clean(&[(
        "ARCHITECTURE.md",
        &architecture(&no_shall.replace("It holds", "It MUST hold")),
    )]);
}

#[test]
fn non_utf8_layer_reported() {
    // req:model/human-layers@c4f90bbc
    let mut files = files(&[]);
    files.insert(PathBuf::from("CAPABILITIES.md"), vec![0xff, 0xfe]);
    let (graph, violations) = load_files(&files);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/human-layers");
    assert_eq!(violations[0].path, Path::new("CAPABILITIES.md"));
    assert!(ids(&graph, Layer::Capability).is_empty());
}

#[test]
fn crlf_layer_same_version() {
    // req:model/human-layers@c4f90bbc
    let lf = clean(&[("CAPABILITIES.md", &capabilities(APPROVE))]);
    let crlf = capabilities(APPROVE).replace('\n', "\r\n");
    let crlf = clean(&[("CAPABILITIES.md", &crlf)]);
    assert_eq!(
        lf.requirement("capability/approve").unwrap().hash,
        crlf.requirement("capability/approve").unwrap().hash
    );
}

/// `specs/<id>/SPEC.md` holding one requirement per slug.
fn rules(id: &str, slugs: &[&str]) -> (String, String) {
    let reqs: String = slugs
        .iter()
        .map(|s| format!("### Requirement: {s}\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n"))
        .collect();
    (
        format!("specs/{id}/SPEC.md"),
        format!("{ROOT}\n## Requirements\n\n{reqs}"),
    )
}

fn with_rules<'a>(
    nodes: &'a [(String, String)],
    extra: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut all: Vec<(&str, &str)> = nodes
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    all.extend_from_slice(extra);
    all
}

const ENFORCED: &str = "### Constraint: x\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/y\n- b/z\n\n#### Untested\nPart of it is a judgement.\n";

#[test]
fn enforcement_blocks_load() {
    // req:model/human-layers@c4f90bbc
    let nodes = [rules("a", &["y"]), rules("b", &["z"])];
    let untested_cap = APPROVE.replace("approve", "c") + "\n#### Untested\nNo program sees it.\n";
    let arch = architecture(&format!("{ENFORCED}\n{GIT_IS_TRUTH}"));
    let caps = capabilities(&format!("{untested_cap}\n{APPROVE}"));
    let graph = clean(&with_rules(
        &nodes,
        &[("ARCHITECTURE.md", &arch), ("CAPABILITIES.md", &caps)],
    ));
    let x = graph.requirement("architecture/x").unwrap();
    assert_eq!(x.enforced_by, ["a/y", "b/z"]);
    assert!(x.untested);
    assert_eq!(
        x.untested_text.as_deref(),
        Some("Part of it is a judgement.")
    );
    let c = graph.requirement("capability/c").unwrap();
    assert_eq!(c.untested_text.as_deref(), Some("No program sees it."));
    assert!(graph.requirement("capability/c").unwrap().untested);
    let plain = graph.requirement("architecture/single-source").unwrap();
    assert!(plain.enforced_by.is_empty() && !plain.untested && plain.untested_text.is_none());
    assert!(!graph.requirement("capability/approve").unwrap().untested);
    assert!(!graph.requirement("a/y").unwrap().untested);
    // Both blocks are part of the entry's block, so of its version.
    assert_eq!(x.hash, content_hash(ENFORCED));
    assert!(x.text.contains("#### Enforced by") && x.text.contains("#### Untested"));
}

#[test]
fn bad_enforcement() {
    // req:model/human-layers@c4f90bbc
    let nodes = [rules("a", &["y"])];
    let gone = ENFORCED.replace("- b/z\n", "- a/gone\n");
    let cap = APPROVE.to_string() + "\n#### Enforced by\n- a/y\n";
    let (graph, violations) = load_files(&files(&with_rules(
        &nodes,
        &[
            ("ARCHITECTURE.md", &architecture(&gone)),
            ("CAPABILITIES.md", &capabilities(&cap)),
        ],
    )));
    let found: Vec<(&Path, &str)> = violations
        .iter()
        .map(|v| (v.path.as_path(), v.rule))
        .collect();
    assert_eq!(
        found,
        [
            (Path::new("CAPABILITIES.md"), "model/human-layers"),
            (Path::new("ARCHITECTURE.md"), "model/human-layers"),
        ],
        "{violations:?}"
    );
    assert!(
        violations[0].message.contains("`approve`"),
        "{violations:?}"
    );
    assert!(violations[1].message.contains("`x`"), "{violations:?}");
    assert!(violations[1].message.contains("`a/gone`"), "{violations:?}");
    assert!(
        graph
            .requirement("capability/approve")
            .unwrap()
            .enforced_by
            .is_empty()
    );
}

#[test]
fn enforcement_block_rules() {
    // req:model/human-layers@c4f90bbc
    const RULE: &str = "model/human-layers";
    let nodes = [rules("a", &["y"]), rules("b", &["z"])];
    let base = "### Constraint: x\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n";
    let arch_cases = [
        format!("{base}\n#### Enforced by\n- a/y\n\n#### Enforced by\n- b/z\n"),
        format!("{base}\n#### Untested\nOne.\n\n#### Untested\nTwo.\n"),
        format!("{base}\n#### Enforced by\n\n#### Untested\nWhy.\n"),
        format!("{base}\n#### Untested\n\n"),
        format!("{base}\n#### Enforced by\na/y\n"),
        format!("{base}\n#### Enforced by\n- architecture/single-source\n"),
        format!("{base}\n#### Enforced by\n- capability/approve\n"),
    ];
    for text in arch_cases {
        let arch = architecture(&format!("{text}\n{GIT_IS_TRUTH}"));
        let caps = capabilities(APPROVE);
        let all = with_rules(
            &nodes,
            &[("ARCHITECTURE.md", &arch), ("CAPABILITIES.md", &caps)],
        );
        one_violation(&all, RULE, "ARCHITECTURE.md", &["`x`"]);
    }
    // A capability may hold `#### Untested`, but only once.
    let twice = APPROVE.to_string() + "\n#### Untested\nOne.\n\n#### Untested\nTwo.\n";
    one_violation(
        &[("CAPABILITIES.md", &capabilities(&twice))],
        RULE,
        "CAPABILITIES.md",
        &["`approve`"],
    );
    // A block heading inside a code fence is text.
    let fenced = format!("{base}\n```markdown\n#### Enforced by\n- a/gone\n```\n");
    let graph = clean(&[("ARCHITECTURE.md", &architecture(&fenced))]);
    assert!(
        graph
            .requirement("architecture/x")
            .unwrap()
            .enforced_by
            .is_empty()
    );
}

#[test]
fn reserved_node_ids_rejected() {
    // req:model/reserved-ids@5ee10777
    let path = "specs/architecture/SPEC.md";
    let graph = one_violation(
        &[(path, ROOT)],
        "model/reserved-ids",
        path,
        &["architecture"],
    );
    assert!(!graph.nodes.contains_key("architecture"));
    // A child of the reserved node has no parent.
    let (graph, violations) = load_files(&files(&[
        ("specs/architecture/SPEC.md", ROOT),
        ("specs/architecture/x/SPEC.md", ROOT),
    ]));
    let rules: Vec<(&str, &Path)> = violations
        .iter()
        .map(|v| (v.rule, v.path.as_path()))
        .collect();
    assert_eq!(
        rules,
        [
            (
                "model/reserved-ids",
                Path::new("specs/architecture/SPEC.md")
            ),
            (
                "model/parse-node-specs",
                Path::new("specs/architecture/x/SPEC.md")
            ),
        ],
        "{violations:?}"
    );
    assert!(graph.nodes.contains_key("architecture.x"));
    // Only the top level is reserved.
    clean(&[
        ("specs/a/SPEC.md", ROOT),
        ("specs/a/architecture/SPEC.md", ROOT),
    ]);
}

const CAPABILITY_NODE: &str = "---\ntitle: Capabilities\nstatus: active\nowners: []\n---\n\n## Contract\nThe capabilities' tests.\n\n## Design\nA harness.\n";

#[test]
fn capability_node_loads() {
    // req:model/reserved-ids@5ee10777
    let a = APPROVE.replace("approve", "a");
    let b = APPROVE.replace("approve", "b");
    let graph = clean(&[
        ("specs/capability/SPEC.md", CAPABILITY_NODE),
        ("specs/capability/x/SPEC.md", ROOT),
        ("CAPABILITIES.md", &capabilities(&format!("{a}\n{b}"))),
    ]);
    let node = graph.capability_node().expect("the capability node");
    assert_eq!(node.id, "capability");
    assert!(node.requirements.is_empty());
    assert!(graph.nodes.contains_key("capability.x"));
    assert_eq!(
        ids(&graph, Layer::Capability),
        ["capability/a", "capability/b"]
    );
    // The capabilities are listed once.
    let all: Vec<&str> = graph.requirements().map(|r| r.id.as_str()).collect();
    assert_eq!(all, ["capability/a", "capability/b"]);
    // Without the node, the graph names none; a nested `capability` isn't it.
    assert!(clean(&[]).capability_node().is_none());
    let nested = clean(&[
        ("specs/a/SPEC.md", ROOT),
        ("specs/a/capability/SPEC.md", ROOT),
    ]);
    assert!(nested.capability_node().is_none());
}

#[test]
fn capability_tests_node_names_the_node() {
    // req:model/capability-tests-component@f2664a6c
    let with = clean(&[("specs/capability/SPEC.md", CAPABILITY_NODE)]);
    assert_eq!(with.capability_tests_node(), "capability");
    assert_eq!(clean(&[]).capability_tests_node(), clm_model::ROOT_ID);
    // A nested `capability` isn't the capability node, so the root builds the tests.
    let nested = clean(&[
        ("specs/a/SPEC.md", ROOT),
        ("specs/a/capability/SPEC.md", ROOT),
    ]);
    assert_eq!(nested.capability_tests_node(), clm_model::ROOT_ID);
}

#[test]
fn requirement_in_capability_node() {
    // req:model/reserved-ids@5ee10777
    let text = format!(
        "{CAPABILITY_NODE}\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
    );
    let path = "specs/capability/SPEC.md";
    let graph = one_violation(&[(path, &text)], "model/reserved-ids", path, &["`x`"]);
    let node = graph.capability_node().expect("still loads");
    assert!(node.requirements.is_empty());
    assert!(graph.requirement("capability/x").is_none());
}

#[test]
fn need_on_entry_rejected() {
    // req:model/requirement-needs@0d58e0b7
    let a = format!(
        "{ROOT}\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Needs\n- architecture/single-source\n\n#### Scenario: s\n- ok\n"
    );
    one_violation(
        &[
            ("ARCHITECTURE.md", &architecture(GIT_IS_TRUTH)),
            ("specs/a/SPEC.md", &a),
        ],
        "model/requirement-needs",
        "specs/a/SPEC.md",
        &["`a/x`", "`architecture/single-source`", "human-layer entry"],
    );
}

#[test]
fn blocks_may_name_an_entry() {
    // req:model/parse-backlog@fa4b09c8
    let item = "---\ntitle: T\nkind: question\nblocks: [capability/approve, architecture/single-source]\n---\n";
    clean(&[
        ("ARCHITECTURE.md", &architecture(GIT_IS_TRUTH)),
        ("CAPABILITIES.md", &capabilities(APPROVE)),
        ("backlog/x.md", item),
    ]);
    one_violation(
        &[
            ("CAPABILITIES.md", &capabilities(APPROVE)),
            (
                "backlog/x.md",
                "---\ntitle: T\nkind: question\nblocks: [capability/nope]\n---\n",
            ),
        ],
        "model/parse-backlog",
        "backlog/x.md",
        &["capability/nope"],
    );
}

#[test]
fn requirements_lists_entries_last() {
    // req:model/human-layers@c4f90bbc
    let a = format!(
        "{ROOT}\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
    );
    let zeta = APPROVE.replace("approve", "zeta");
    let graph = clean(&[
        (
            "CAPABILITIES.md",
            &capabilities(&format!("{zeta}\n{APPROVE}")),
        ),
        ("ARCHITECTURE.md", &architecture(GIT_IS_TRUTH)),
        ("specs/a/SPEC.md", &a),
    ]);
    let all: Vec<&str> = graph.requirements().map(|r| r.id.as_str()).collect();
    assert_eq!(
        all,
        [
            "a/x",
            "architecture/single-source",
            "capability/zeta",
            "capability/approve"
        ]
    );
}

#[test]
fn layer_names() {
    // req:model/human-layers@c4f90bbc
    assert_eq!(Layer::ALL, [Layer::Architecture, Layer::Capability]);
    assert_eq!(Layer::Architecture.id(), "architecture");
    assert_eq!(Layer::Capability.file(), "CAPABILITIES.md");
    assert_eq!(Layer::of("capability/x"), Some(Layer::Capability));
    assert_eq!(Layer::of("architecture/x"), Some(Layer::Architecture));
    assert_eq!(Layer::of("capability"), None);
    assert_eq!(Layer::of("land/x"), None);
}

#[test]
fn load_files_same_as_load() {
    // req:model/load-from-files@5011a677
    let extra = [
        ("ARCHITECTURE.md", architecture(GIT_IS_TRUTH)),
        ("CAPABILITIES.md", capabilities(APPROVE)),
        ("specs/a/SPEC.md", "no frontmatter".to_string()),
        ("README.md", "not read".to_string()),
    ];
    let dir = TempDir::new().unwrap();
    let mut map = files(&[]);
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join("specs/SPEC.md"), ROOT).unwrap();
    for (path, text) in &extra {
        let file = dir.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
        if *path != "README.md" {
            map.insert(PathBuf::from(path), text.as_bytes().to_vec());
        }
    }
    let from_disk = load(dir.path());
    assert_eq!(from_disk, load_files(&map));
    assert_eq!(from_disk.0.requirements().count(), 2);
}

#[test]
fn friction_may_flag_an_entry() {
    // req:model/parse-backlog@fa4b09c8
    let version = &content_hash(APPROVE)[..8];
    let flag = |v: &str| {
        format!("---\ntitle: T\nkind: friction\nrequirement: capability/approve@{v}\n---\n")
    };
    clean(&[
        ("CAPABILITIES.md", &capabilities(APPROVE)),
        ("backlog/f.md", &flag(version)),
    ]);
    one_violation(
        &[
            ("CAPABILITIES.md", &capabilities(APPROVE)),
            ("backlog/f.md", &flag("00000000")),
        ],
        "model/parse-backlog",
        "backlog/f.md",
        &["capability/approve"],
    );
}

#[test]
fn node_parser_rejects_a_reserved_id() {
    // req:model/graph-check@e0dbe7bc
    let path = Path::new("specs/architecture/SPEC.md");
    let (node, violations) = clm_model::parse_node("architecture", path, ROOT, &Default::default());
    assert!(node.is_none());
    let rules: Vec<&str> = violations.iter().map(|v| v.rule).collect();
    assert_eq!(rules, ["model/reserved-ids"]);
}

const ADDS_NEW: &str = "---\ncomponent: a\n---\n## ADDED Requirements\n\n### Requirement: new\nN.\n\n#### Scenario: s\n- y\n";

/// An architecture entry enforced by `a/new`, node `a` holding only `y`, and a change at
/// `changes/<place>` with `status` whose delta for `a` is `delta`.
fn enforced_by_new(place: &str, status: &str, delta: &str) -> Vec<Violation> {
    let arch = architecture(&ENFORCED.replace("- a/y\n- b/z\n", "- a/new\n"));
    let node = rules("a", &["y"]);
    let change = format!("---\nstatus: {status}\n---\n");
    let (change_path, delta_path) = (
        format!("changes/{place}/change.md"),
        format!("changes/{place}/deltas/a.md"),
    );
    let (graph, violations) = load_files(&files(&[
        ("ARCHITECTURE.md", &arch),
        (&node.0, &node.1),
        (&change_path, &change),
        (&delta_path, delta),
    ]));
    assert_eq!(
        graph.requirement("architecture/x").unwrap().enforced_by,
        ["a/new"]
    );
    violations
}

#[test]
fn enforcement_named_before_apply() {
    // req:model/human-layers@c4f90bbc
    for status in ["proposed", "abandoned"] {
        let violations = enforced_by_new("c", status, ADDS_NEW);
        assert!(violations.is_empty(), "{status}: {violations:?}");
    }
    // Only a change outside the archive counts.
    let archived = enforced_by_new("archive/c", "done", ADDS_NEW);
    assert_eq!(archived.len(), 1, "{archived:?}");
    assert!(archived[0].message.contains("a/new"), "{archived:?}");
    // A delta that only modifies, or one that failed to load, adds nothing.
    let modifies = ADDS_NEW
        .replace("ADDED", "MODIFIED")
        .replace(": new", ": new <!-- base: 0123456789abcdef -->");
    // A delta for another node adds nothing to `a`.
    let other = ADDS_NEW.replace("component: a", "component: b");
    let violations = enforced_by_new("c", "proposed", &other);
    assert!(
        violations.iter().any(|v| v.message.contains("a/new")),
        "{violations:?}"
    );
    let rejected = format!("{ADDS_NEW}\n## MODIFIED Design\n\n### Requirement: z\n");
    for delta in [modifies.as_str(), rejected.as_str()] {
        let violations = enforced_by_new("c", "proposed", delta);
        let ours = violations.iter().filter(|v| v.message.contains("a/new"));
        assert_eq!(ours.count(), 1, "{violations:?}");
    }
}
