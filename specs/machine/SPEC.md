---
title: Machine load
status: active
owners: ["@clunny"]
depends_on: [orchestrator, vcs]
paths: ["crates/clm-machine/**"]
successor: ""
serves: [capability/watch-machine-load]
---

## Contract
`clm machine [--format text|json|ansi] [--width <columns>]` reads the load of the machine `clm` runs on and judges whether it bears the builds this clone runs (`machine-load-measured-by-clm`): CPU load, available memory, and free disk on the filesystem holding the clone's git common directory, weighed against what the clone's builds have been measured to use. Each reading has a state, `ok`, `warn`, `critical` or `unknown`. Each run also keeps a sample in a clone-local history of the last 30 minutes, and `--format ansi --width <columns>` draws that history as three chart rows, which the CLM pane shows as they are (`plugin.panel/machine-charts`, `views-rendered-by-clm`). `--format json` gives, among the fields `machine-report` lists, `disk.state` and `disk.room`, how many more builds free space covers once the counting builds have grown to the largest build measured (`null` before any measurement), which the build loop reads to hold back launches (`orchestrator.loop/plan-order`); a field may be added, never renamed or removed without a spec change. It decides nothing about builds.

Crate `clm-machine` offers:
- `read(root, max_builds, expiry, now)`, which takes the readings, measures the builds when due, keeps the sample and returns a snapshot: the report and the history. `expiry` is the claim expiry in hours, by which the places that count are found (`orchestrator`), and `now` the time in Unix seconds that it dates the sample with and judges the record's age by, so a caller passes the time. It never fails.
- `judge(readings, record, counting, max_builds)`, the pure function from the readings, the record, the names of the builds that count and `max_builds` to the report: the three states, with the disk's `need`, `room`, `free_places` and builds.
- `text(report)`, `json(report)` and `rows(report, history, width)`, which give what the command prints in each form.

Root `clm`'s `machine-command` holds the command: it parses the arguments, among them `--expiry <hours>` and `--root <path>`, passes `config`'s `max_builds`, the expiry and the time, writes what these calls give, and decides the exit status.

## Design
Crate `clm-machine`, at `crates/clm-machine/`. It reads load and memory through a maintained crate rather than per-platform code of its own (`reuse-first`): `sysinfo` with its default features off and only the `system` feature, which builds for every target the release builds with zig (`cross-built-macos-binaries`). Free disk is `statvfs`'s `f_bavail × f_frsize`, the space an unprivileged process can still write, and total `f_blocks × f_frsize`.

**CPU** is the 1- and 5-minute load averages over the logical cores (`machine-cpu-flags-the-minute`). The load average is the operating system's own smoothing, with no sampling of its own. A parallel build fills every core, so a ratio near 1 is a machine at work, and beyond it work queues for the cores. The 5-minute ratio is judged first because queueing that has held for minutes is worse than queueing that has just begun and may pass.

**Memory** is the memory the system can still hand out, not the free memory: Linux's `MemAvailable`, which counts reclaimable cache, and on macOS the free, inactive and purgeable pages. Free memory alone reads near zero whenever the page cache is full, while allocations fail only when available memory runs out.

**Builds' disk** (`build-measure`). A build's need is how much disk its worktree takes: its checkout plus the build output it keeps, which for this repository is 6 to 9 GB and for another toolchain may be little, so it is measured, never assumed. The builds are the places that count (`orchestrator`'s `places`), drafts included, since a draft holds a place of `max_builds` and may build code in its worktree; each is matched to the first worktree `clm-git`'s `worktrees` lists on one of the branches its file names. A worktree is measured wherever it lies, while free space is that of the git common directory's filesystem, which holds the main checkout, under which builds' worktrees are made (`orchestrator.run`). A worktree's size is `st_blocks × 512` on Unix, walking without following symbolic links and crossing into no other filesystem. Walking several GB takes seconds, which is why a measurement is reused. The record `<git common dir>/clm/machine` is local state of the clone beside `builds/` and never committed (`build-places`); it holds `measured <RFC 3339 time>`, `peak <bytes>` and one `build <name> <bytes>` line per build measured that time, and is written through a temporary file and a rename. The clone's shared land build directory isn't measured: it is one, whatever the number of builds.

**Disk** judges free space against that need (`machine-report`, `disk-room`). Free space below the growth is `critical` since a running build may then fail, and below the growth plus the starts `warn` since builds `clm next` may still start won't all fit. Under WSL the filesystem is a virtual disk whose free space can exceed the Windows host's, so the reading can be `ok` while the host is full; nothing inside the system can see the host's disk.

**History** (`machine-history`, `machine-cells-keep-their-state`). The samples are in `<git common dir>/clm/machine-samples`, beside the record, one line each. The 5-second gap is there so two panes sampling the one clone don't double the history. A sample keeps its state because the state can't be judged again from the share: the disk's depends on the `peak` and the free places of that moment, and the CPU's on the 5-minute load. A line of another shape is at most 30 minutes old, so dropping it costs the chart at most one half-hour, once, where reading it would paint a cell in a state nobody judged. The file is written through a temporary file and a rename; two runs at once may lose one of their samples. Sampling rides on the pane's runs: nothing samples while nothing runs `clm machine`, and the chart shows that gap only as fewer cells.

**Rows.** The current reading's state is written as a word so it reads without colour (`colour-meanings`); a past cell's state is shown by its colour alone, an exception `machine-cells-keep-their-state` records. The pane reads the escapes run by run (`plugin.panel/machine-charts`), which is why each run is reset on its own.

## Requirements

### Requirement: machine-report
`clm machine` SHALL print the CPU, memory and disk readings and their states, as text by default, as JSON with `--format json` and as chart rows with `--format ansi`, and exit 0 whatever the states. `read` SHALL return those readings and states as a report, and `text`, `json` and `rows` SHALL give for a report what the command prints in each form. `read` SHALL return a report whatever the system refuses, whatever the record and the history files hold, and whether or not they can be read or written. Outside a git repository it SHALL judge the disk of `root`'s filesystem, with no builds, and keep no record and no history file.

The JSON SHALL be `{"view": "machine", "cpu": {...}, "memory": {...}, "disk": {...}}`: `cpu` with `cores`, `load1`, `load5` (both `null` when unknown) and `state`; `memory` with `total`, `available` (both `null` when the system gave no memory reading) and `state`; `disk` with `path` (the clone's common git directory, whose filesystem is read, or `root` outside a git repository), `total`, `available` (both `null` when the system gave no disk reading), `peak` (`null` before any measurement), `measured` (an RFC 3339 UTC time, `<yyyy>-<mm>-<dd>T<hh>:<mm>:<ss>Z`, or `null`), `builds` (each counting build's `name` and last measured `bytes`, or `null`), `free_places`, `need` (`null` with no `peak` or no disk reading), `room` (`disk-room`) and `state`, each state its word and all sizes in bytes. The text SHALL be three lines: `cpu  <load1 ratio>× of <cores> cores, <load5 ratio>× over 5 min  <state>`, each ratio the load average over the cores to one decimal, followed by `×`, or `-` alone in place of a ratio whose load average is unknown, each of the two by itself; `mem  <available> available of <total>  <state>`, or `mem  -  <state>` when the system gave no memory reading; and `disk <available> free of <total>, <need> needed by <n> builds  <state>`, `<n>` the number of counting builds and `, <need> needed by <n> builds` left out when `need` is `null`, or `disk -  <state>` when the system gave no disk reading. A size in the text and the rows SHALL be written as a number and a unit, `B`, `K`, `M`, `G` or `T`: the bytes divided by 1024 once for each unit after `B`, stopping at the first unit where the number, before it is rounded, is under 1000, or at `T`; then rounded to one decimal when the unit isn't `B` and the number before rounding is under 9.95, and to none otherwise, as in `512B`, `850M`, `12G` and `1.3T`, so 1,023,999 bytes read `1000K`.

CPU SHALL be judged on the load averages over the logical cores: `critical` when the 5-minute one is at least 1.25, else `warn` when the 1-minute one is at least 1.25, else `ok`. Memory SHALL be judged on the available memory, never the free memory: `critical` below 10% of total and `warn` below 20%. A memory total of 0 from the system SHALL be no memory reading. Disk SHALL be judged against the builds' measured `peak` when the record holds one: the *growth* is the sum over the counting builds of `peak` less the build's last measured size, when positive, a build not measured owing the whole `peak`; `free_places` is `max_builds` less the number of counting builds, at least 0; the *need* is the growth plus `peak` times `free_places`; and the state is `critical` when free space is below the growth, else `warn` when it is below the need, else `ok`. With no `peak` it SHALL be `critical` below 5% of total and `warn` below 10%, a disk whose total and free space are both 0 being `critical`. A reading the system doesn't give SHALL be `unknown`, and with no disk reading `need` and `room` are `null` whatever the record holds. `judge(readings, record, counting, max_builds)` SHALL return the same states, `need`, `room` and `free_places` from those four alone.

#### Scenario: the JSON
- GIVEN a CLM repository with no build places
- WHEN `clm machine --format json` runs in it
- THEN it exits 0 and prints JSON with `view` `machine`, `cpu.cores` at least 1, `memory.total` and `disk.total` above 0, `disk.peak` `null` and `disk.free_places` equal to `max_builds`

#### Scenario: sustained CPU
- GIVEN 8 cores, `load1` 10 and `load5` 6
- WHEN the readings are judged
- THEN `cpu` is `warn`; with `load5` 10 it is `critical`; with `load1` 4 and `load5` 10 it is still `critical`; and with `load1` 9.9 and `load5` 9.9 it is `ok`

#### Scenario: available, not free
- GIVEN 16 GiB of memory with 0.5 GiB free and 6 GiB available
- WHEN the readings are judged
- THEN `memory` is `ok`, and with 2.5 GiB available `warn`, and with 1 GiB available `critical`

#### Scenario: disk against the builds
- GIVEN a record with `peak` 8 GiB, `max_builds` 3, two counting builds measured at 5 GiB and 8 GiB, and 10 GiB free
- WHEN the readings are judged
- THEN the growth is 3 GiB, the need 11 GiB and `disk` `warn`; with 2 GiB free it is `critical`, and with 12 GiB free `ok`

#### Scenario: an unmeasured build owes the peak
- GIVEN a record with `peak` 8 GiB, `max_builds` 3, a counting build measured at 5 GiB and one the record doesn't hold, and 7 GiB free
- WHEN the readings are judged
- THEN the need is 19 GiB and `disk` is `critical`

#### Scenario: the disk's state in the JSON
- GIVEN the readings of "disk against the builds" with 2 GiB free
- WHEN the report is printed as JSON
- THEN `disk.state` is `critical`, `disk.need` is 11 GiB in bytes and `disk.builds` holds the two builds with their `bytes`

#### Scenario: disk before any measurement
- GIVEN no record, a 100 GiB filesystem and 8 GiB free
- WHEN the readings are judged
- THEN `disk` is `warn`, `need` is `null`, and with 4 GiB free it is `critical`

#### Scenario: no load average
- GIVEN readings whose load averages the system didn't give
- WHEN they are judged
- THEN `cpu` is `unknown` with `load1` and `load5` `null`, and memory and disk are judged as ever

#### Scenario: no memory or disk reading
- GIVEN readings whose memory and disk the system didn't give
- WHEN they are judged, and the report is printed as text
- THEN `memory` and `disk` are `unknown`, `cpu` is judged as ever, and the text's last two lines are `mem  -  unknown` and `disk -  unknown`

#### Scenario: text
- GIVEN 8 cores with `load1` 12 and `load5` 8, 4 GiB available of 16 GiB, and 20 GiB free of 100 GiB with no record
- WHEN `clm machine`'s text is rendered
- THEN its three lines start `cpu  1.5× of 8 cores, 1.0× over 5 min`, `mem  4.0G available of 16G` and `disk 20G free of 100G`, and end `warn`, `ok` and `ok`

#### Scenario: text names the need
- GIVEN the readings of "text", `max_builds` 3, and a record with `peak` 8 GiB and one counting build measured at 5 GiB
- WHEN `clm machine`'s text is rendered, and again with no record
- THEN the disk line is `disk 20G free of 100G, 19G needed by 1 builds  ok` the first time and `disk 20G free of 100G  ok` the second

#### Scenario: outside a git repository
- GIVEN a directory in no git repository
- WHEN `read` runs with it as `root`
- THEN the report's `disk.path` is that directory, its builds are none and `disk.peak` is `null`, the history it returns is the one sample of this run, and no file is written under the directory

### Requirement: build-measure
`clm machine` SHALL measure the disk each counting build's worktree takes when the record `<git common dir>/clm/machine` is at least 10 minutes old, is dated after `now`, or is absent, and otherwise reuse the record's sizes. A worktree's size SHALL be the space its files take on disk, counting a hard-linked file once and following no symbolic link. A counting build with no worktree on a branch its place names SHALL be unmeasured, its `bytes` `null`. The record SHALL keep in `peak` the largest size any one build has been measured at in this clone, never lowering it; a record that can't be read or parsed SHALL count as none.

#### Scenario: measured and kept
- GIVEN a clone with a counting build place whose worktree holds a 3 MiB file and a symbolic link to a 50 MiB file outside it, and no record
- WHEN `clm machine --format json` runs, and again a minute later after the file is deleted
- THEN both times `disk.peak` is at least 3 MiB and under 50 MiB, the build's `bytes` the same both times, and the record's `measured` time unchanged by the second run

#### Scenario: due again
- GIVEN that record, written 11 minutes ago, and the file deleted
- WHEN `clm machine --format json` runs
- THEN the build's `bytes` is under 3 MiB, `measured` is now, and `peak` is still at least 3 MiB

#### Scenario: a record from the future
- GIVEN that record, dated an hour after `now`
- WHEN `clm machine --format json` runs at `now`
- THEN the builds are measured again and `measured` is `now`

#### Scenario: hard links once
- GIVEN a counting build's worktree holding a 4 MiB file and a hard link to it
- WHEN its size is measured
- THEN it is under 8 MiB

#### Scenario: a sparse file
- GIVEN a counting build's worktree holding a 50 MiB file none of whose blocks is written
- WHEN its size is measured
- THEN it is under 10 MiB

#### Scenario: a place with no worktree
- GIVEN a clone with `max_builds` 3 and one counting build place `ghost` whose branch no worktree is on, and no record
- WHEN `clm machine --format json` runs
- THEN `disk.builds` holds `ghost` with `bytes` `null`, `disk.peak` is `null` and `disk.free_places` is 2

#### Scenario: damaged record
- GIVEN a record `<git common dir>/clm/machine` holding `not a record`
- WHEN `clm machine --format json` runs
- THEN it exits 0 and measures the builds, and the record parses afterwards

### Requirement: machine-history
Each `clm machine` run SHALL append a sample of its readings' shares and states to the clone's history, `<git common dir>/clm/machine-samples`, unless the newest sample is under 5 seconds old, and SHALL drop the samples older than 30 minutes and those dated after `now`, keeping at most 180, the newest. A sample SHALL be one line of seven fields, written with one space between them and read with any run of spaces or tabs between them: its RFC 3339 UTC time, then for CPU, memory and disk in turn the reading's *share* and the word of the state that run judged it at (`ok`, `warn`, `critical` or `unknown`), as in `2026-11-12T10:00:00Z 0.6250 warn 0.7500 ok - unknown`. A share SHALL be written as a number with four decimals, never below 0, or as `-` when the reading gives none, and SHALL be read as one only when it is `-` or one or more digits, optionally followed by a point and one or more digits, so `NaN`, `inf`, `-0.5` and `1e3` are no share. CPU's share SHALL be `load1 / cores` over 2, `-` when `load1` is unknown, and memory's and disk's `1 − available / total`, 0 when available exceeds total and `-` when the system gave no reading or its total is 0. A line of the history that isn't such a sample, one with another state word, with a field that is no share where a share belongs, with more or fewer fields, or written before samples held states (a time and three shares) included, SHALL count as none and be dropped when the file is next written, and neither `read` nor `rows` SHALL fail on it. A history file that can't be read SHALL count as empty.

`rows(report, history, width)` SHALL give the three lines `clm machine --format ansi --width <columns>` prints, `cpu`, `mem` and `disk`: the label padded to four cells, a space, a chart of one cell per sample, newest on the right, as many of the newest as fit the width beside the rest of the line, a space, the latest figure and, for a state other than `ok`, a space and the state's word. A cell SHALL be the `k`th of `▁▂▃▄▅▆▇█`, `k` the sample's share times 8 rounded up, at least 1 and at most 8, and a blank for `-`. The figure SHALL be `<load1 / cores>×` to one decimal for `cpu`, whether or not the 5-minute load average is known, `<available> avail` for `mem` and `<available> free` for `disk`, sizes as `machine-report` writes them, or `-` when the system gave no such reading. Each cell SHALL be coloured by the state its sample recorded, a blank one included, and the label, figure and state word by the latest reading's state: `ok` green (`ESC[32m`), `warn` amber (`ESC[33m`), `critical` rose (`ESC[31m`) and `unknown` dimmed (`ESC[2m`), as `colour-meanings` gives them. Adjacent cells of one state SHALL make one coloured run, and each coloured run SHALL end with a reset (`ESC[0m`). No line SHALL be wider than the width, its escapes not counted: a line whose label, figure and state word alone are wider is cut to the width.

#### Scenario: rows from the history
- GIVEN a history of three samples, the last with CPU `load1` 12 of 8 cores and state `warn`, memory 4 GiB available of 16 GiB `ok`, and disk 20 GiB free of 100 GiB `critical`, the latest readings being those
- WHEN the rows are rendered 40 columns wide
- THEN they are `cpu`, `mem` and `disk`, each with a chart of three blocks, the last of `cpu` `▆`, the figures `1.5×`, `4.0G avail` and `20G free`, `cpu`'s line ending ` warn` in amber, `mem`'s figure in green with no state word, and `disk`'s line ending ` critical` in rose

#### Scenario: cells keep their state
- GIVEN a history of three samples whose CPU states are `critical`, `warn` and `ok`, oldest first, the latest reading's CPU state `ok`
- WHEN the rows are rendered 40 columns wide
- THEN the `cpu` chart's cells are rose, amber and green in that order, and its label and figure green with no state word

#### Scenario: a reset ends each run
- GIVEN a history of three samples whose CPU states are `critical`, `critical` and `ok`, oldest first, the latest reading's CPU state `ok`
- WHEN the rows are rendered 40 columns wide
- THEN the `cpu` line is the label in green and a reset, the first two cells together in rose and a reset, the third cell in green and a reset, and the figure in green and a reset

#### Scenario: a cell's height
- GIVEN a history of eight samples whose CPU shares are 0, 0.01, 0.125, 0.126, 0.5, 0.9, 1.0 and 1.7, oldest first
- WHEN the rows are rendered wide enough for all eight
- THEN the `cpu` chart reads `▁▁▁▂▄███`

#### Scenario: the sample holds the shares
- GIVEN a report with `load1` 12 of 8 cores, 4 GiB available of 16 GiB of memory, and 20 GiB free of a 100 GiB disk
- WHEN its sample is taken
- THEN the sample's shares are 0.75 for CPU, 0.75 for memory and 0.8 for disk

#### Scenario: samples written before states
- GIVEN a history file holding two lines of the earlier form, a time and three shares, taken within the last 30 minutes, then one sample with states
- WHEN `clm machine --format ansi` runs 10 seconds after the newest
- THEN it exits 0, each chart row holds two blocks, and the history file holds two samples and no line of the earlier form

#### Scenario: a wrong state word
- GIVEN a history file holding one line whose state word is `fine` and one line of eight fields
- WHEN `clm machine` runs
- THEN the history holds the one sample of this run

#### Scenario: a share that is no number
- GIVEN a history file holding four lines dated within the last 30 minutes, each a sample but for its CPU share, which is `NaN`, `inf`, `-0.5` and `1e3` in turn
- WHEN `read` runs more than 5 seconds after the newest of them, and the rows are rendered from what it returns
- THEN the history it returns is the one sample of this run, the three rows are rendered, and the history file holds that one sample alone

#### Scenario: a clock stepped back
- GIVEN a history whose samples are dated after `now`
- WHEN `clm machine` runs at `now`
- THEN the history holds the one sample of this run, dated `now`

#### Scenario: the newest fit
- GIVEN a history of 180 samples
- WHEN the rows are rendered 30 columns wide
- THEN no line is wider than 30 cells and each chart ends with the newest sample

#### Scenario: a narrow width
- GIVEN a report and a history of one sample
- WHEN the rows are rendered 10 columns wide
- THEN no line is wider than 10 cells, its escapes not counted

#### Scenario: thirty minutes kept
- GIVEN a history of 180 samples, the oldest taken 30 minutes ago
- WHEN `clm machine --format ansi` runs 10 seconds later
- THEN the history holds 180 samples, the oldest gone and the newest the one just taken

#### Scenario: two runs at once
- GIVEN a history whose newest sample was taken 2 seconds ago
- WHEN `clm machine` runs
- THEN the history holds no more samples than before

#### Scenario: unknown CPU
- GIVEN a history whose newest sample's CPU share is `-` and state `unknown`, the latest reading's CPU unknown
- WHEN the rows are rendered
- THEN the `cpu` chart's last cell is blank, its figure `-`, and its label, figure and the ` unknown` that ends the line dimmed

#### Scenario: damaged history
- GIVEN a history file holding `not a sample`
- WHEN `clm machine --format ansi` runs
- THEN it exits 0 and its chart rows each hold one block

### Requirement: disk-room
`clm machine --format json` SHALL give in `disk.room` how many more builds free space covers beside the counting builds' growth (`machine-report`): free space less the growth, divided by `peak` and rounded down, 0 when free space is below the growth, and `null`, the key still present, when the record holds no `peak` or a `peak` of 0, or the system gave no disk reading.

#### Scenario: room against the builds
- GIVEN a record with `peak` 8 GiB, `max_builds` 3, two counting builds measured at 5 GiB and 8 GiB, and 10 GiB free
- WHEN the readings are judged
- THEN `room` is 0; with 20 GiB free it is 2, with 2 GiB free 0, and with 100 GiB free 12

#### Scenario: no room before any measurement
- GIVEN a CLM repository with no build places and no record
- WHEN `clm machine --format json` runs in it
- THEN it exits 0 and `disk.room` is `null`

#### Serves
- capability/build-from-spec
