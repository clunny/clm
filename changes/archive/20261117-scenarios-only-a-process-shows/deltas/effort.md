---
component: effort
---

## MODIFIED Requirements



### Requirement: capabilities-list <!-- base: daa12c28754317f9 -->
`clm capabilities` SHALL print `capabilities <n> · <k> ⚑ · costs are estimates: a component's cost is split by its rules' share`, with `k` the flagged ones; then one row per capability in the report's order, `⚑ ` or two spaces, the slug padded to the longest, and `  ≈$<d>  <tokens> tok  <c> commits  <a> reviews  <w> workarounds  ` then its latest decision, `<decision> <value> <YYYY-MM-DD>`, or `never decided`; then `unattributed` and `architecture only`, each padded to the width of the rows' `⚑ <slug>` or of `architecture only`, whichever is wider, followed by `  ≈$<d>  <tokens> tok`. Tokens are work tokens and SHALL read as `850`, `85k` or `1.3M`, and dollars to the cent under $10 and to the tenth above, as the panel formats them; `<w>` counts unsettled flags; and a number only history gives SHALL read `-` without it. In a directory that holds no `specs/SPEC.md`, `clm capabilities` SHALL print nothing on standard output and exit 1 with a line on standard error that starts `not a CLM repository`.

#### Scenario: the list
- GIVEN a report of two capabilities, one flagged, without history
- WHEN `list_text` runs
- THEN its first line is `capabilities 2 · 1 ⚑ · costs are estimates: a component's cost is split by its rules' share`, the flagged row starts with `⚑ ` and shows `- commits`, and the last two lines are `unattributed` and `architecture only`

#### Scenario: outside a CLM repository
- GIVEN a directory with a `CAPABILITIES.md` and no `specs/SPEC.md`
- WHEN `clm capabilities` runs there
- THEN it exits 1, and its standard error holds `not a CLM repository`
