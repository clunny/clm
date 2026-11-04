---
title: "Distribution as a Claude Code plugin + Rust CLI"
date: 2026-10-21
status: accepted
---

The plugin (skills, hooks, subagents) handles workflow. A Rust `clm` binary handles everything deterministic, and hooks and CI call it. Rust was the owner's preference, and the deterministic core doesn't benefit enough from an existing JS/Python tool to justify switching.
