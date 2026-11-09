---
title: "A changed design, story or visual asset reaches no build, and cross-cutting design has no home"
kind: idea
components: [model, apply, check, gate, init]
---

Products made with CLM aren't only code and rules: a web app has UX flows and a design system, a game has journeys, story beats, an art bible and a tone of voice, and both ship content (copy, dialogue, levels, images, audio). `node-assets` gives a component's spec a place for files like mockups (`specs/<component>/assets/`, hashed, listed by `clm context`, changed only through deltas and `clm apply`, checked by `check/asset-references`), but four things are missing.

## The problems

1. **A changed design builds nothing.** Pending work is counted per requirement (unbuilt, behind, obsolete), and `node-assets` says requirements win when an asset disagrees with them. A new mockup changes the component's bundle hash, so a build in progress sees it, but it makes no test `behind`, so `clm next` never schedules a rebuild and nothing proves the code matches the new design. A changed UX design, game journey or visual asset reaches the code only if the same change also rewrites a requirement.
2. **Cross-cutting design has no home.** The root component has no assets, and the human layers (`CAPABILITIES.md`, `ARCHITECTURE.md`) are text only. A design system, story bible, world map or journey storyboard belongs to no single component.
3. **Design changes skip review.** An asset-only change isn't among what `humans-review-intent` makes need review, so it lands on checks. A UX redesign or a changed story beat is intent, which a human should see.
4. **Content edits can't ask for a human.** Shipped content is product material, like code, and an edit that keeps every rule true takes the small-change route on checks. For creative work, a writer or designer may want to review those edits anyway, and CLM has no setting for "these paths need a human".

## Three kinds of thing

Most of the confusion comes from treating these as one:

| Kind | Examples | What it is | Where it belongs |
|---|---|---|---|
| Experience intent | journeys, story beats, UX flows, art direction, tone of voice | spec: what the product should be like | the human layers and assets |
| Shipped content | dialogue files, levels, copy, images, audio | product material, like code | a component's `paths` |
| Evidence | screenshots, playthrough recordings | test output | not in git, or test fixtures |

A changed line of dialogue is like a code edit. A changed story beat ("the mentor dies before act 2") is a change of intent.

## What already fits, with no change

- **Journeys are capabilities.** A capability is what a person can do, seen from outside, with scenarios: "GIVEN the player has the key, WHEN they reach the bridge, THEN the gate opens and the mentor's line plays". A scripted, seeded playthrough at the game's boundary proves it, as Playwright proves CLM's web capabilities (`human-layer-proof`). What no program can judge ("this moment feels earned") takes `#### Untested`, and a human playtest is a `human-task` item.
- **Cross-cutting look-and-feel rules are architecture requirements**, each with `#### Enforced by`: contrast ratios, "no copy uses exclamation marks", "every string goes through localization", "a scene's textures stay under 2 MB".
- **Content is code.** A `content` component owns `content/**`, and its requirements are structural rules (schema-valid, every dialogue node reachable, every asset reference resolves, budgets met), proven by tests that read the content.

## Direction

CLM should decide where intent lives and how a change to it reaches builds and tests, since that is what CLM is for. It should stay out of formats and tools: no dialogue-tree schema, no Figma integration, no story template in the binary. Projects differ too much, and anything domain-specific would be dead weight in most repositories.

- **A requirement can adopt an asset (main piece, fixes 1).** A requirement whose text cites `` `assets/<path>` `` ("the settings screen SHALL match `assets/settings.png`'s layout") folds that asset's hash into its own version, so changing the asset makes the tests citing the requirement `behind`, the component pending, and `clm next` schedules the rebuild, proven again by, say, a visual-snapshot test citing the requirement. An asset no requirement cites stays informative, as now, so "requirements win" still holds. This changes how a requirement's version is computed (`versioned-markers`, `hash-based-staleness`, the model's hashing, `clm hashes`), widens `check/asset-references` to requirement text (today mentions in requirements and scenarios never count), and `apply`'s stale-base handling has to treat an adopted asset's base like part of the requirement's. Open: whether a scenario's mention also adopts, and how a requirement adopts a whole directory (a sprite sheet set).
- **Assets on human-layer entries (fixes 2 and part of 3).** For example `capabilities/<slug>/assets/` for a journey's storyboard or flow diagram, and `architecture/<slug>/assets/` for a design system's tokens or an art bible. Editing one needs review like any edit to its layer. Since `serves` ties rules to capabilities, a changed storyboard could show which components serve the journey. This is preferred over a third Markdown layer (an `EXPERIENCE.md` owned by a design or creative lead): journeys already fit capabilities and cross-cutting look-and-feel rules already fit architecture, so what is missing is a place for non-text files, not a new kind of entry. Revisit if projects keep wanting experience entries that are neither.
- **Asset edits that serve a capability need review (rest of 3).** An asset change on a component that serves a capability needs review, perhaps as a project setting beside `approval` (`approval-setting-is-spec`).
- **Templates as guide pages and `clm init` examples, not tooling.** For instance "a game: capabilities as journeys, a `content` component, a seeded playthrough harness" and "a web app: a design system as architecture assets, mockups adopted by requirements, snapshot tests".

## Open questions

- **Content review (4).** Should a project be able to name paths whose edits need a human even on the small-change route (something like CODEOWNERS, but with the guard's approval), and does that break `yes-no-review`'s rule that the gate computes review from the diff alone?
- **Large binaries.** Art and audio belong in Git LFS. Should an asset's hash be the LFS pointer's hash (cheap, stable) or the content's (needs the object)? `size` already skips binary files.
- **Generated assets.** Exported sprites or compiled design tokens: is the source the asset, with the export as code, or both?
- **Snapshot tolerance.** A test proving an adopted mockup needs a project-chosen notion of "matches" (layout, not pixels). That's the project's harness, but the guide should say so.
