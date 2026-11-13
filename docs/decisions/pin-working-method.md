---
title: "The pinned working method is an architectural constraint, not a capability"
date: 2026-11-13
status: accepted
---

`pin-working-method` was a capability: "a person can rely on every build following the same fixed version of the AI's working method". It states what must always hold, not something a person does, so on 2026-11-13 it became an architectural constraint, labelled checked. No behaviour changed.

Its rules are `orchestrator.run/pin-file`, the one line that names the pinned superpowers tag and commit, and `orchestrator.run/superpowers-path`, by which `clm superpowers <skill>` prints the pinned copy's path whatever the session has installed. What stays unchecked is that a build follows the text it reads, which is a model's behaviour.

The capability's own test, `crates/clm/tests/capability/pin_working_method.rs`, went with the entry, since a test never cites an architectural constraint (`human-layer-proof`); the same behaviour is proven by the tests that cite `orchestrator.run/superpowers-path` and `orchestrator.run/pin-file`.
