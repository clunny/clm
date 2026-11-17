---
component: config
---

## MODIFIED Requirements



### Requirement: retired-terms-detector <!-- base: afaa1f7a9a7a9763 -->
This repository's `clm.toml` SHALL name, after `unreferenced-public-items`, a detector `retired-terms` whose command is `["sh", "scripts/retired-terms.sh"]`, the root's scan for retired terms run with no path (`clm/retired-terms`), and whose hint says the script needs only `sh` and `grep`. `clm check` then fails, naming each file, line and term the scan prints, on a branch whose text names a retired term, so a spec branch finds it while it is drafted rather than at its land; that is `clm check` running a detector, which only a `clm` process shows, so the root states and proves it (`clm/check-across-components`). With this repository as it stands, the detector SHALL exit 0.

#### Scenario: named here
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `detectors` holds `unused-dependencies`, then `unreferenced-public-items`, then `retired-terms`, whose command is `sh`, `scripts/retired-terms.sh` and whose hint names `sh` and `grep`

#### Serves
- architecture/plain-terms

#### Needs
- clm/retired-terms
- ci/detector-tools

### Requirement: unreferenced-detector <!-- base: f0c53e15a530fe0f -->
This repository's `clm.toml` SHALL name, after `unused-dependencies`, a detector `unreferenced-public-items` whose command is `"clm"`, `"unreferenced"`, then any `"--entry", "<attribute>"` pairs (`check.unreferenced/entry-attributes`) and then only `"--allow", "<entry>"` pairs, each entry naming one item (`<crate>::<name>` or `<crate>::<Type>::<name>`), never a whole library, and whose hint is `scripts/dev-install.sh`. The array SHALL hold the two command words on its first line and one pair per line after them, and every pair SHALL come under a `#` comment line within the array that gives its reason: kept (live by a route the name match can't see, saying which) or pending (naming `backlog/narrow-public-items-<component>`, `<component>` being the item's component ID with `.` replaced by `-`). With those arguments `clm unreferenced` exits 0 on this repository, so every entry excuses a finding and no finding is unexcused; that is another component's command run on this repository, so the root states and proves it (`clm/check-across-components`). Every `backlog/narrow-public-items-<component>` a pending comment names SHALL exist. An entry for an item of `clm-config`, this component's own crate, SHALL be kept, never pending: this component's build narrows or removes the rest of its findings.

#### Scenario: named here
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `detectors` holds `unused-dependencies` and then `unreferenced-public-items`, whose command starts `clm`, `unreferenced`, whose other arguments are pairs, `--entry` and an attribute name and then `--allow` and an entry, with every `--allow` entry holding `::`, and whose hint is `scripts/dev-install.sh`

#### Scenario: every entry has a reason
- GIVEN this repository's `clm.toml`
- WHEN a test reads the `unreferenced-public-items` command's array as text
- THEN its first line holds `"clm", "unreferenced",`, every other non-blank line is a `#` comment, one `"--entry", "<attribute>",` pair or one `"--allow", "<entry>",` pair, no `"--entry"` line comes after an `"--allow"` line, the first pair's line comes after a comment line, and every `backlog/narrow-public-items-<component>` a comment names is a file under `backlog/`, and no pending comment names a `config` item

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- clm/clean-check-runs-own-clm
- ci/detector-tools
- check.unreferenced/public-items
- check.unreferenced/references
- check.unreferenced/report
- check.unreferenced/entry-attributes
