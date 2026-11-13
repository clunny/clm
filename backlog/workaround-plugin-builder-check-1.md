---
title: "Skill text the requirement states lies in another component's file"
kind: workaround
components: [plugin]
requirement: plugin/builder-check@c0b39b63
---

plugin/builder-check requires wording in skills/propose/SKILL.md, a file plugin.propose owns, and no requirement of plugin.propose carries it. The build leaves it unbuilt rather than edit that file (path-ownership); see backlog/builder-check-text-lives-in-the-propose-skill-file.
