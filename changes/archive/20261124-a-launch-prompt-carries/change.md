---
status: done
resolves: [a-launch-prompt-carries-the-loop-s-notes-inline-so-the-session]
---

## Intent

The build loop keeps a launch's prompt a launcher of a few lines however long the human's notes for the loop are: it writes the notes the loop holds to a file of the run's own, and the prompt's notes section names that file for the run to read whole, rather than holding the notes inline (`notes-reach-every-launch`).

## Problem

`clm build start --notes <file>` keeps the human's notes, and every prompt file a `launch` names ends with them under `## The human's notes for this loop`. The build skill starts each subagent with the named file's text unchanged as its prompt. With notes of about 60 KB, every prompt was about 63 KB and the first step printed seven launches: the session would read each into its own context and emit it again as the Agent tool's prompt, about twice the file's size per launch, for twenty or more launches a wave, with a copying error possible each time. The session in the report worked around it by passing a one-line prompt telling the run to read the file.

The design is what's wrong, not the skill. `build-prompt-file` already settled this for every other prompt CLM writes: `clm run`, `clm draft` and `clm draft --fold` write the prompt to a file and print a launcher of a few lines that names it, so the session passes the launcher unchanged and the subagent reads the rest with the Read tool. The loop's prompt file is that launcher with a first line added, and the notes are the one part that breaks the pattern by being held inline.

## Approaches

- **The notes in a file the prompt names (chosen).** The prompt's notes section holds one line naming a copy of the notes by its absolute path, telling the run to read it whole and follow it. The prompt stays a few lines, the skill's "the named file's text unchanged" stays true and costs the session nothing, and the run reads one more file, as it already reads the prompt the launcher names.
- **The skill passes a pointer to the prompt file instead of its text.** This is the reporter's workaround made the rule. It changes the one instruction every agent harness adapter follows for every launch, so a session would pass a pointer to a file that is itself a pointer, and it leaves the loop as the one writer of a long launcher against `build-prompt-file`. Rejected.
- **The prompt names `loop.notes` itself.** No copy, but a run would read whatever the notes are when it reads them: a later `--notes` would change a running build's notes midway, and the drain's `end` removes the file under a run still going or relaunched. Rejected for a copy per run, which keeps the rule that a run gets the notes the loop held when it wrote its prompt.

## Outcome

- Each prompt file a `launch` names stays a few lines whatever the notes' size; its `## The human's notes for this loop` section holds one line naming `.clm/prompts/loop/notes/<name>.md` by its absolute path, which holds the notes unchanged.
- The copy is written with the prompt and replaced by the next write of that run's prompt, so a relaunch, an adoption by a later session and a resume see the same rule as today. It is in a directory of its own because a component ID may hold a dot, so a name beside the prompts (`build-web.notes.md`) could be another run's prompt.
- A prompt written while the loop holds no notes has no such section, as today; a copy an earlier write left is not named by it and does nothing.
- The skill, `clm build start --notes` and `loop.notes` don't change.

## Constraints

- `build-prompt-file`: every launcher CLM writes names its prompt rather than holding it.
- The run's own context still holds the notes in full; only the session's no longer does.

## What the requirements no longer state

- `notes-reach-every-launch` loses "Every prompt file a `launch` names, written when its run is prepared (`step-actions`), SHALL end with a section headed `## The human's notes for this loop` holding unchanged the notes the loop held when it wrote that prompt, and, when it held none or only white space, with no such section." The section now names a file holding the notes unchanged, in place of holding them.
- Its scenario *notes reach every launch* changes outcome under its name: the prompt file ends with the heading and a line naming the notes' copy, which holds `Keep the old tests.`, where it ended with `Keep the old tests.` itself.
