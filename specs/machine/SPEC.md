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
`clm machine [--format text|json|ansi] [--width <columns>]` reads the load of the machine `clm` runs on and judges whether it bears the builds this clone runs (`machine-load-measured-by-clm`): CPU load, available memory, and free disk on the filesystem holding the clone's git common directory, weighed against what the clone's builds have been measured to use. Each reading has a state, `ok`, `warn`, `critical` or `unknown`. Each run also keeps a sample in a clone-local history of the last 30 minutes, and `--format ansi --width <columns>` draws that history as three chart rows, which the CLM pane shows as they are (`plugin.panel/machine-charts`, `views-rendered-by-clm`). It decides nothing about builds. Crate `clm-machine` exposes `read(root, max_builds, expiry, now)`, which takes the readings and measures the builds when due, `now` being the time it judges the record's age by, so tests pass one, and `judge`, the pure function from readings to states; root `clm`'s `machine-command` dispatches the command, passing `config`'s `max_builds`.

## Design
Crate `clm-machine`, at `crates/clm-machine/`. It reads the machine through a maintained crate rather than per-platform code of its own (`reuse-first`): `sysinfo` with its default features off and only the features the readings need, provided it builds for every target the release builds with zig (`cross-built-macos-binaries`); otherwise it reads `/proc` on Linux and `sysctl` and `host_statistics64` on macOS through `libc`, and this Design says why. Free disk is `statvfs`'s `f_bavail × f_frsize`, the space an unprivileged process can still write, and total `f_blocks × f_frsize`.

**CPU** is the 1- and 5-minute load averages over the logical cores. The load average is the operating system's own smoothing, so a sustained load shows and a spike doesn't, with no sampling of its own. The state follows the 5-minute ratio: `critical` from 2, `warn` from 1.25, else `ok`; a compile keeps it near 1, so only queueing beyond the cores flags. Where the system gives no load average the state is `unknown`.

**Memory** is the memory the system can still hand out, not the free memory: Linux's `MemAvailable`, which counts reclaimable cache, and on macOS the free, inactive and purgeable pages. Free memory alone reads near zero whenever the page cache is full, while allocations fail only when available memory runs out. `critical` below 10% of total, `warn` below 20%.

**Builds' disk** (`build-measure`). A build's need is how much disk its worktree takes: its checkout plus the build output it keeps, which for this repository is 6 to 9 GB and for another toolchain may be little, so it is measured, never assumed. The builds are the places that count (`orchestrator`'s `places`), drafts included, since a draft holds a place of `max_builds` and may build code in its worktree; each is matched to the first worktree `clm-git`'s `worktrees` lists on one of the branches its file names. A place with no such worktree, or whose file can't be read, is unmeasured, its `bytes` `null`. A worktree is measured wherever it lies, while free space is that of the git common directory's filesystem, which holds the main checkout, under which builds' worktrees are made (`orchestrator.run`). A worktree's size is the space its files take on disk (`st_blocks × 512` on Unix), walking without following symbolic links, counting a hard-linked file once and crossing into no other filesystem. Walking several GB takes seconds, so it runs at most once every 10 minutes: the record `<git common dir>/clm/machine`, local state of the clone beside `builds/` and never committed (`build-places`), holds `measured <RFC 3339 time>`, `peak <bytes>`, the largest size any one build's worktree has had in this clone, the record's earlier `peak` included, so it only ever rises, and one `build <name> <bytes>` line per build measured that time. A record that can't be read or parsed counts as none and is rewritten; it is written through a temporary file and a rename, so two runs at once never leave a torn one. The clone's shared land build directory isn't measured: it is one, whatever the number of builds.

**Disk** judges free space against that need. With a `peak`, the *growth* still owed is the sum over the counting builds of `peak` minus its last measured size, when positive (a build not yet measured owes `peak`), and the *starts* are `peak` times the free places, `max_builds` minus the places that count, at least 0. `critical` when free space is below the growth, since a running build may then fail; `warn` when it is below the growth plus the starts, since builds `clm next` may still start won't all fit; else `ok`. With no `peak` yet, `critical` below 5% of total and `warn` below 10%. Under WSL the filesystem is a virtual disk whose free space can exceed the Windows host's, so the reading can be `ok` while the host is full; nothing inside the system can see the host's disk.

**History** (`machine-history`). Each run appends a sample to `<git common dir>/clm/machine-samples`, beside the record, unless the newest sample there is under 5 seconds old, so two panes sampling the one clone don't double it, and drops the samples older than 30 minutes, keeping at most 180. A sample is one line: its RFC 3339 time, then the CPU, memory and disk *shares*, each a decimal or `-` when its reading is unknown: CPU `load1 / cores` over 2, so a share of 1 is twice the cores; memory `1 − available / total`; disk `1 − available / total`, so for all three a larger share is a heavier load. The file is written through a temporary file and a rename; two runs at once may lose one of their samples, never tear the file, and a file that can't be read counts as empty. Sampling rides on the pane's runs: nothing samples while nothing runs `clm machine`, and the chart shows that gap only as fewer cells.

**Rows.** `--format ansi` prints three lines, `cpu`, `mem` and `disk`: the label padded to four cells, a space, the chart, a space, the figure, and, for a state other than `ok`, a space and the state's word, so the row reads without colour (`colour-meanings`). The chart is one cell per sample, newest on the right, as many of the newest as fit `--width` (default 80) beside the rest of the row; a cell is the `k`th of `▁▂▃▄▅▆▇█`, `k` its share times 8 rounded up, at least 1 and at most 8, and a blank for `-`. The figure is the latest reading's: `<load1 / cores>×` to one decimal, or `-` when unknown; `<available> avail`; `<available> free`. Each line is coloured by its state, as `colour-meanings` gives them: `ok` green, `warn` amber, `critical` rose, `unknown` dimmed. Sizes, here and in the text, are powers of 1024 written `850M`, `12G` or `1.3T`, one decimal under 10.

**Output.** `--format json` prints `{"view": "machine", "cpu": {...}, "memory": {...}, "disk": {...}}`: `cpu` with `cores`, `load1`, `load5` (both `null` when unknown) and `state`; `memory` with `total`, `available` and `state`; `disk` with `path`, `total`, `available`, `peak` (`null` before any measurement), `measured` (an RFC 3339 time or `null`), `builds` (each counting build's `name` and last measured `bytes`, or `null`), `free_places`, `need` (the growth plus the starts, `null` with no `peak`) and `state`, all sizes in bytes. Fields may be added, never renamed or removed without a spec change. `--format text`, the default, prints three lines: `cpu  <load1 ratio>× of <cores> cores, <load5 ratio>× over 5 min  <state>`, `mem  <available> available of <total>  <state>` and `disk <available> free of <total>, <need> needed by <n> builds  <state>` (`needed` left out with no `peak`), sizes as the rows write them. Outside a git repository the disk is the current directory's filesystem, with no builds. It exits 0 whatever the states; a reading the system refuses is `unknown`, and only a failure to write its output exits 1.

## Requirements

### Requirement: machine-report
`clm machine` SHALL print the CPU, memory and disk readings and their states as the Design's **Output** gives them, as text by default, as JSON with `--format json` and as chart rows with `--format ansi`, and exit 0 whatever the states. CPU SHALL be judged on the 5-minute load average over the logical cores (`critical` from 2, `warn` from 1.25), memory on the available memory, never the free memory (`critical` below 10% of total, `warn` below 20%), and disk as the Design's **Disk** says, against the builds' measured `peak` when the record holds one and against 5% and 10% of total otherwise. A reading the system doesn't give SHALL be `unknown`. The judgement SHALL be a pure function of the readings, the record, the places that count and `max_builds`.

#### Scenario: the JSON
- GIVEN a CLM repository with no build places
- WHEN `clm machine --format json` runs in it
- THEN it exits 0 and prints JSON with `view` `machine`, `cpu.cores` at least 1, `memory.total` and `disk.total` above 0, `disk.peak` `null` and `disk.free_places` equal to `max_builds`

#### Scenario: sustained CPU
- GIVEN 8 cores, `load1` 20 and `load5` 11
- WHEN the readings are judged
- THEN `cpu` is `warn`, and with `load5` 16 it is `critical`, and with `load1` 20 and `load5` 6 it is `ok`

#### Scenario: available, not free
- GIVEN 16 GiB of memory with 0.5 GiB free and 6 GiB available
- WHEN the readings are judged
- THEN `memory` is `ok`, and with 2.5 GiB available `warn`, and with 1 GiB available `critical`

#### Scenario: disk against the builds
- GIVEN a record with `peak` 8 GiB, `max_builds` 3, two counting builds measured at 5 GiB and 8 GiB, and 10 GiB free
- WHEN the readings are judged
- THEN the growth is 3 GiB, the need 11 GiB and `disk` `warn`; with 2 GiB free it is `critical`, and with 12 GiB free `ok`

#### Scenario: disk before any measurement
- GIVEN no record, a 100 GiB filesystem and 8 GiB free
- WHEN the readings are judged
- THEN `disk` is `warn`, `need` is `null`, and with 4 GiB free it is `critical`

#### Scenario: no load average
- GIVEN readings whose load averages the system didn't give
- WHEN they are judged
- THEN `cpu` is `unknown` with `load1` and `load5` `null`, and memory and disk are judged as ever

#### Scenario: text
- GIVEN 8 cores with `load1` 8 and `load5` 11, 4 GiB available of 16 GiB, and 20 GiB free of 100 GiB with no record
- WHEN `clm machine`'s text is rendered
- THEN its three lines start `cpu  1.0× of 8 cores, 1.4× over 5 min`, `mem  4.0G available of 16G` and `disk 20G free of 100G`, and end `warn`, `ok` and `ok`

### Requirement: build-measure
`clm machine` SHALL measure the disk each counting build's worktree takes when the record is older than 10 minutes or there is none, and otherwise reuse the record's sizes. A worktree's size SHALL be the space its files take on disk, counting a hard-linked file once and following no symbolic link. The record SHALL keep in `peak` the largest size any one build has been measured at in this clone, never lowering it, and be written through a temporary file and a rename; a record that can't be read or parsed SHALL count as none.

#### Scenario: measured and kept
- GIVEN a clone with a counting build place whose worktree holds a 3 MiB file and a symbolic link to a 50 MiB file outside it, and no record
- WHEN `clm machine --format json` runs, and again a minute later after the file is deleted
- THEN both times `disk.peak` is at least 3 MiB and under 50 MiB, the build's `bytes` the same both times, and the record's `measured` time unchanged by the second run

#### Scenario: due again
- GIVEN that record, written 11 minutes ago, and the file deleted
- WHEN `clm machine --format json` runs
- THEN the build's `bytes` is under 3 MiB, `measured` is now, and `peak` is still at least 3 MiB

#### Scenario: hard links once
- GIVEN a counting build's worktree holding a 4 MiB file and a hard link to it
- WHEN its size is measured
- THEN it is under 8 MiB

#### Scenario: damaged record
- GIVEN a record `<git common dir>/clm/machine` holding `not a record`
- WHEN `clm machine --format json` runs
- THEN it exits 0 and measures the builds, and the record parses afterwards

### Requirement: machine-history
Each `clm machine` run SHALL append a sample of its readings' shares to the clone's history, unless the newest sample is under 5 seconds old, and SHALL drop the samples older than 30 minutes, keeping at most 180. `--format ansi --width <columns>` SHALL print the rows the Design's **Rows** gives: `cpu`, `mem` and `disk`, each its label, a chart of one block per sample, newest on the right, as many of the newest as fit the width, the latest figure and, for a state other than `ok`, the state's word, each line coloured by its state. No line SHALL be wider than the width.

#### Scenario: rows from the history
- GIVEN a history of three samples, the last with CPU `load1` 8 of 8 cores and state `warn`, memory 4 GiB available of 16 GiB `ok`, and disk 20 GiB free of 100 GiB `critical`
- WHEN the rows are rendered 40 columns wide
- THEN they are `cpu`, `mem` and `disk`, each with a chart of three blocks, the last of `cpu` `▄`, the figures `1.0×`, `4.0G avail` and `20G free`, `cpu`'s line ending ` warn` in amber, `mem`'s in green with no state word, and `disk`'s ending ` critical` in rose

#### Scenario: the newest fit
- GIVEN a history of 180 samples
- WHEN the rows are rendered 30 columns wide
- THEN no line is wider than 30 cells and each chart ends with the newest sample

#### Scenario: thirty minutes kept
- GIVEN a history of 180 samples, the oldest taken 30 minutes ago
- WHEN `clm machine --format ansi` runs 10 seconds later
- THEN the history holds 180 samples, the oldest gone and the newest the one just taken

#### Scenario: two runs at once
- GIVEN a history whose newest sample was taken 2 seconds ago
- WHEN `clm machine` runs
- THEN the history holds no more samples than before

#### Scenario: unknown CPU
- GIVEN a history whose newest sample's CPU share is `-`
- WHEN the rows are rendered
- THEN the `cpu` chart's last cell is blank, its figure `-` and its line dimmed, ending ` unknown`

#### Scenario: damaged history
- GIVEN a history file holding `not a sample`
- WHEN `clm machine --format ansi` runs
- THEN it exits 0 and its chart rows each hold one block
