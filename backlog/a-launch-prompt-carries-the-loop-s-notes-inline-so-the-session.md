---
title: "A launch prompt carries the loop's notes inline, so the session must retype tens of kilobytes per launch"
kind: bug
components: [orchestrator.loop]
---

Step: `/clm:build`, carrying out `launch` actions after `clm build start --notes <file>`.

What happened: the loop writes each launch's prompt file as a short header (read this other prompt and follow it) followed by the whole notes file under "The human's notes for this loop". With notes of about 60 KB, every launch file is about 63 KB, and the first step printed seven launches (about 440 KB). The build skill says to start each subagent with "its prompt the named file's text unchanged", so the session would have to read every file into its own context and then retype it as the Agent tool's prompt: roughly twice the file's size in the session's tokens per launch, repeated for every launch of the run (twenty or more in a wave), with a transcription error possible on each copy. The header already tells the run to read another file with the Read tool, so the notes could be reached the same way.

What I did: launched each subagent with a one-line prompt telling it to read its loop prompt file whole and follow it, so it got the file's text unchanged without the session copying it.

Expected: a launch prompt the session can pass on cheaply, for example the notes kept in their own file that the launch file names for the run to read, or the skill allowing the session to pass a pointer to the launch file instead of its text.

Rule involved: the build skill's `launch` action ("its prompt the named file's text unchanged") together with `clm build start --notes`, which the skill says never to add to a launch's prompt by hand.
