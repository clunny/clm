---
title: "verify-env's bisect case can't hold with bad as origin/main's head"
kind: bug
components: [verify]
---

`verify/verify-env`'s scenario "a table that can't be used", case 4, builds a fixture holding a passing record for `g`, a land `l1`, then `bad` (whose `[env]` `config/env-setting` refuses) and a red record for `bad`, and runs `clm verify --bisect <bad>`. As written `bad` is `origin/main`'s head, and `settings-from-main` loads `origin/main:clm.toml` first, which refuses the same table, so the call exits 1 before any probe, as the requirement itself says ("for origin/main's head settings-from-main's load refuses first"). The test (`crates/clm-verify/tests/env_commits.rs::a_bisect_of_a_red_commit_whose_table_cant_be_used`) adds a later land that restores the table. The scenario should say so: "then `bad`, then a land that restores `g`'s `clm.toml`".
