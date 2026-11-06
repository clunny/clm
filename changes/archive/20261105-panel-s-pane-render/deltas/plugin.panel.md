---
component: plugin.panel
---

## ADDED Requirements

### Requirement: superseded-draw
A pane draw whose `$` call the engine aborts, because a newer draw replaced it, SHALL end without throwing and draw nothing of its own, so the engine reports no hook as having thrown, and the newer draw SHALL draw the pane. An aborted call is one whose rejection's message ends in `aborted`. A run the engine aborted SHALL NOT be kept as the answer of `clm` or of git, so the newer draw runs it again. Any other failure of a draw SHALL be handled as before.

#### Scenario: opening replaces the first draw
- GIVEN a session in a CLM repository with no pane open
- WHEN the person runs `/clm:view`, and the engine aborts the first draw's `clm view` run once the redraw that opening starts replaces that draw
- THEN the first draw's hook resolves without drawing and without throwing, and the second draw runs `clm view` again and shows its chart

#### Scenario: another failure still shows
- GIVEN an open pane whose `clm view` rejects with a message that doesn't end in `aborted`
- WHEN it redraws
- THEN the pane shows why `clm` couldn't start, as before
