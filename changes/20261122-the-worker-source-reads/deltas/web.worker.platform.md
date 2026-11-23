---
component: web.worker.platform
base: {}
---

## ADDED Requirements

### Requirement: unix-time
`install::unix_time` SHALL give the Unix seconds of the instant an RFC 3339 date-time names, `YYYY-MM-DD`, `T` or `t`, `HH:MM:SS`, an optional fraction of a second, which it drops, and `Z`, `z` or an offset `+HH:MM` or `-HH:MM`, which it counts; and SHALL give none for any other text: a non-digit where a digit goes, a month, day, hour, minute or offset that doesn't exist, no offset, text after the offset, or an instant before 1970.

#### Scenario: times GitHub gives
- GIVEN the texts `2026-10-01T10:00:00Z`, `2026-10-01T12:00:00+02:00`, `2026-10-01t08:30:00.75-01:30` and `2026-10-01T10:00:00z`
- WHEN `install::unix_time` reads each
- THEN each gives the Unix seconds of 10:00:00 UTC on 2026-10-01

#### Scenario: text that isn't such a time
- GIVEN the texts `2026-10-01T12:00:00`, `2026-10-01T12:00:00+02:00x`, `2026-10-01T12:00:00Zx`, `2026-1a-01T12:00:00Z`, `2026-02-30T12:00:00Z`, `2026-10-01T24:00:00Z`, `2026-10-01T12:00:00+2:00`, `2026-10-01T12:00:00.Z`, `2026-10-01` and `1969-12-31T23:59:59Z`
- WHEN `install::unix_time` reads each
- THEN each gives none

#### Serves
- capability/follow-from-browser
