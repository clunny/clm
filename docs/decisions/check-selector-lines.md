---
title: "A land check names a step's selecting files with ::clm-selected:: lines"
date: 2026-11-08
status: accepted
---

Inside a step marked with GitHub Actions' `::group::` lines (`check-step-markers`), a check prints `::clm-selected::<path>` for each selected file that made it run the step. `clm land` keeps the paths with the step's time and traces each step's chain from its first path through the spec graph (`clm affected --why`). This amends `check-step-markers`, which kept CLM to GitHub Actions' form, by one line of CLM's own.

**Why a line of CLM's own** (`reuse-first`). No existing convention says which inputs made a script run a step. GitHub Actions' workflow commands name steps (`::group::`), annotate files (`::error file=...`) and set outputs, and none of these fits. An annotation would show as a warning or error in a GitHub log, and an output goes to a file, not to the step. Only the check knows why it ran a step: CLM hands it files, and the script's own rules, such as one suite always running after another, turn them into steps. The paths therefore have to come from the check, inside the step they explain. The line keeps the workflow-command shape (`::name::value`), so a GitHub log shows it as plain text, and a check that prints none loses only the chains.

**Alternatives.**
- *CLM guesses the step's files.* For example, from a step name that matches a directory. That's a convention of names that projects don't share, and it can't see a script rule that runs a step on another step's files, which is exactly the reach worth showing.
- *A mapping in `clm.toml` from steps to paths.* It duplicates the script's logic in a second place that drifts from it.
- *JUnit XML reports.* They give per-test times but not why a test ran, and need runner configuration in every project.
