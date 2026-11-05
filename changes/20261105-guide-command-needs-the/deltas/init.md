---
component: init
---

## MODIFIED Requirements

### Requirement: guide-command <!-- base: 07905d3cdbc09e29 -->
`clm guide` SHALL print one line per page of the guide, as compiled into the binary, each `<name>  <title>`. The pages SHALL be every `.md` file in `docs/guide/`, `docs/spec-format.md` and the generated page `constraints` (`guide-constraints-page`), whose title is `CLM's constraints`; `<name>` is the file name without `.md` (`index` for `README.md`) and `<title>` the page's first `# ` heading; `index` SHALL come first, then the pages `README.md`'s numbered list names, in its order, then `spec-format`, then `constraints`. For a page read from a file, every page but `constraints`, `clm guide <name>` SHALL print its Markdown exactly as its file is at build time, and exit 0; `clm guide constraints` prints the page `guide-constraints-page` states. For a name that isn't a page it SHALL exit 1, printing `no guide page <name>` and the index to stderr. It SHALL read no file at run time.

#### Scenario: index
- GIVEN the binary built from this repository
- WHEN `clm guide` runs in a directory that isn't in a repository
- THEN it exits 0, its first line names `index`, one line is `shaping-the-graph  Shaping the graph`, its second-to-last line is `spec-format  Spec format (v0, provisional)` and its last line is `constraints  CLM's constraints`

#### Scenario: one page
- GIVEN the binary built from this repository
- WHEN `clm guide shaping-the-graph` runs
- THEN it prints `docs/guide/shaping-the-graph.md` byte for byte and exits 0

#### Scenario: the spec format
- GIVEN the binary built from this repository
- WHEN `clm guide spec-format` runs
- THEN it prints `docs/spec-format.md` byte for byte and exits 0

#### Scenario: unknown page
- GIVEN the binary
- WHEN `clm guide no-such-page` runs
- THEN it exits 1 and stderr holds `no guide page no-such-page` and the index

#### Scenario: page left out
- GIVEN a `.md` file in `docs/guide/` that the compiled table doesn't hold
- WHEN the tests run
- THEN a test fails naming the file
