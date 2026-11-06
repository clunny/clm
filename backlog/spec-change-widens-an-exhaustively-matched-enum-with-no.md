---
title: "Spec change widens an exhaustively matched enum with no buildable order"
kind: bug
components: [config, host]
---

Change 20261106-azure-devops-host adds the value azure to config/host-setting, widening clm_config::Host, which clm-host matches exhaustively. forced-fixes-cover-additions forbids the arm, and host requirements Need config/host-setting, so neither side can build first. The drafting check did not catch a widened exhaustive enum. See backlog/azure-host-variant-breaks-clm-host.
