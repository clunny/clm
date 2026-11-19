---
title: "The propose skill doesn't say to list a removed requirement's tests in other components' files"
kind: deferred
components: [plugin]
origin: 20261119-a-removed-requirement-s-tests-elsewhere
---

`check/obsolete-tests-elsewhere` fails `clm check` for a test in a file another component owns that cites a requirement a change removed and that no `deleted-tests.txt` names. `plugin/skills/propose/SKILL.md` tells a designing session how to write `deleted-tests.txt` and doesn't say this: when a delta removes a requirement, search for tests citing it from files of other components (`clm coverage --obsolete-tests` lists them with their files), list each, and where the behaviour stays stated, have the stating requirement's own build prove it. The check's message says as much when it fails, so the skill's sentence saves a failed check, not a fault. Skill text only (`prompt-text-is-source`): a code fix under `clm claim plugin --fix backlog/the-propose-skill-doesn-t-say-to-list-a-removed-requirement-s-tests-elsewhere`, with no test.
