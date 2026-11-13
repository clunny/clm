---
component: view
base: {}
---

## MODIFIED Requirements

### Requirement: queue-view <!-- base: b97e55e3dbf4c397 -->
`clm-view` SHALL offer the `queue` model, a plain type with the sections `running`, `next`, `waiting_on_you` and `waiting_on_work`, each a list of entries carrying a name, a kind, and the fields its section shows: for a running entry its model, round, how long it has run and whether it is held by a session taken over; for a next entry its priority and the reason for its place; for an entry waiting on the human what it is, since when, and what the human does about it; for an entry waiting on work what it waits on. Its `text` and `ansi` charts SHALL begin with the checkout line, then a line `loop held, renewed <n>m ago`, `loop not held` or `stop requested: drain|now`, then each section under its heading (`running`, `next up`, `waiting on you`, `waiting on work`) with `  none` under an empty one, one entry per line starting with two spaces and its name, in the model's order; in `ansi`, each entry line under `waiting on you` SHALL be amber from its name to its end, so what waits on the human catches the eye (`colour-meanings`), while the heading stays a heading and `  none` stays plain; `json` SHALL be the model with `"view": "queue"`.

#### Scenario: chart sections
- GIVEN a queue model with running `build:a` on `sonnet` for 12 minutes, next `build:b` at `P2` waiting for a place, a fold batch `x` waiting on the human, and nothing waiting on work
- WHEN it is rendered as `text`
- THEN the lines after the checkout line are the loop line, `running`, `  build:a` with `sonnet` and `12m`, `next up`, `  build:b` with `P2` and `waits for a place`, `waiting on you`, `  fold:x` with `say "approve x"`, `waiting on work` and `  none`

#### Scenario: waiting on you highlighted
- GIVEN the same queue model
- WHEN it is rendered as `ansi`
- THEN every character of the line holding `fold:x`, from `fold:x` to its end, lies between `\x1b[33m` and `\x1b[0m`, the lines holding `build:a` and `build:b` hold no `\x1b[33m` before their names, and with the escapes removed the lines are those the `text` chart gives

#### Scenario: wasm
- WHEN `cargo build --target wasm32-unknown-unknown --no-default-features -p clm-view` runs
- THEN it builds

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec
