---
status: applied
---

# The defect ask shows the report it asks about

## Intent

When `clm defect` would file its report as a public issue, the guard asks the human to confirm, saying to confirm only after reading the report, but the question doesn't show the report. Claude Code shows a hook's reason in that prompt; the command, and the report in it, appear only in the permission prompt that follows. A human reading the first prompt is told to judge text they can't see, which scares people off reporting at all. This change makes the guard quote the report, and the `clm defect` line that sends it, in its own question, and refuse a `clm defect` whose report it can't show, telling the session how to resend it.

## Problem

`plugin.hooks.guard/defect-approval` asks with a fixed reason, and the guard's Design assumes "Claude Code then shows the human the command before it runs". Claude Code's hook docs promise only that `permissionDecisionReason` is shown; in practice, from an outside project, the first prompt held the reason alone, and a second prompt then showed the command with the report. The guard reads the command but drops its heredoc bodies and redirections before the rules see them, so it can't say what the report is.

## Approaches

1. **The guard quotes the report in its reason, and refuses when it can't (chosen).** The report is the body of the segment's heredoc when that heredoc is what the shell feeds `clm defect`'s standard input and its text is what the shell sends: a quoted delimiter, or a body with no `$`, `` ` `` or `\` the shell would expand. Any other source (a file, a pipe, a here-string, a variable) is refused with exit 2 and a reason that tells the session to resend the report as a `<<'EOF'` heredoc. Whatever Claude Code's dialog shows or leaves out, the prompt that asks the human to read the report holds it. A file isn't read in its place: it could change between the prompt and the run, so what the human read might not be what is filed.
2. **Reword the question only** ("the report is shown in the next prompt"). Cheap, but it depends on an undocumented layout that can differ by permission mode, and the first prompt would still show nothing.
3. **Have `clm defect` itself show the report and ask.** Under Claude Code the command has no terminal to ask on, and the guard's prompt is the one that makes the yes the human's.

The human chose approach 1, and refusing over asking when the report can't be shown: a yes to a report no one can see isn't informed.

## Outcome

- `plugin.hooks.guard.shell/guard-standard-input` (added): each piece the walk yields says what feeds its standard input: its heredoc's body and whether the shell expands it, a here-string, a file, or a pipe.
- `plugin.hooks.guard/defect-approval` (modified): the ask's reason quotes each `clm defect` segment's own line and its report; a segment whose report the guard can't show is refused.
- `plugin.hooks/defect-rule` (modified): with `defects = "ask"`, the session-start rule tells sessions to send the report as a `<<'EOF'` heredoc, so they don't meet the refusal first.
- Refused too: a heredoc with no line ending it, and any `<&` input. A refusal wins over another rule's ask.
- The guard's Design no longer claims Claude Code shows the command with the reason.
- Decision `defect-ask-shows-report`.

## Constraints

- With `defects = "backlog"` or `"off"` nothing changes: the report never leaves the project, so nothing asks or refuses.
- The heredoc body still matches no guard rule (`guard-heredocs`); quoting it in the reason doesn't judge it.
