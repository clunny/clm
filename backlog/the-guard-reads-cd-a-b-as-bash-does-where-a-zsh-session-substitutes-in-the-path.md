---
title: "The guard reads `cd a b` as bash does, where a session running zsh substitutes `b` for `a` in the current path"
kind: idea
components: [plugin.hooks.guard.shell]
---

The guard follows a `cd` by bash's rules (`plugin.hooks.guard.shell/guard-checkout`): a `cd` with a second directory is one bash refuses with "too many arguments", so the checkout stays where it was. Some sessions on this machine run zsh, where `cd a b` instead replaces the first `a` in the current directory's path with `b` and moves there. zsh's `cd` also refuses `-e` and aborts on an unmatched glob. The Design of `20261123-wave-5-hooks-by-the-rule` keeps bash's rules on purpose, and every blocking scenario it states still blocks under zsh, but a `cd a b` that zsh follows into a checkout on `main` would push from there while the guard judges the push from where it was.

Whether the guard should read a `cd` by the shell the session actually runs (which the hook's event doesn't name), or treat `cd a b` as a `cd` it can't follow (resetting to the event's `cwd` and keeping it for `push-guard`'s reason), is open. Raised as the shell notes' open question 2 in wave 5.
