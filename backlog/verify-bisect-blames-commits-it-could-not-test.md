---
title: "clm verify's bisect counts commits it could not test as red and blames the wrong land"
kind: bug
components: [verify]
issues: [417]
---

When the slow tests fail over several lands, `clm verify` bisects to find the culprit and treats every probe whose check does not pass as a failure. A commit older than the project's slow-test script cannot run the check at all: the command is not found. That commit is still counted red. With no green record the search starts at the first commit on main's first-parent chain, so the outcome can name the repository's first commit as the culprit and, since that diff touches no component's paths, the root component. Triage then takes a culprit and component that cannot be right, and `land/verify-stop` holds the wrong component's lands.

Expected: a probe whose check cannot run on that commit is untestable and is skipped, not counted red. When no commit in the range can be tested, the outcome is `culprit unknown`, with the components of the covered lands, rather than a guess. `verify-bisect` needs to say what counts as untestable and what the record holds then.
