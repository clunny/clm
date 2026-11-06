---
title: Sample
status: active
owners: []
depends_on: []
paths: ["tests/sample.rs"]
successor: ""
---

## Contract

Greets the person who runs it.

## Design

One function, tested once.

## Requirements

### Requirement: greets
The sample SHALL greet the person by name.

#### Scenario: a name
- GIVEN the name `Ada`
- WHEN the sample greets
- THEN it says `Hello, Ada`
