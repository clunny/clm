//! The `propose` skill, `plugin/skills/propose/SKILL.md`: the tests of `plugin.propose`'s requirements.

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin() -> PathBuf {
    repo().join("plugin")
}

#[test]
fn propose_brainstorms() {
    // req:plugin.propose/propose-brainstorms@3e173409
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "clm superpowers brainstorming",
        "## Approaches",
        "docs/superpowers/",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    // A prohibition, not a permission: the never comes before the path in its own
    // sentence, since a markdown line can hold a whole paragraph.
    let sentence = sentence_with(&body, "docs/superpowers/");
    let before = &sentence[..sentence.find("docs/superpowers/").unwrap()];
    assert!(
        before.contains("never") || before.contains("Never"),
        "propose: SKILL.md must say never to write under docs/superpowers/"
    );
    // Brainstorming comes before the change is written.
    let brainstorm = body.find("clm superpowers brainstorming").unwrap();
    let change = body
        .find("/change.md` with the intent")
        .expect("change.md step");
    assert!(
        brainstorm < change,
        "propose: SKILL.md must brainstorm before writing change.md"
    );
}

#[test]
fn propose_claims_items_before_designing() {
    // req:plugin.propose/propose-claims@63373bf0
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(node_syntax_gaps(&body), Vec::<String>::new());
    // one flag for several nodes: the ambiguous form is reported, naming the right one,
    // even when another step still writes the right form.
    let ambiguous = body.replacen(
        PROPOSALS_FORM,
        "clm proposals --component <component>...",
        1,
    );
    let gaps = node_syntax_gaps(&ambiguous);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    let gaps =
        node_syntax_gaps(&body.replacen(PROPOSALS_FORM, "clm proposals --component <a> <b>", 1));
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    let ambiguous = body.replace(PROPOSALS_FORM, "clm proposals --component <component>...");
    let gaps = node_syntax_gaps(&ambiguous);
    assert!(
        !gaps.is_empty() && gaps.iter().all(|g| g.contains(PROPOSALS_FORM)),
        "{gaps:?}"
    );
    for needle in [
        "clm proposals",
        "clm claim backlog/",
        "clm proposals --component",
        "refused",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    let steps = numbered_steps(&body);
    let position = |needle: &str| {
        steps
            .iter()
            .position(|s| s.contains(needle))
            .unwrap_or_else(|| panic!("propose: SKILL.md has no step with {needle}"))
    };
    // Items are left out, then claimed, before brainstorming.
    let proposals = position("clm proposals");
    let claim = position("clm claim backlog/");
    let brainstorm = position("clm superpowers brainstorming");
    assert!(
        proposals < brainstorm,
        "propose: `clm proposals` before brainstorming"
    );
    assert!(
        claim < brainstorm,
        "propose: `clm claim backlog/` before brainstorming"
    );
    // Right after claiming, the spec branch holds the items.
    for needle in ["commit", "push", "`from`", "`resolves`", "status: proposed"] {
        assert!(
            steps[claim].contains(needle),
            "propose: the claim step must mention {needle}"
        );
    }
    let refused = sentence_with(&steps[claim], "refused");
    assert!(
        refused.contains("stop") && refused.contains("holds"),
        "propose: a refused claim stops, telling what holds the item: {refused}"
    );
    // The overlap check runs once the nodes are known, and again before the page, and the
    // human decides.
    let deltas = position("deltas/<component>.md");
    let pr = position("clm pr open");
    let overlap: Vec<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.contains("clm proposals --component"))
        .map(|(i, _)| i)
        .collect();
    assert!(
        overlap.iter().any(|&i| i < deltas),
        "propose: `clm proposals --component` before the deltas, got {overlap:?}"
    );
    assert!(
        overlap.iter().any(|&i| i > deltas && i <= pr),
        "propose: `clm proposals --component` again before the page, got {overlap:?}"
    );
    for needle in ["carry on", "fold", "wait"] {
        assert!(
            steps[overlap[0]].contains(needle),
            "propose: the overlap check must offer to {needle}"
        );
    }
    // Only a `proposed` or `claimed` line is a question; a `main` line needs none.
    for &i in &overlap {
        let ask = sentence_with(&steps[i], "`proposed` or `claimed` line");
        assert!(ask.contains("ask"), "{ask}");
    }
    let main = sentence_with(&body, "`main` line");
    for needle in ["applied on `main`", "ask nothing"] {
        assert!(main.contains(needle), "{needle}: {main}");
    }
    // A claimed item already fixed stays in `resolves`: deleted, and `## Intent` says why.
    let fixed = sentence_with(&body, "already fixed");
    for needle in ["`resolves`", "`backlog/<slug>.md`", "`## Intent`"] {
        assert!(fixed.contains(needle), "{needle}: {fixed}");
    }
    // skill without already-fixed items; asks about applied changes
    for needle in ["already fixed", "`main` line"] {
        let gaps = claim_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
    assert_eq!(claim_gaps(&body), Vec::<String>::new());
}

#[test]
fn propose_follows_agent_harness() {
    // req:plugin.propose/propose-follows-agent-harness@f2fd7f68
    let body = skill_body("propose");
    assert_eq!(agent_harness_gaps(&body), Vec::<String>::new());
    for needle in ["clm agent-harness", "confirm", "own terminal"] {
        let gaps = agent_harness_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
    // The check and the audit run before the pull request, and `subagents` `no` is named.
    let check = numbered_steps(&body)
        .into_iter()
        .find(|s| s.contains("Run a builder check"))
        .unwrap();
    for needle in [
        "`clm agent-harness`",
        "`subagents` is `no`",
        "one after the other",
    ] {
        assert!(check.contains(needle), "{needle}: {check}");
    }
    let ask = sentence_with(&body, "`confirm` is `no`");
    let step = numbered_steps(&body)
        .into_iter()
        .find(|s| s.contains("`confirm` is `no`"))
        .unwrap();
    for needle in ["own terminal", "never run `clm approve` yourself"] {
        assert!(step.contains(needle), "{needle}: {ask}");
    }
}

#[test]
fn propose_archives_design_only_changes() {
    // req:plugin.propose/propose-archives-design-only@b3297811
    let body = skill_body("propose");
    assert_eq!(design_only_gaps(&body), Vec::<String>::new());
    // skill without archiving
    let gaps = design_only_gaps(&body.replace("clm archive", "elided"));
    assert_reports(&gaps, "propose", "clm archive");
    // skill resolving `specs/` by hand on an archived change
    let said = body.replacen(
        "(`archive-in-spec-pr`).",
        "(`archive-in-spec-pr`). Resolve a conflict under `specs/` by hand.",
        1,
    );
    assert_ne!(said, body);
    let gaps = design_only_gaps(&said);
    assert_reports(&gaps, "propose", "by hand");
    // A change with no requirement added, modified or removed is archived in the spec
    // branch, after the builder check and as the last commit before the page.
    let archive = body
        .find("clm archive <id>")
        .expect("propose: `clm archive <id>`");
    let check = body.find("Run a builder check").unwrap();
    let page = body.find("clm pr open spec/<id>").unwrap();
    assert!(
        check < archive && archive < page,
        "propose: archive between check and page"
    );
    let when = sentence_with(&body, "archive-in-spec-pr");
    for needle in ["add, modify and remove no requirement", "nothing to build"] {
        assert!(when.contains(needle), "{needle}: {when}");
    }
    // skill without the promised work recorded
    for needle in [
        "kind: deferred",
        "deferred-work-resolves-in-code",
        "no requirement it adds or modifies makes pending",
    ] {
        let gaps = design_only_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
    let how = sentence_with(&body, "clm archive <id>");
    for needle in [
        "last commit before opening the pull request",
        "status: done",
        "`resolves`",
        "clm check",
    ] {
        assert!(how.contains(needle), "{needle}: {how}");
    }
}

#[test]
fn propose_sets_priority() {
    // req:plugin.propose/propose-priority@b217daba
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "`priority`",
        "check/priority-caps",
        "highest priority first",
        "clm backlog",
        "`P5`",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    // The human picks which holder to demote.
    let sentence = sentence_with(&body, "check/priority-caps");
    assert!(
        sentence.contains("ask") && sentence.contains("demote"),
        "propose: on check/priority-caps, ask the human which holder to demote: {sentence}"
    );
    assert!(
        body.contains("never choosing one yourself") || body.contains("never choose one yourself"),
        "propose: SKILL.md must leave the demotion to the human"
    );
    // The change's priority comes from its items, and P5 all round means none.
    let sentence = sentence_with(&body, "highest `priority`");
    for needle in ["`from`", "`resolves`", "`P5`"] {
        assert!(sentence.contains(needle), "{sentence}");
    }
    // Suggestions follow `clm backlog`'s order.
    let sentence = sentence_with(&body, "highest priority first");
    assert!(sentence.contains("clm backlog"), "{sentence}");
    // propose suggests only ideas, and points at triage for the rest.
    assert_eq!(propose_triage_gaps(&body), Vec::<String>::new());
    let sentence = sentence_with(&body, "/clm:triage");
    for needle in [
        "only the remaining `idea` items",
        "`bug`, `question`, `human-task`, `deferred` and `workaround`",
        "no `idea` item is left",
        "triage-owns-backlog-work",
        "of any kind the user names is still taken as the intent",
    ] {
        assert!(sentence.contains(needle), "{needle}: {sentence}");
    }
    // propose without the triage pointer
    assert_reports(
        &propose_triage_gaps(&body.replace("/clm:triage", "")),
        "propose",
        "/clm:triage",
    );
}

#[test]
fn propose_elaborates_ideas() {
    // req:plugin.propose/propose-elaborates-ideas@839ccda3
    let full = skill_body("propose");
    // The rules sit in a section of their own, outside the numbered steps.
    let section = full
        .split("\n## ")
        .find(|s| s.starts_with("Elaborating an idea"))
        .expect("propose: a section on elaborating an idea");
    let body = section.to_owned();
    assert_eq!(elaboration_gaps(&body), Vec::<String>::new());
    for needle in [
        "when the claim is refused",
        "`title`, `kind: idea` and `components`",
        "no `change.md` and no delta",
        "never landing at every pause",
        "`rebase ok`",
        "`code`",
        "afresh from `origin/main`",
        "once the claim has lapsed",
    ] {
        assert!(
            body.contains(needle),
            "propose: the section must mention {needle}"
        );
    }
    // skill without elaboration
    assert_reports(
        &elaboration_gaps(&body.replace("## Settled", "")),
        "propose",
        "## Settled",
    );
    for needle in [
        "git ls-remote origin 'refs/heads/spec/*-idea-<slug>'",
        "spec/<YYYYMMDD>-idea-<slug>",
        "## Intent",
        "## Approaches",
        "propose/<slug>",
        "autosync/spec/<YYYYMMDD>-idea-<slug>",
        "clm branch delete",
        "elaborate-ideas-on-main",
        "from: backlog/<slug>",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
}

#[test]
fn propose_nests_splits() {
    // req:plugin.propose/propose-nests-splits@7b381998
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "specs/<component>/<part>/SPEC.md",
        "`<component>.<part>`",
        "`slim-components`",
        "`nested-splits`",
        "clm guide shaping-the-graph",
        "clm survey --components",
        "`splits-import-one-way`",
        "outside: [\"**\"]",
        "## Edges the change removes",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    // The carved-out part becomes a child of the component it came from.
    let sentence = sentence_with(&body, "specs/<component>/<part>/SPEC.md");
    assert!(
        sentence.contains("make the carved-out part a child of the component it came from"),
        "propose: a carved-out part becomes a child of its node: {sentence}"
    );
    // The product's ID stays on the half that stays.
    let sentence = sentence_with(&body, "product's ID");
    assert!(
        sentence.contains("Keep the product's ID on the half that stays"),
        "{sentence}"
    );
    // Requirements move unchanged: removed from the parent, added to the child.
    let sentence = sentence_with(&body, "Move each requirement unchanged");
    let removed = sentence.find("REMOVED from the parent").expect(sentence);
    let added = sentence.find("ADDED").expect(sentence);
    let child = sentence.find("to the child").expect(sentence);
    assert!(removed < added && added < child, "{sentence}");
    // An undeclared edge may stand only when all three hold; a cycle never does.
    let sentence = sentence_with(&body, "`survey/undeclared-edge`");
    for needle in [
        "importing file moves into a crate or package of the child's own",
        "states the edge's removal",
        "whose `#### Needs` names the child's",
        "`## Edges the change removes`",
    ] {
        assert!(sentence.contains(needle), "propose: {needle}: {sentence}");
    }
    assert!(sentence.contains("stand"), "{sentence}");
    let sentence = sentence_with(&body, "`survey/cycle`");
    assert!(sentence.contains("never stands"), "{sentence}");
    // Seams only, never a size budget.
    let sentence = sentence_with(&body, "architectural seams");
    assert!(
        sentence.contains("never") && sentence.contains("budget"),
        "propose: never split for a budget alone: {sentence}"
    );
}

#[test]
fn propose_offers_code_fixes() {
    // req:plugin.propose/propose-code-fixes@528d2c04
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(code_fix_gaps(&body), Vec::<String>::new());
    // A test file lies in the nodes its markers cite too, so its fix claims the cited node.
    let cited = sentence_with(&body, "fixes-count-where-cited");
    for needle in [
        "`req:` markers",
        "`paths`",
        "claims the component the test cites",
    ] {
        assert!(cited.contains(needle), "{needle}: {cited}");
    }
    // Each scenario: a skill without the piece fails, naming it.
    for needle in [
        "Resolves: backlog/",
        "clm claim <component> --fix",
        "fixes-count-where-cited",
    ] {
        let gaps = code_fix_gaps(&body.replace(needle, "elided"));
        assert!(
            gaps.len() == 1 && gaps[0].contains(needle),
            "{needle}: {gaps:?}"
        );
    }
    // Brainstorming is where the offer is made: a code-only bug gets a code branch, on a yes.
    let steps = numbered_steps(&body);
    let brainstorm = steps
        .iter()
        .find(|s| s.contains("clm superpowers brainstorming"))
        .expect("propose: SKILL.md has a brainstorming step");
    let offer = sentence_with(brainstorm, "code branch");
    for needle in [
        "`kind: bug`",
        "code branch instead of a spec branch",
        "on a no",
        "changes nothing under `specs/`, `changes/`",
        "`components`",
        "offer",
        "Code fixes",
    ] {
        assert!(offer.contains(needle), "{needle}: {offer}");
    }
    // The path itself, in order: drop the spec branch, claim the node for the fix, fix it
    // test first, delete the item with the trailer, gate `code`, open the page and land.
    let path = code_fix_section(&body);
    let order = [
        "`spec/<id>`",
        "`autosync/spec/<id>`",
        "clm claim <component> --fix backlog/<slug>",
        "test first",
        "`backlog/<slug>.md`",
        "`Co-Authored-By`",
        "`Resolves: backlog/<slug>`",
        "`clm gate --base origin/main`",
        "clm branch push",
        "clm pr open",
        "clm land",
    ];
    let mut at = 0;
    for needle in order {
        let found = path[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("propose: `## Code fixes` must name {needle} after {at}"));
        at += found + needle.len();
    }
    let drop = sentence_with(path, "`spec/<id>`");
    for needle in ["delete", "`origin`", "`autosync/spec/<id>`"] {
        assert!(drop.contains(needle), "{needle}: {drop}");
    }
    let gate = sentence_with(path, "clm gate --base origin/main");
    assert!(gate.contains("`code`"), "{gate}");
    // A fix the path can't carry stays a spec branch.
    let fallback = sentence_with(path, "stays a spec branch");
    for needle in [
        "more than one component",
        "`components`",
        "spec change",
        "refused",
    ] {
        assert!(fallback.contains(needle), "{needle}: {fallback}");
    }
}

#[test]
fn propose_code_fixes_need_a_written_rule() {
    // req:plugin.propose/code-fix-needs-a-written-rule@00b46bab
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(written_rule_gaps(&body), Vec::<String>::new());
    let section = code_fix_section(&body);
    // Only a behaviour a requirement already states takes the code-branch route.
    let only = sentence_with(section, "as written");
    for needle in [
        "only when",
        "requirement or scenario",
        "cites that requirement",
    ] {
        assert!(only.contains(needle), "{needle}: {only}");
    }
    // Otherwise a spec branch writes the rule first, and the fix follows citing it.
    let otherwise = sentence_with(section, "code-fix-needs-a-written-rule");
    for needle in [
        "spec branch",
        "adds the requirement or scenario",
        "`impl/<component>`",
    ] {
        assert!(otherwise.contains(needle), "{needle}: {otherwise}");
    }
    // skill without the rule: each missing piece is reported, naming the skill.
    // A bug in the capability suite's harness alone is the exception, never a requirement.
    let exception = sentence_with(section, "harness-only");
    for needle in [
        "harness",
        "`propose-harness-only-work`",
        "never gets a requirement",
    ] {
        assert!(exception.contains(needle), "{needle}: {exception}");
    }
    for needle in [
        "code-fix-needs-a-written-rule",
        "as written",
        "harness-only",
        "own test suite",
        "only an architecture entry states",
    ] {
        let gaps = written_rule_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
}

#[test]
fn propose_clears_workarounds() {
    // req:plugin.propose/propose-clears-workarounds@33b6c8ba
    let body = skill_body("propose");
    let rule = body
        .lines()
        .find(|l| l.contains("delete each `workaround` backlog item"))
        .expect("propose: SKILL.md must tell Claude to delete the workaround items a delta clears");
    for needle in [
        "modifies or removes a requirement",
        "whose `requirement` names it",
        "re-record the item's `requirement` at the version `clm hashes <component>` prints after `clm apply`",
        "the model rejects a flag at an old version",
        "delete the `reconsider-<component>-<slug>` question the change answers",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // A skill that doesn't mention `workaround` is reported.
    let gaps = text_gaps(
        "propose",
        &body.replace("workaround", "elided"),
        &["workaround"],
        &[],
    );
    assert_reports(&gaps, "propose", "workaround");
}

#[test]
fn propose_in_a_fork() {
    // req:plugin.propose/propose-in-a-fork@c67f83af
    let body = skill_body("propose");
    assert_eq!(fork_gaps(&body), Vec::<String>::new());
    let section = fork_section(&body);
    // Asked once, before the branch is cut.
    let ask = sentence_with(section, "clm contribution parent");
    for needle in ["once", "before cutting the branch", "parent", "own project"] {
        assert!(ask.contains(needle), "{needle}: {ask}");
    }
    // Work going back: sync first, neither land nor approve, then open the pull request.
    let sync = sentence_with(section, "clm contribution sync");
    assert!(sync.contains("first"), "{sync}");
    let no_land = sentence_with(section, "neither");
    assert!(
        no_land.contains("clm land") && no_land.contains("clm approve"),
        "{no_land}"
    );
    let mut at = 0;
    for needle in [
        "clm contribution sync",
        "clm contribution open",
        "Allow edits by maintainers",
        "maintainer reviews it",
        "unless they ask",
        "clm guide contributions",
    ] {
        at += section[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("propose: fork section must name {needle} after {at}"))
            + needle.len();
    }
    // A skill without it: each missing piece is reported, naming the skill.
    for needle in [
        "clm contribution parent",
        "clm contribution sync",
        "clm contribution open",
    ] {
        let gaps = fork_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
}

#[test]
fn propose_adds_glossary_terms() {
    // req:plugin.propose/propose-adds-glossary-terms@ab62ecc4
    let body = skill_body("propose");
    assert_eq!(glossary_terms_gaps(&body), Vec::<String>::new());
    // a skill without it
    // the path, the rule and each kind of term are named in the paragraph
    for word in [
        "docs/guide/glossary.md",
        "clm/glossary-covers-terms",
        "frontmatter",
    ] {
        let gaps = glossary_terms_gaps(&body.replace(word, "elided"));
        assert!(gaps.iter().any(|g| g.contains(word)), "{word}: {gaps:?}");
    }
}

#[test]
fn propose_adds_settings_sections() {
    // req:plugin.propose/propose-adds-settings-sections@93ffbeeb
    let body = skill_body("propose");
    assert_eq!(settings_sections_gaps(&body), Vec::<String>::new());
    // a skill without it
    for word in [
        "docs/guide/settings.md",
        "config/settings-guide-covers-keys",
    ] {
        let gaps = settings_sections_gaps(&body.replace(word, "elided"));
        assert!(gaps.iter().any(|g| g.contains(word)), "{word}: {gaps:?}");
    }
    // It sits beside the glossary's paragraph, among the design steps.
    let glossary = body.find("docs/guide/glossary.md").unwrap();
    let settings = body.find("docs/guide/settings.md").unwrap();
    let steps_end = body.find("\n## ").unwrap();
    assert!(settings.abs_diff(glossary) < 1500 && settings < steps_end);
}

#[test]
fn propose_fix_runs_setup() {
    // req:plugin.propose/propose-fix-runs-setup@69f20c07
    let body = skill_body("propose");
    let section = code_fix_section(&body);
    assert_eq!(
        text_gaps(
            "propose",
            section,
            &["clm setup", "tests-run-in-their-own-tree", "[build] setup"],
            &[],
        ),
        Vec::<String>::new()
    );
    // It runs before any test in the checkout, and again after dependency changes.
    let rule = sentence_with(section, "When `clm.toml` sets `[build] setup`");
    for needle in [
        "in this checkout",
        "before you run any test",
        "and again after",
        "`clm rebase`",
        "dependency declarations",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // a skill without the rule
    let gaps = text_gaps(
        "propose",
        &body.replace("clm setup", "elided"),
        &["clm setup"],
        &[],
    );
    assert_reports(&gaps, "propose", "clm setup");
}

#[test]
fn propose_harness_only_work() {
    // req:plugin.propose/propose-harness-only-work@a8431dc3
    let body = skill_body("propose");
    assert_eq!(
        text_gaps(
            "propose",
            &body,
            &[
                "harness-only",
                "`small-change-no-claim`",
                "`code-fix-resolves-bug`",
                "`archive-in-spec-pr`",
                "`deferred-work-resolves-in-code`",
                "`node-reconciliation`",
                "`human-layer-proof`",
            ],
            &[],
        ),
        Vec::<String>::new()
    );
    // a skill without the harness route
    let gaps = text_gaps(
        "propose",
        &body.replace("harness-only", "elided"),
        &["harness-only"],
        &[],
    );
    assert_reports(&gaps, "propose", "harness-only");
    // Harness-only work is never a requirement, takes no claim, and deletes its item with a
    // trailer rather than `clm claim --fix`.
    let route = sentence_with(&body, "is never a requirement of any component");
    assert!(route.contains("harness-only"), "{route}");
    let claim = sentence_with(&body, "takes the small-change route");
    assert!(claim.contains("no claim"), "{claim}");
    let trailer = sentence_with(&body, "never a `clm claim --fix`");
    assert!(trailer.contains("Resolves: backlog/<slug>"), "{trailer}");
    // A Design edit alone builds nothing: a deferred item carries the code it owes, done in
    // the same session on a small-change branch.
    let design = sentence_with(&body, "a spec branch edits the Design with no requirement");
    assert!(
        design.contains("`kind: deferred`") && design.contains("builds nothing"),
        "{design}"
    );
}

#[test]
fn builder_check_asks_migrations() {
    // req:plugin.propose/builder-check-asks-migrations@16ee864d
    let step = builder_check_step();
    assert_eq!(migration_gaps(&step), Vec::<String>::new());
    let gaps = migration_gaps(&step.replace("upgrades-carry-migrations", "elided"));
    assert_reports(&gaps, "propose", "upgrades-carry-migrations");
}

#[test]
fn builder_check_asks_capability_tests() {
    // req:plugin.propose/builder-check-capability-tests@5dd84ff9
    let step = builder_check_step();
    assert_eq!(capability_test_gaps(&step), Vec::<String>::new());
    let gaps = capability_test_gaps(&step.replace("capability-tests-follow-rules", "elided"));
    assert_reports(&gaps, "propose", "capability-tests-follow-rules");
}

/// The YAML frontmatter and the body of a markdown file.
fn frontmatter(path: &Path) -> (serde_yaml::Value, String) {
    let text = fs::read_to_string(path).unwrap();
    let rest = text.strip_prefix("---\n").expect("frontmatter");
    let (yaml, body) = rest.split_once("\n---\n").expect("frontmatter end");
    (serde_yaml::from_str(yaml).unwrap(), body.to_owned())
}

/// The first sentence of `text` holding `needle`: sentences end at a newline or at a
/// period followed by a space, so file names such as `change.md` don't cut one short.
fn sentence_with<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains(needle))
        .unwrap_or_else(|| panic!("no sentence mentions {needle}"))
}

/// The top-level numbered steps of a skill, each with the lines under it.
fn numbered_steps(body: &str) -> Vec<String> {
    let mut steps = Vec::new();
    let mut step: Option<String> = None;
    for line in body.lines() {
        let numbered = line
            .split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        // A new number starts a step; other unindented text ends one.
        if numbered || !(line.is_empty() || line.starts_with(' ')) {
            steps.extend(step.take());
        }
        if numbered {
            step = Some(String::new());
        }
        if let Some(step) = step.as_mut() {
            step.push_str(line);
            step.push('\n');
        }
    }
    steps.extend(step);
    steps
}

/// What a skill lacks of `must` and names of `must_not`, one line per gap naming the skill
/// and the text. Empty when it says it all.
fn text_gaps(skill: &str, body: &str, must: &[&str], must_not: &[&str]) -> Vec<String> {
    let mut gaps: Vec<String> = must
        .iter()
        .filter(|needle| !body.contains(*needle))
        .map(|needle| format!("{skill}: SKILL.md must mention {needle}"))
        .collect();
    gaps.extend(
        must_not
            .iter()
            .filter(|needle| body.contains(*needle))
            .map(|needle| format!("{skill}: SKILL.md must not mention {needle}")),
    );
    gaps
}

/// The body of `skill`'s `SKILL.md`.
fn skill_body(skill: &str) -> String {
    frontmatter(&plugin().join(format!("skills/{skill}/SKILL.md"))).1
}

/// Asserts `gaps` reports `needle` for `skill`, and nothing else's.
fn assert_reports(gaps: &[String], skill: &str, needle: &str) {
    assert!(
        !gaps.is_empty()
            && gaps
                .iter()
                .all(|g| g.starts_with(&format!("{skill}:")) && g.contains(needle)),
        "{needle}: {gaps:?}"
    );
}

/// The overlap check as the CLI takes it: `--node` once per node (`propose-claims`).
const PROPOSALS_FORM: &str = "clm proposals --component <component> [--component <component>]...";

/// How a `propose` skill writes the overlap check. Every inline `` `clm proposals --node` ``
/// command must be exactly [`PROPOSALS_FORM`], and there must be one. Empty when it is.
fn node_syntax_gaps(body: &str) -> Vec<String> {
    let mut gaps: Vec<String> = body
        .match_indices("`clm proposals --component")
        .map(|(at, _)| {
            let command = &body[at + 1..];
            &command[..command.find('`').unwrap_or(command.len())]
        })
        .filter(|command| *command != PROPOSALS_FORM)
        .map(|command| {
            format!("propose: SKILL.md writes `{command}`; write the form `{PROPOSALS_FORM}`")
        })
        .collect();
    if !body.contains(PROPOSALS_FORM) {
        gaps.push(format!(
            "propose: SKILL.md must write the form `{PROPOSALS_FORM}`"
        ));
    }
    gaps
}

/// What `propose-claims` asks of a `propose` skill beyond the overlap check's form.
fn claim_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "clm claim backlog/",
            "clm proposals --component",
            "already fixed",
            "`main` line",
        ],
        &[],
    )
}

/// What `propose-archives-design-only` asks of a `propose` skill: archiving, and no sentence
/// of the paragraph naming `archive-in-spec-pr` that has a conflict under `specs/` resolved
/// by hand, since `clm rebase` regenerates `specs/` for an archived change (`sync/reapply`).
fn design_only_gaps(body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "propose",
        body,
        &["clm archive", "status: done", "archive-in-spec-pr"],
        &[],
    );
    let by_hand = body
        .lines()
        .filter(|paragraph| paragraph.contains("archive-in-spec-pr"))
        .flat_map(|paragraph| paragraph.split(". "))
        .any(|sentence| sentence.contains("specs/") && sentence.contains("by hand"));
    if by_hand {
        gaps.push(
            "propose: an archived change's conflict under `specs/` isn't resolved by hand"
                .to_owned(),
        );
    }
    // The paragraph naming `clm archive` records the work the outcome promises.
    let paragraph = body
        .lines()
        .find(|l| {
            l.trim_start()
                .starts_with("A change whose deltas add, modify and remove no requirement")
        })
        .unwrap_or_default();
    for needle in ["kind: deferred", "deferred-work-resolves-in-code"] {
        if !paragraph.contains(needle) {
            gaps.push(format!(
                "propose: the paragraph naming `clm archive` must mention {needle}"
            ));
        }
    }
    let check = numbered_steps(body)
        .into_iter()
        .find(|s| s.contains("Run a builder check"))
        .unwrap_or_default();
    let needle = "no requirement it adds or modifies makes pending";
    if !check.contains(needle) {
        gaps.push(format!(
            "propose: the builder-check step must mention {needle}"
        ));
    }
    gaps
}

/// What `propose-follows-agent-harness` asks of the `propose` skill.
fn agent_harness_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &["clm agent-harness", "confirm", "own terminal"],
        &[],
    )
}

/// What `propose-priority` asks of the `propose` skill about ideas and triage.
fn propose_triage_gaps(body: &str) -> Vec<String> {
    text_gaps("propose", body, &["idea", "/clm:triage"], &[])
}

/// What `propose-elaborates-ideas` asks of the `propose` skill.
fn elaboration_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &["## Settled", "## Open questions", "-idea-", "clm wrap-up"],
        &[],
    )
}

/// What a `propose` skill's code-fix path leaves out (`code-fix-resolves-bug`), one line
/// per missing piece naming it. Empty when the path is all there.
fn code_fix_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "Resolves: backlog/",
            "clm claim <component> --fix",
            "fixes-count-where-cited",
        ],
        &[],
    )
}

/// The `## Code fixes` section of a `propose` skill, up to the next `## ` heading.
fn code_fix_section(body: &str) -> &str {
    let path = &body[body
        .find("## Code fixes")
        .expect("a `## Code fixes` section")..];
    &path[..path[3..].find("\n## ").map_or(path.len(), |end| end + 3)]
}

/// What `code-fix-needs-a-written-rule` asks of a `propose` skill's code-fix section, one
/// line per missing piece naming the skill and the text. Empty when it says it all.
fn written_rule_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        code_fix_section(body),
        &[
            "code-fix-needs-a-written-rule",
            "as written",
            "harness-only",
            "own test suite",
            "only an architecture entry states",
        ],
        &[],
    )
}

/// The `## Contributing back from a fork` section of a `propose` skill, up to the next `## `.
fn fork_section(body: &str) -> &str {
    let Some(at) = body.find("## Contributing back from a fork") else {
        return "";
    };
    let section = &body[at..];
    &section[..section[3..]
        .find("\n## ")
        .map_or(section.len(), |end| end + 3)]
}

/// What `propose-in-a-fork` asks of the `propose` skill, one line per missing piece.
fn fork_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        fork_section(body),
        &[
            "## Contributing back from a fork",
            "clm contribution parent",
            "clm contribution sync",
            "clm contribution open",
            "Allow edits by maintainers",
            "clm guide contributions",
            "clm land",
            "clm approve",
        ],
        &[],
    )
}

/// What `propose-adds-glossary-terms` asks of a `propose` skill: one paragraph (a line of
/// the skill) holding the glossary path, the rule, and the words for each kind of term.
fn glossary_terms_gaps(body: &str) -> Vec<String> {
    let must = [
        "docs/guide/glossary.md",
        "clm/glossary-covers-terms",
        "command",
        "status",
        "kind",
        "frontmatter",
    ];
    let paragraph = body
        .lines()
        .find(|line| line.contains(must[0]) && line.contains(must[1]));
    match paragraph {
        Some(line) => text_gaps("propose", line, &must, &[]),
        None => text_gaps("propose", "", &must, &[]),
    }
}

/// What `propose-adds-settings-sections` asks of a `propose` skill: one paragraph (a line of
/// the skill) holding the settings page's path and the rule.
fn settings_sections_gaps(body: &str) -> Vec<String> {
    let must = [
        "docs/guide/settings.md",
        "config/settings-guide-covers-keys",
    ];
    let paragraph = body
        .lines()
        .find(|line| line.contains(must[0]) && line.contains(must[1]));
    text_gaps("propose", paragraph.unwrap_or(""), &must, &[])
}

/// What `builder-check-asks-migrations` asks of the `propose` skill's builder-check step.
fn migration_gaps(step: &str) -> Vec<String> {
    text_gaps(
        "propose",
        step,
        &[
            "`clm init`",
            "migration",
            "requirement of `upgrade`",
            "`done` check",
            "`apply` or `note`",
            "upgrades-carry-migrations",
            "`required` migration is always an `apply`",
        ],
        &[],
    )
}

/// What `builder-check-capability-tests` asks of the `propose` skill's builder-check step.
fn capability_test_gaps(step: &str) -> Vec<String> {
    text_gaps(
        "propose",
        step,
        &[
            "capability tests",
            "capability-tests-follow-rules",
            "neither a requirement of `capability`",
            "nor a `#### Needs`",
            "nor a backlog item",
            "`CAPABILITIES.md` entry",
        ],
        &[],
    )
}

fn builder_check_step() -> String {
    numbered_steps(&skill_body("propose"))
        .into_iter()
        .find(|s| s.contains("Run a builder check"))
        .expect("builder check step")
}

/// What `propose-replacing-constraints` asks of the `propose` skill.
fn replacing_constraints_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "#### Replaces",
            "clm guide constraints",
            "`carried-constraints`",
            "`method`",
            "architecture-auditor",
            "## Open questions",
            "`deterministic-enforcement`",
        ],
        &[],
    )
}

#[test]
fn propose_replacing_constraints() {
    // req:plugin.propose/propose-replacing-constraints@44f3128f
    let body = skill_body("propose");
    assert_eq!(replacing_constraints_gaps(&body), Vec::<String>::new());
    // propose without replacements
    let gaps = replacing_constraints_gaps(&body.replace("#### Replaces", "elided"));
    assert_reports(&gaps, "propose", "#### Replaces");
    // The paragraph tells the tier, what CLM relies on, the guarantee and what is given up,
    // refuses a `method` constraint, and leaves the decision to the human.
    let paragraph = body
        .split("\n\n")
        .find(|p| p.contains("#### Replaces"))
        .expect("a paragraph on replacing constraints");
    for needle in [
        "its tier",
        "what CLM relies on",
        "what a replacement must guarantee",
        "what the project gives up",
        "Refuse to replace a `method` constraint",
        "`replaceable`",
        "`#### Enforced by` or `#### Untested`",
        "the human's approval, not the audit, decides",
        "under `## Open questions`",
        "The `architecture-auditor` judges",
        "`deterministic-enforcement`",
    ] {
        assert!(paragraph.contains(needle), "{needle}");
    }
}
