---
title: "reads renames itself because the model rejects the key"
kind: workaround
components: [rename]
requirement: rename/component-rename@4e038ba9
---

The model does not read a change's renames yet (model/rename-records is unbuilt), so a change.md that records them fails its parse and never loads. clm-rename reads the renames itself, writes change.md without the key while it loads, applies and archives the change, and puts the key back in the archived copy. Until model/rename-records lands, the archived change.md of a rename still fails clm check's model/parse-changes. The hiding goes once the model loads renames.
