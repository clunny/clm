---
description: Show a CLM view (status, queue, spec, flight or blockers); with no name, open the CLM panel
argument-hint: "[status|queue|spec|flight|blockers]"
---
The `clm-panel` plugin answers this command itself, without you. Since you are reading this, the panel either isn't installed or didn't load, so:

- With a name (`$ARGUMENTS`) of `status`, `queue`, `spec`, `flight` or `blockers`, run `clm status` (for `status`) or `clm view $ARGUMENTS` with the Bash tool and show its output unchanged in a code block.
- With no name, say that the CLM panel isn't installed or didn't load: if it isn't installed, `/plugin install clm-panel@clm` installs it; if it is, `claude plugin validate` and `claude --debug` show why it didn't load. Then show `clm view flight` the same way.
- With any other name, list the names above.
