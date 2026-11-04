//! `clm rename` rebases the other live changes' deltas whose bases it moves
//! (`rename/other-deltas`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

const SPROCKET: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nPrint the sprocket.\n\n## Design\nRead the file.\n\n## Requirements\n\n### Requirement: x\nX SHALL print the sprocket.\n\n#### Scenario: holds\n- ok\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

/// A repository holding component `a` as `spec`.
fn repo(spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", spec);
    dir
}

/// `clm rename` in a git checkout with everything staged, as the command runs.
fn rename(d: &Path, id: &str) -> Vec<String> {
    clm_git::git(d, &["init", "-q"]).unwrap();
    clm_git::git(d, &["add", "-A"]).unwrap();
    clm_rename::rename(d, id).unwrap()
}

/// A change `id` that records no renames, with `status`.
fn plain_change(d: &Path, id: &str, status: &str, delta: &str) {
    write(
        d,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n\n## Intent\nEdit.\n"),
    );
    write(d, &format!("changes/{id}/deltas/a.md"), delta);
}

fn renaming(renames: &str) -> String {
    format!("---\nstatus: proposed\nrenames:\n{renames}---\n\n## Intent\nRename.\n")
}

fn swap() -> String {
    renaming("  - term: sprocket\n    to: cogs\n")
}

/// The hash `clm hashes` gives `element` of component `a` in the tree at `d`.
fn hash(d: &Path, element: &str) -> String {
    let graph = clm_model::load(d).0;
    let hashes = clm_apply::hashes(&graph.nodes["a"]);
    hashes.into_iter().find(|(e, _)| e == element).unwrap().1
}

/// Every violation the load and `apply`'s validation report for change `id`.
fn violations(d: &Path, id: &str) -> Vec<String> {
    let (graph, loaded) = clm_model::load(d);
    let mut all: Vec<String> = loaded.iter().map(|v| v.message.clone()).collect();
    if let Some(change) = graph.changes.get(id) {
        let found = clm_apply::validate(&graph, change);
        all.extend(found.iter().map(|v| v.message.clone()));
    }
    all
}

#[test]
// req:rename/other-deltas@e4bc7840
fn a_reworded_contract_another_change_modifies() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    let (contract, design) = (hash(d, "contract"), hash(d, "design"));
    plain_change(
        d,
        "y",
        "proposed",
        &format!(
            "---\ncomponent: a\nbase: {{ contract: \"{contract}\", design: \"{design}\" }}\n---\n## MODIFIED Contract\nPrint the sprocket.\n\nExit 0.\n\n## MODIFIED Design\nRead the file twice.\n"
        ),
    );
    write(d, "changes/r/change.md", &swap());
    let printed = rename(d, "r");
    assert_eq!(
        printed,
        [
            "renamed sprocket -> cogs",
            "rebased contract of a in changes/y/deltas/a.md"
        ]
    );
    let fresh = hash(d, "contract");
    assert_ne!(fresh, contract);
    assert_eq!(
        read(d, "changes/y/deltas/a.md"),
        format!(
            "---\ncomponent: a\nbase: {{ contract: \"{fresh}\", design: \"{design}\" }}\n---\n## MODIFIED Contract\nPrint the cogs.\n\nExit 0.\n\n## MODIFIED Design\nRead the file twice.\n"
        )
    );
    assert_eq!(violations(d, "y"), Vec::<String>::new());
}

#[test]
// req:rename/other-deltas@e4bc7840
fn a_removed_requirement_keeps_its_removal() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    let x = hash(d, "requirement:x");
    let delta = format!(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: x <!-- base: {x} -->\n"
    );
    plain_change(d, "y", "proposed", &delta);
    write(d, "changes/r/change.md", &swap());
    let printed = rename(d, "r");
    assert_eq!(
        printed[1],
        "rebased requirement:x of a in changes/y/deltas/a.md"
    );
    let fresh = hash(d, "requirement:x");
    assert_ne!(fresh, x);
    assert_eq!(read(d, "changes/y/deltas/a.md"), delta.replace(&x, &fresh));
    assert_eq!(violations(d, "y"), Vec::<String>::new());
}

fn architecture(slugs: &[&str]) -> String {
    let entries: String = slugs
        .iter()
        .map(|s| {
            format!(
                "### Constraint: {s}\nA SHALL.\n\n#### Scenario: s\n- ok\n\n#### Untested\nJudged.\n\n"
            )
        })
        .collect();
    format!("# A\n\n## Constraints\n\n{entries}")
}

#[test]
// req:rename/other-deltas@e4bc7840
fn frontmatter_and_requirement_blocks_of_an_approved_change() {
    let spec = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\nserves: [architecture/old]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL print.\n\nSee `architecture/old`.\n\n#### Scenario: holds\n- ok\n";
    let dir = repo(spec);
    let d = dir.path();
    write(d, "ARCHITECTURE.md", &architecture(&["old", "other"]));
    let (fm, x) = (hash(d, "frontmatter"), hash(d, "requirement:x"));
    let delta = format!(
        "---\ncomponent: a\nbase: {{ frontmatter: \"{fm}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\nserves: [architecture/old, architecture/other]\n```\n\n## MODIFIED Requirements\n### Requirement: x <!-- base: {x} -->\nX SHALL print more.\n\nSee `architecture/old`.\n\n#### Scenario: holds\n- ok\n\n#### Serves\n- architecture/old\n"
    );
    plain_change(d, "y", "approved", &delta);
    write(
        d,
        "changes/r/change.md",
        &renaming("  - requirement: architecture/old\n    to: new\n"),
    );
    let printed = rename(d, "r");
    assert_eq!(
        printed,
        [
            "renamed architecture/old -> architecture/new",
            "rebased frontmatter of a in changes/y/deltas/a.md",
            "rebased requirement:x of a in changes/y/deltas/a.md"
        ]
    );
    let expected = delta
        .replace(&fm, &hash(d, "frontmatter"))
        .replace(&x, &hash(d, "requirement:x"))
        .replace("[architecture/old,", "[architecture/new,")
        .replace("See `architecture/old`", "See `architecture/new`")
        .replace("- architecture/old", "- architecture/new");
    assert_eq!(read(d, "changes/y/deltas/a.md"), expected);
    assert_eq!(violations(d, "y"), Vec::<String>::new());
}

#[test]
// req:rename/other-deltas@e4bc7840
fn a_stale_base_and_the_rest_of_the_change_stay() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    let delta = "---\ncomponent: a\nbase: { design: \"0000000000000000\", frontmatter: \"1111111111111111\" }\n---\n## MODIFIED Frontmatter\n```yaml\nstatus: active\n```\n\n## MODIFIED Design\nRead the sprocket twice.\n\n## ADDED Requirements\n### Requirement: w\nW SHALL print the sprocket.\n\n#### Scenario: holds\n- ok\n";
    plain_change(d, "y", "proposed", delta);
    write(d, "changes/y/assets/a/note.txt", "the sprocket\n");
    write(d, "changes/r/change.md", &swap());
    assert_eq!(rename(d, "r"), ["renamed sprocket -> cogs"]);
    assert_eq!(read(d, "changes/y/deltas/a.md"), delta);
    assert_eq!(read(d, "changes/y/assets/a/note.txt"), "the sprocket\n");
}

#[test]
// req:rename/other-deltas@e4bc7840
fn a_frontmatter_the_rename_left_alone_stays() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    let fm = hash(d, "frontmatter");
    let delta = format!(
        "---\ncomponent: a\nbase: {{ frontmatter: \"{fm}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\nstatus: active\n```\n"
    );
    plain_change(d, "y", "proposed", &delta);
    write(d, "changes/r/change.md", &swap());
    assert_eq!(rename(d, "r"), ["renamed sprocket -> cogs"]);
    assert_eq!(read(d, "changes/y/deltas/a.md"), delta);
}

#[test]
// req:rename/other-deltas@e4bc7840
fn several_changes_and_a_removal_print_in_order() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    write(d, "notes.txt", "the sprocket\n");
    let (fm, c, ds, x) = (
        hash(d, "frontmatter"),
        hash(d, "contract"),
        hash(d, "design"),
        hash(d, "requirement:x"),
    );
    let removal = format!(
        "---\ncomponent: a\nremove: true\nbase: {{ frontmatter: \"{fm}\", contract: \"{c}\", design: \"{ds}\" }}\n---\n## REMOVED Requirements\n### Requirement: x <!-- base: {x} -->\n"
    );
    plain_change(d, "z", "proposed", &removal);
    let modify = format!(
        "---\ncomponent: a\nbase: {{ contract: \"{c}\" }}\n---\n## MODIFIED Contract\nPrint the sprocket.\n"
    );
    plain_change(d, "y", "proposed", &modify);
    write(d, "changes/r/change.md", &swap());
    assert_eq!(
        rename(d, "r"),
        [
            "renamed sprocket -> cogs",
            "rebased contract of a in changes/y/deltas/a.md",
            "rebased contract of a in changes/z/deltas/a.md",
            "rebased requirement:x of a in changes/z/deltas/a.md",
            "still names sprocket notes.txt:1",
        ]
    );
    let fresh = hash(d, "contract");
    assert_eq!(
        read(d, "changes/z/deltas/a.md"),
        removal
            .replace(&c, &fresh)
            .replace(&x, &hash(d, "requirement:x"))
    );
}

#[test]
// req:rename/other-deltas@e4bc7840
fn an_applied_change_is_not_rebased() {
    let dir = repo(SPROCKET);
    let d = dir.path();
    let contract = hash(d, "contract");
    let delta = format!(
        "---\ncomponent: a\nbase: {{ contract: \"{contract}\" }}\n---\n## MODIFIED Contract\nPrint the sprocket.\n"
    );
    plain_change(d, "y", "applied", &delta);
    write(d, "changes/r/change.md", &swap());
    assert_eq!(rename(d, "r"), ["renamed sprocket -> cogs"]);
    assert_eq!(read(d, "changes/y/deltas/a.md"), delta);
}
