---
status: done
---

## Intent

Make the web app the place a person does their human-only work, on a phone as well as a desk. A repo opens on a **Waiting on you** home page. Every backlog item, capability and architectural constraint opens to its full text. A `question` that lists its possible answers can be answered with a tap, and a `human-task` marked done. Nothing that needs a dialogue with an agent moves into the app. The spec graph stays, made easier to read: a selected node colours its incoming and outgoing edges apart, and its details open in an overlay. Everything from the repo renders as Markdown.

## Problem

- The app holds the human steps that need only a yes (approve, request changes, comment, decide on a capability or a constraint, propose an item), but nothing gathers what waits on the person. On a phone, finding it means visiting five tabs.
- A backlog item shows only its title, kind, components and what it blocks; the snapshot carries no body, so it can't be read anywhere in the app. Capability and constraint pages show the one-line statement as plain text (backticks stay raw) and never the scenarios, `#### Enforced by` or `#### Untested` blocks. The `.md` styles cover paragraphs and lists only, so code, headings and tables in a contract render badly.
- A `question` or `human-task` can only be settled in a `/clm:triage` session, even when the answer is a yes or a no.
- New proposal asks for components, which a session never asks for when it files an item; the draft that takes the item up decides them.
- In the spec graph every edge looks alike, so a node's dependencies and dependents can't be traced, and the inspector is a narrow column too small for a component's contract.

## Approaches

Where the full text comes from:
- **(a) In the data the app already loads and caches (chosen).** The snapshot carries each backlog item's body, priority and choices, and the capabilities and architecture reports each entry's full text. Detail pages draw at once and offline, which matters on the move. Costs a larger snapshot, about 100 KB for this repository's backlog.
- **(b) A detail endpoint per item.** Smaller loads, but an item not opened while online can't be read offline, and each tap waits on the network. Not chosen.

Where the new pages live: in `web.views`, carved out of `web` by `20261112-web-views-split` on this branch, rather than growing `web`, which is over both budgets.

How a person answers: a `question` lists the answers it accepts in `choices`, and only those become buttons; a `human-task` offers **Done**. Free-text answers are out: an answer that needs explaining needs a dialogue, which belongs in a session (`docs/decisions/web-answers-as-ledger-records.md`).

## Outcome

- **Home** (`web.views/home-waiting-on-you`, `web/home-first`): where a repo opens, at `#/<owner>/<repo>`. It lists the spec branches that need review with no approval of their head, the `question` items with `choices` and the `human-task` items, each with its answer buttons and any answer already given, the flagged capabilities and constraints, and the other `question` items, marked to be answered in a session. Sidebar and tab bar start with Home; on phones the tab bar is Home, Backlog, Graph, Build and More (PRs, Capabilities, Arch.).
- **Detail pages**: a backlog item at `#/<owner>/<repo>/backlog/<slug>` with its body, kind, priority, components, origin, what it blocks and what holds it (`web.views/backlog-item-page`); capability and constraint pages show the entry's full text (`web.views/capability-page`, `web.views/architecture-page`). The snapshot carries items' `body`, `priority` and `choices` (`replica/snapshot-backlog-text`); the reports carry each entry's `text` (`effort/entry-text`).
- **Answers**: `choices` on a `question` item (`model/backlog-choices`, `docs/spec-format.md`); an `answered` ledger record (`ledger/answer-records`) appended by `POST .../backlog/{slug}/answer` (`web.worker/answers`), checked by the replica (`replica/validate-answers`) and listed in the implementation response (`replica/open-answers`); `clm triage` prints each under its item (`orchestrator.triage/triage-web-answers`) and the `triage` skill takes it as the human's answer (`plugin.triage/triage-web-answers`).
- **New proposal** has a title and an intent only and posts `/nodes` `[]`, which the Worker already accepts (`web.views/propose-from-backlog`).
- **Graph**: a selected node's edges from its dependents and to its dependencies take two colours of their own and the rest dim (`web.views/edge-direction`); the inspector opens as an overlay (`web.views/inspector-overlay`).
- **Markdown**: every text from the repository the app shows is rendered as sanitised Markdown (`web.views/repo-markdown`), styled for code, headings, tables and quotes without widening the page (`web/markdown-styles`).
- `CAPABILITIES.md`'s `follow-from-browser` says what a person can now do from a phone.

## Constraints

- No agent reasoning in the app: it records a person's yes, choice or done, and a session acts on it.
- `web-optional` holds: every answer can still be given in a `/clm:triage` session.
- An `answered` record is open while its item is in the backlog; answering again replaces that person's answer. An older `clm` skips the kind with a warning, as it does any kind it doesn't know.

## Open questions

- The split's build (`web.views`) moves files out of `web`'s `paths`, rewires `web/src/main.ts` and re-points markers in `web/e2e/`. `path-ownership`'s text lists no such exception, while `nested-splits` sanctions it and every earlier split built this way. Left as established practice; the architecture audit flagged the gap in the constraint's wording.
