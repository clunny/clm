---
component: web
---

## MODIFIED Requirements

### Requirement: says-new-state-words <!-- base: c44f7914395c74c8 -->
No file under `web/src/`, its tests included, SHALL hold the old name `plain-terms-renames` gives for the component status `unbuilt` as a word of its own, nor the old name it gives for the backlog kind `workaround` in the forms `clm/retired-terms` lists it, after `clm` and a space or as `kind: <old name>`, matched as `clm/retired-terms` matches; the latter as a plain English word is free. The app SHALL show a component's status as the `spec` model gives it and map no other word to `unbuilt`: the model writes `unbuilt` and reads the old word only from files (`renamed-words-still-load`), so it never hands the app the old word. The doc comment on the type of a capability's open workaround flags SHALL call one `A workaround flag open on a capability`. A test that needs either old word SHALL build it from pieces, so the word never appears whole in the file.

#### Scenario: an unbuilt component
- GIVEN a snapshot whose `spec` model gives component `b` the status `unbuilt`
- WHEN the inspector opens on `b`
- THEN its header names the status `unbuilt`

#### Scenario: the shipped app
- GIVEN the repository
- WHEN the web app's tests run
- THEN no file under `web/src/` holds the first old name as a word of its own or the second in either listed form, and a line using the second as a plain English word passes

#### Serves
- architecture/plain-terms
