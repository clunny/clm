---
title: "keys::fingerprint's doc comment says it is clm fingerprint without the top-level-root limit"
kind: bug
components: [web.worker.platform]
---

The doc comment on `clm_web_platform::keys::fingerprint` (crates/clm-web-platform/src/keys.rs, above `pub fn fingerprint`) calls it "`clm fingerprint` of the changes from `base` to `head`" with no limit. Since change 20261119-the-fingerprint-covers-only, `clm fingerprint` covers only the CLM root's subdirectory, so the two are equal only when the CLM root is the repository's top (`web.worker.review/fingerprint-from-trees`). The comment contradicts the Design; fix it by component (`web.worker.platform`), with no test. Found by the review of impl/web.worker.review, which fixed the same wording in its own `Trees::fingerprint`.
