---
title: "spec/* approval scenarios in land.approval name clm.toml, which is not a spec path"
kind: question
components: [land, land.approval, gate]
---

`land/spec-branch-holds-spec-paths` refuses a `spec/*` branch that changes `clm.toml`, since `gate/files-outside-spec-paths` does not count `clm.toml` as a spec path. But `land.approval/approval-from-base` scenarios land and judge a `spec/x` whose head changes `clm.toml`'s `approval`, and `humans-review-intent` makes that a branch needing review. The build renamed the fixtures of those tests to `propose/x` to keep them passing. A spec branch should either reword those scenarios to a non-`spec/` name, or `gate/files-outside-spec-paths` should treat a `clm.toml` approval change as a spec path.
