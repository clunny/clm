---
title: "clm unreferenced counts a type named in a used public item's signature as used"
date: 2026-10-28
status: accepted
---

`check.unreferenced` matches uses by name (`public-item-detector`), so a type that another crate receives from a public function, or reads as a public field, without ever spelling its name was reported: `clm_view::FlightView`, which `flight_view` returns and `clm-web` serialises, or `clm_orchestrator::PlaceFile`, the type of `Place`'s public `file` field that `clm-checkpoint` matches on. None of them can be narrowed: `pub(crate)` on the type makes the function's or field's signature a `private_interfaces` warning. This repository's first run listed some twenty of them, each a kept `--allow` entry (`unreferenced-allow-then-narrow`).

A public item's signature now **carries** the names it spells, the way an exported macro's body and a renamed re-export already do (`unreferenced-carriers`): every exactly-`pub` item of a listed kind carries, under its own name, every whole word of the types in its signature, and an `impl` block carries the words of its header under its type's name. A name is used from outside when it is mentioned outside the library or carried under a name that is, followed to any depth, so `blockers_view` used outside makes `BlockersView` used, whose public field makes `Root` used, whose field makes `Held` used.

- **It stays a name match.** The carried words are the signature's text, read from the syntax tree the detector already walks, matched as loosely as any mention: `Option<PlaceFile>` carries `Option` and `PlaceFile`, a parameter `file: PlaceFile` carries `file` too. Like every mention rule it can only miss dead code, never report a used item.
- **Only what an outsider can reach carries.** A field with restricted visibility carries nothing, nor does a `pub(crate)` function: a type they name is reachable only inside the library, where rustc's lint already covers it.
- **The same rule in every language.** A TypeScript function's parameter and return types, a class's non-private members and an interface's body, a Python function's annotations and a class's bases and body annotations, carry the same way (`unreferenced-package-api`).
- **This repository's kept entries went with the build.** Each kept entry whose reason named `unreferenced-signature-types` became a `skips no finding` finding under the new rule (`unreferenced-allow-then-narrow`), and was deleted in the build's own branch, during the re-founding (`20261029-refounding`): its checks ran the tree's own `clm` (`clean-check-runs-own-clm`), and the installed one was rebuilt at the merge back.

Rejected:
- **Resolving the type of each call and field read.** It needs the compiler's view of every outside crate; a name match needs none of it.
- **Carrying only the return type.** Public field types and parameter types are reached the same way (`clm_web::Listing`, the parameter of `fingerprint`), and a trait's member signatures are what an outside implementor sees.
- **Listing such types as `pub(crate)` with `#[allow(private_interfaces)]`.** It hides a real warning to satisfy the detector, and the type is public API in fact.
