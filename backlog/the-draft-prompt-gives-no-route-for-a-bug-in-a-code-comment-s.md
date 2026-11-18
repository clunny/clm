---
title: "The draft prompt gives no route for a bug in a code comment's wording"
kind: bug
components: [orchestrator.triage]
---

Step: drafting `20261118-clm-coverage-doc-still`, for `backlog/clm-coverage-doc-still-says-clm-check-re-exports-it` (a crate doc comment in `crates/clm-coverage/src/lib.rs` that contradicts the Contract).

What happens: the draft prompt calls a bug code-only only when a requirement or scenario as written states the behaviour its fix restores and the fix's test cites it (`code-fix-needs-a-written-rule`). A doc comment's wording restores no behaviour a test can observe, so no test can cite anything, yet adding a requirement about a comment's text would be absurd (`yagni`). The decision's exceptions cover test plumbing, the capability harness and the fixed text of prompts and skills, not a code comment. The session had to work out the route itself; it took the `prompt-text-is-source` route by analogy (`clm claim <component> --fix`, no test, `Resolves:` trailer), since the Contract already states what the comment should say.

The fix: name code comments and doc comments that contradict a Contract or Design beside the prompt-text exception in `code-fix-needs-a-written-rule` and in the draft prompt's text (`crates/clm-triage/src/draft.rs`): fixed by component, with no test.
