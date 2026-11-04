---
title: "A requirement's hash covers its Serves block; its version, the hash's first half, doesn't"
date: 2026-10-30
status: accepted
---

A requirement's version leaves out its `#### Serves` block, so mapping a rule to what it serves makes no test `behind` (`human-layers`). The hash the version is cut from was also the element identity that delta bases record and `clm sync` compares across trees, so leaving the block out of the hash made an edit to only that block invisible to both: a delta written before it applied over it and reverted the block, and sync's three-way merge took a base text that already held it and read the delta as deleting it.

The hash now has two halves: the first 8 hex digits of the content hash of the block less its `#### Serves` block, which is the version tests cite, then the last 8 hex digits of the content hash of the whole block (`model/content-hash`). A Serves-only edit moves the hash and not the version. A requirement with no `#### Serves` block hashes exactly as before, so no recorded base or marker needs touching, and `clm hashes`' output, the base format and every reader of the hash's first 8 digits stay as they are. Each half is 32 bits, ample for telling one text from its next edit.

Rejected:
- **Accepting it.** Traceability and effort accounting read what rules serve; a silently reverted block mis-attributes them, and the fix is one function.
- **Comparing texts in `apply` when the bases match.** `apply` has no base text to tell an old block from an intended one; only sync's history walk does, so the fix would grow two nodes' code to answer what one hash answers.
- **Two hashes printed separately** (a version and an identity hash). Changes what `clm hashes` prints and what bases record, and every skill and guide page that reads them.
- **Putting the block back in the version.** A Serves edit would make every test of the rule `behind` and rebuild code that didn't move.
