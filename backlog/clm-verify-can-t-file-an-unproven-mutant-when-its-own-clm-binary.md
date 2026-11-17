---
title: "clm verify can't file an unproven mutant when its own clm binary is replaced mid-run"
kind: bug
components: [verify]
---

Step: `clm verify --due`, run by the build loop as a background `run` action, while other loop runs landed on main.

What happened: the mutation pass found an unproven mutant but did not file it. Its last line was `unproven: not filed: can't copy clm (<clm-dev>/bin/clm (deleted)) to <tmp>/land-bin-XXXX/clm (deleted)`. Landings during the run moved main, the dev install rebuilt the clm-dev binary in place, and verify then tried to copy its own executable by a path that no longer exists. The same run also reported `mutate.sh failed with status 1`, with 39 mutants not judged, which may share this cause.

Expected: a long verify run survives its own binary being replaced. For example, it copies the binary once at start (or reads /proc/self/exe rather than the path it resolves to), so that it still files what it found.

Rule: the build loop runs verify alongside landings (`build-loop-in-clm`), and `dogfood-from-main` makes the dev binary change whenever main moves.
