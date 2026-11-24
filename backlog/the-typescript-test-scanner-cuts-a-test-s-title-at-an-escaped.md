---
title: "The TypeScript test scanner cuts a test's title at an escaped quote"
kind: bug
components: [check.coverage]
---

The TypeScript test scanner in check.coverage names a test by its title up to the first quote character, and doesn't read a backslash-escaped quote inside a single-quoted title. A vitest test titled 'the declarations are the panel\'s own' is named `the declarations are the panel\` in coverage's lists, and so is any cited test whose title holds an escaped apostrophe (one in plugin-panel's agents tests).

Met during wave 5's design: a deleted-tests.txt line written with the full title is refused at the gate as naming no test with a marker, and must be written with the truncated name. clm check doesn't read the list's lines, so it passes either way.

Expected: a test's name is its whole title as the language reads it, escapes included. Fixing it renames every test whose title holds an escaped quote, so it lands after any branch whose deleted-tests.txt names such a test by its truncated name (wave 5's plugin branch), and its build re-reads those lists.
