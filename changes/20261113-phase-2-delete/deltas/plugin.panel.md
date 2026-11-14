---
component: plugin.panel
---

## MODIFIED Requirements

### Requirement: inspector-detail <!-- base: 7b641ee8eb630082 -->
While a component is selected (`component-inspector`), the inspector SHALL hold, after the work in flight and from the `spec` model's component (`view/component-detail`): under `changes` one line per entry of its `changes` with the change's ID, status and `<current>/<total> requirements`, and `complete` when the entry says so, or `changes none`; under `backlog` one line per item of its `backlog` with `backlog/<slug>`, its kind and its title, or `backlog none`; and a `Contract` button, `secondary` while the contract is hidden and `primary` while it shows, which SHALL show or hide the component's `contract` text below it as plain lines no wider than the pane's columns, broken at spaces. The contract SHALL be hidden at first, and whether it shows SHALL hold across components and a reload.

#### Serves
- capability/see-project-state

#### Needs
- view/component-detail

#### Scenario: changes and backlog
- GIVEN a terminal pane 80 columns wide on the Spec tab with `a` selected, whose `clm view spec --format json` gives `a` the `changes` entry `20260101-c` with status `applied`, current 1, total 2 and complete false, and the `backlog` item `q` of kind `question` titled `Ask`
- WHEN the pane draws
- THEN the inspector holds, after the work in flight, a line holding `20260101-c`, `applied` and `1/2`, a line holding `backlog/q`, `question` and `Ask`, and a `secondary` `Contract` button, and none of `a`'s `contract` text

#### Scenario: nothing touches it
- GIVEN `b` selected, whose `changes` and `backlog` in the `spec` model are empty
- WHEN the pane draws
- THEN the inspector holds `changes none` and `backlog none`

#### Scenario: contract shown
- GIVEN `a` selected, its `contract` being a paragraph of 300 characters, and the pane 80 columns wide
- WHEN the person presses `Contract`, and in a second run the module is reloaded after that press
- THEN both times the button is `primary` and the paragraph follows it on lines no wider than 80 columns, broken at spaces

#### Scenario: contract hidden again
- GIVEN the contract shown
- WHEN the person presses `Contract` again, and then presses `b`'s line
- THEN after the first press the button is `secondary` and no contract text is drawn, and `b`'s inspector draws its button `secondary` too

#### Scenario: spec model fails on the Flight tab
- GIVEN `a` selected on the Flight tab, and a `clm view spec --format json` that exits 1 with `boom` on stderr
- WHEN the pane draws
- THEN the inspector holds `boom` and no `changes`, `backlog` or `Contract` button
