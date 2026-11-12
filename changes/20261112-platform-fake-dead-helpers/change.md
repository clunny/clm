---
status: proposed
resolves: [platform-fake-dead-helpers]
---

## Intent

Remove the helpers and fields that nothing uses from `clm-web-platform`'s fake (`write_checkout`, `version_of`, `citing`, and `FakeRepo`'s `compare_status` and `ref_times`), with the `fake` feature's dependencies they alone needed.

## The human's answers

- backlog/platform-fake-dead-helpers: "Draft now (Recommended)" — the helpers write_checkout, version_of, citing and fields compare_status, ref_times were confirmed unused outside crates/clm-web-platform/src/fake.rs on origin/main.
