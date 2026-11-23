---
title: "No test signals clm verify while it files what the mutation pass found"
kind: deferred
components: [clm]
---

`clm/verify-command@31b600dd` puts the filing of what the mutation pass found (`verify/files-unproven`) inside the window where `SIGINT` and `SIGTERM` set the stop request rather than end the process. The code does this by structure: `crates/clm/src/main.rs`'s `verify` restores the handlers only once `clm_verify::verify_with` returns. But no test signals during the filing: `crates/clm/tests/verify.rs`'s `stopped_by_hand` signals during the pass, and `a_signal_before_the_pass_ends_it_as_any_process` before it. Code that restored the handlers when the pass ended would pass every test citing the requirement.

Needed: a test in `crates/clm/tests/verify.rs` that holds the filing (for example with a slow `land` or push stand-in), sends `SIGTERM` then, and asserts the filing completes and the exit is 143. Found by the review of wave 3's root build, whose notes asked for no new test.
