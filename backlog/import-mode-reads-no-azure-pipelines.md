---
title: "clm verify's import mode reads no Azure Pipelines runs"
kind: deferred
components: [host]
origin: 20261107-slow-tests-after-land
---

`host/list-runs` lists a pipeline's runs on `main` only on `github`; on `azure` it answers that the host lists no pipeline runs, so an Azure DevOps project can use `[verify]` only in `local` or `pipeline` mode. Add the Azure adapter's `runs` (`az pipelines runs list --branch main --pipeline-ids <id>`, through `clm-git`'s `run`) once an Azure project wants `import`.
