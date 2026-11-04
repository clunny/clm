---
title: "In TypeScript and Python, clm unreferenced lists what a package's entry modules export, not every export"
date: 2026-10-28
status: accepted
---

`public-item-detector` left Python and TypeScript to a later grammar in the same crate (`tree-sitter-analysis`). The Rust rule doesn't carry over as it stands: Rust has `pub(crate)`, so a `pub` item is a claim of outside use; TypeScript and Python have no package-private visibility, so an `export`, or a Python name without a leading `_`, is also how a package's own modules share code, and "nothing outside the package mentions it" would report every such helper.

Each language gets its own answer to which names are the package's API, and the rest of the rule stays the Rust one (a name match, the package's own tests count as outside, exceptions are `--allow` entries):

- **The API is what the entry modules expose.** A TypeScript package's entry modules are the files its `package.json` points at (`exports`, `main`, `module`, `browser`, `types`), mapped from a built path to the tracked source when needed, else `src/index.ts` or `index.ts`; its API is the names they export, re-exports followed within the package. A Python package is a directory with `__init__.py` whose parent has none, its entry module is that `__init__.py`, and its API is `__all__` when assigned, else the top-level names without a leading `_`, imports from the package's own modules followed. An exported helper that no entry module exposes isn't listed: it is the package's internal sharing, which these languages have no other way to spell.
- **A package is the unit, not a module.** A TypeScript package is a `package.json` with a `name`; a Python package is the top-level import package, not the distribution, since the import package is what outside code names. The package's test files (`*.test.ts`, `__tests__/`, `test_*.py`, `tests/`) are outside it, as Rust's `tests/` are: a name only they use is the package's tested API.
- **Mentions are read from files the grammar can't parse too.** A name used only from a `.vue`, `.svelte`, `.astro`, `.html` or `.mdx` file, or a notebook, would be reported though it is live, which breaks the detector's one promise; those files' whole text counts, comments included, since a loose match only misses dead code. A Python entry point in `pyproject.toml` (`acme = "acme_core.cli:main"`) counts as a mention the same way.
- **A default export isn't listed.** Its importers name it as they like, so no name match can find them.
- **One naming for every language.** `<package>::<name>` with the package's import name (`@acme/core::parse`, `acme_core::parse`), so `--allow` entries read alike and `allow-list`'s rules apply unchanged.

Rejected:
- **Listing every `export` and every non-underscore name**, with the `--allow` list taking the strain. It reports the package's internal sharing wholesale, and a list that long excuses nothing.
- **Running `knip` or `vulture` per package and merging.** Each sees one package, which is the gap; and `dead-code-detectors` already names them for the per-package case.
- **Resolving `tsconfig` paths, bundler aliases or Python `sys.path`.** A name match needs none of it.
- **CommonJS `module.exports` and `export =`.** Their shapes are open-ended; a package that uses them is skipped unless its `package.json` also points at an ES module, and a backlog item can add them.
