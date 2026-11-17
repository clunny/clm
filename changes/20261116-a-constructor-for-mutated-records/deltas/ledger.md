---
component: ledger
---

## ADDED Requirements

### Requirement: mutated-constructor
`clm-ledger` SHALL offer `Body::mutated(commit: String, since: Option<String>, killed: Vec<String>, unproven: Vec<String>, not_judged: Vec<String>) -> Body`, the types of the variant's fields, returning the `mutated` body with those fields, beside the variant, whose fields stay as they are, so a component that builds a `mutated` record can do so without a struct literal of the variant, and a field the variant gains later breaks no caller (`coupled-changes-sequence`).

#### Scenario: the same record as a literal
- GIVEN a commit, a `since`, one `killed` ID, one `unproven` entry and one `not_judged` ID
- WHEN `Body::mutated` is called with them and the body is appended and folded
- THEN the fold holds a `mutated` record with exactly those fields, as one built by the variant's literal does

#### Serves
- architecture/proven-by-tests
- architecture/git-is-truth

## MODIFIED Requirements

### Requirement: mutated-record <!-- base: 4137cc100eee0004 -->
The fold, `fold` and `fold_records` alike, SHALL parse `mutated` records and hold them by `commit`, the newest by `at` first, skipping with a warning a line whose `commit` or `since` holds anything but a full 40-hex lowercase commit ID (or null for `since`), whose `killed` or `not_judged` isn't a list of requirement IDs, or whose `unproven` isn't a list of `<component>/<slug>@<version>` entries with 8 lower-case hex digits, or whose `left` is present and isn't a list of requirement IDs; a record with no `left` holds it empty, so records written before the field read as they did. `Body::mutated` (`mutated-constructor`) SHALL set `left` empty, and `Body::with_left(self, left: Vec<String>) -> Body` SHALL return a `mutated` body with that list as its `left`, and any other body unchanged, so a caller that built its body through the constructor sets `left` without naming the variant's fields. `append` SHALL write it like any record, unsigned in every repository, since it holds no land. `clm ledger show` SHALL print it as `mutated <commit> <k> killed <u> unproven <n> not judged <writer> <at>`, with ` <l> left` after `not judged` when `left` isn't empty. `clm ledger compact` SHALL keep the caller's newest `mutated` record for each commit and every one that holds an `unproven` entry, and drop the others.

#### Scenario: folded and shown
- GIVEN a `mutated` record for commit `c` with one `killed`, one `unproven` and no `not_judged`
- WHEN the fold reads it and `clm ledger show` runs
- THEN the fold holds it under `c`, and the line is `mutated <c> 1 killed 1 unproven 0 not judged <writer> <at>`

#### Scenario: bad entry
- GIVEN a `mutated` line whose `unproven` holds `a/x` with no version
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: what a pass left
- GIVEN a `mutated` record for commit `c` with one `killed` and `left` holding `a/y` and `a/z`
- WHEN the fold reads it and `clm ledger show` runs
- THEN the fold holds `a/y` and `a/z` as its `left`, and the line is `mutated <c> 1 killed 0 unproven 0 not judged 2 left <writer> <at>`

#### Scenario: an older record
- GIVEN a `mutated` line with no `left` key
- WHEN the fold reads it
- THEN it holds the record with an empty `left`

#### Scenario: a bad left
- GIVEN a `mutated` line whose `left` is the string `a/y`
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: left set through the constructor
- GIVEN `Body::mutated` called with a commit and one `killed` ID, and `Body::with_left` called on what it returns with `a/y`
- WHEN the body is appended and the fold reads it
- THEN the fold holds the record with `a/y` as its `left`, and a body from `Body::mutated` alone holds an empty `left`

#### Serves
- architecture/proven-by-tests
- architecture/git-is-truth

#### Needs
- verify/mutated-through-constructor
