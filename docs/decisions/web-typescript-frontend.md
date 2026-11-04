---
title: "The web front end is TypeScript"
date: 2026-10-22
status: accepted
---

`web/` is a TypeScript single-page app built with Vite. It's an exception to the Rust preference: graph layout (ELK via `elkjs`), word-level diffs (`diff`), PWA tooling and browser testing are mature in JavaScript and immature in Rust UI frameworks (`reuse-first`). Everything that must agree with `clm` (parsing, coverage, the queue) happens in the Rust Worker, so the front end only presents data.

Alternative considered: an all-Rust front end (Leptos or Yew). It would share types with the Worker, but layout and diffing would have to be written or wrapped.
