---
title: "web.worker's code size budget is set well above its code since the views moved to replica"
kind: idea
components: [web.worker]
---

`web.worker`'s `size_budget` was raised to 72,000 code tokens while the Worker still computed the views. That code has since moved to `replica`, and `clm size web.worker` now reports about 45,600 of 72,000. Review the budget against what stays (the routes, the view cache, proposals, decide, the webhook's dispatch, the live socket and the Worker adapter) and lower it, or say why it stays. Until then the Size paragraph's reason for the raised budget goes unchecked.
