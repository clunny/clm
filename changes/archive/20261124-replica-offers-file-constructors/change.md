---
status: done
---

## Intent

`replica` offers three constructors for a `File`: `File::text(entry, text)`, `File::binary(entry)` and `File::unfetched(entry)` (`replica/files-a-source-gives`). It is the first step of wave 6's fix for `backlog/the-replica-s-snapshot-leaves-out-a-component-s-binary-assets` (phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`): wave 6's branch `20261124-wave-6-worker-by-the-rule` adds a field to `File`, which `clm-web-replica` builds by struct literal (`crates/clm-web-replica/src/source.rs:222`, `src/object.rs:336`). As wave 3 settled, a field on a record other components build by hand goes in steps the session runs before the wave (`expand-contract`): the provider offers a constructor, each user moves onto it on a small code branch, and only then does the wave add the field. A Need from `replica` onto the optional `web.worker.replica` would be refused (`model/requirement-needs`, `contract-step-deferred`), so the steps run before the wave instead.

## Outcome

- `replica`'s Contract names the three constructors, and `files-a-source-gives` states what each holds.
- Nothing is cut. `File`'s fields stay public until the wave adds its field.
- The session builds `replica` under `clm claim replica`, then moves `clm-web-replica`'s two literals onto the constructors on a small code branch.
