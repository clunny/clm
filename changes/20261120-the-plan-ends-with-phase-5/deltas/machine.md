---
component: machine
base: { design: "9f6519fa36708f49" }
---

## MODIFIED Design
Crate `clm-machine`, at `crates/clm-machine/`. It reads load and memory through a maintained crate rather than per-platform code of its own (`reuse-first`): `sysinfo` with its default features off and only the `system` feature, which builds for every target the release builds with zig (`cross-built-macos-binaries`). Free disk is `statvfs`'s `f_bavail × f_frsize`, the space an unprivileged process can still write, and total `f_blocks × f_frsize`.

**CPU** is the 1- and 5-minute load averages over the logical cores (`machine-cpu-flags-the-minute`). The load average is the operating system's own smoothing, with no sampling of its own. A parallel build fills every core, so a ratio near 1 is a machine at work, and beyond it work queues for the cores. The 5-minute ratio is judged first because queueing that has held for minutes is worse than queueing that has just begun and may pass.

**Memory** is the memory the system can still hand out, not the free memory: Linux's `MemAvailable`, which counts reclaimable cache, and on macOS the free, inactive and purgeable pages. Free memory alone reads near zero whenever the page cache is full, while allocations fail only when available memory runs out. `sysinfo` reports a total of 0 when it has no memory reading, so `read` takes that as no reading before anything is judged. So `clm machine` prints `mem  -  unknown` for it, and the reading whose total is 0 that `machine-report` judges is one a caller hands `judge`.

**Builds' disk** (`build-measure`). A build's need is how much disk its worktree takes: its checkout plus the build output it keeps, which for this repository is 6 to 9 GB and for another toolchain may be little, so it is measured, never assumed. The builds are the places that count (`orchestrator`'s `places`), drafts included, since a draft holds a place of `max_builds` and may build code in its worktree; each is matched to the first worktree `clm-git`'s `worktrees` lists on one of the branches its file names. A worktree is measured wherever it lies, while free space is that of the git common directory's filesystem, which holds the main checkout, under which builds' worktrees are made (`orchestrator.run`). A worktree's size is `st_blocks × 512` on Unix, walking without following symbolic links and crossing into no other filesystem. Walking several GB takes seconds, which is why a measurement is reused. The record `<git common dir>/clm/machine` is local state of the clone beside `builds/` and never committed (`build-places`); it holds `measured <RFC 3339 time>`, `peak <bytes>` and one `build <name> <bytes>` line per build measured that time, and is written through a temporary file and a rename. The clone's shared land build directory isn't measured: it is one, whatever the number of builds.

**Disk** judges free space against that need (`machine-report`, `disk-room`). Free space below the growth is `critical` since a running build may then fail, and below the growth plus the starts `warn` since builds `clm next` may still start won't all fit. Under WSL the filesystem is a virtual disk whose free space can exceed the Windows host's, so the reading can be `ok` while the host is full; nothing inside the system can see the host's disk.

**History** (`machine-history`, `machine-cells-keep-their-state`). The samples are in `<git common dir>/clm/machine-samples`, beside the record, one line each. The 5-second gap is there so two panes sampling the one clone don't double the history. A sample keeps its state because the state can't be judged again from the share: the disk's depends on the `peak` and the free places of that moment, and the CPU's on the 5-minute load. A line of another shape is at most 30 minutes old, so dropping it costs the chart at most one half-hour, once, where reading it would paint a cell in a state nobody judged. The file is written through a temporary file and a rename; two runs at once may lose one of their samples. Sampling rides on the pane's runs: nothing samples while nothing runs `clm machine`, and the chart shows that gap only as fewer cells.

**Rows.** The current reading's state is written as a word so it reads without colour (`colour-meanings`); a past cell's state is shown by its colour alone, an exception `machine-cells-keep-their-state` records. The pane reads the escapes run by run (`plugin.panel/machine-charts`), which is why each run is reset on its own.

## MODIFIED Requirements

### Requirement: machine-report <!-- base: ec4a0cdd3ac20a18 -->
`clm machine` SHALL print the CPU, memory and disk readings and their states, as text by default, as JSON with `--format json` and as chart rows with `--format ansi`, and exit 0 whatever the states. `read` SHALL return those readings and states as a report, and `text`, `json` and `rows` SHALL give for a report what the command prints in each form. `read` SHALL return a report whatever the system refuses, whatever the record and the history files hold, and whether or not they can be read or written. Outside a git repository it SHALL judge the disk of `root`'s filesystem, with no builds, and keep no record and no history file.

The JSON SHALL be `{"view": "machine", "cpu": {...}, "memory": {...}, "disk": {...}}`: `cpu` with `cores`, `load1`, `load5` (both `null` when unknown) and `state`; `memory` with `total`, `available` (both `null` when the system gave no memory reading) and `state`; `disk` with `path` (the clone's common git directory, whose filesystem is read, or `root` outside a git repository), `total`, `available` (both `null` when the system gave no disk reading), `peak` (`null` before any measurement), `measured` (an RFC 3339 UTC time, `<yyyy>-<mm>-<dd>T<hh>:<mm>:<ss>Z`, or `null`), `builds` (each counting build's `name` and last measured `bytes`, or `null`), `free_places`, `need` (`null` with no `peak` or no disk reading), `room` (`disk-room`) and `state`, each state its word and all sizes in bytes. The text SHALL be three lines: `cpu  <load1 ratio>× of <cores> cores, <load5 ratio>× over 5 min  <state>`, each ratio the load average over the cores to one decimal, followed by `×`, or `-` alone in place of a ratio whose load average is unknown, each of the two by itself; `mem  <available> available of <total>  <state>`, or `mem  -  <state>` when the system gave no memory reading; and `disk <available> free of <total>, <need> needed by <n> builds  <state>`, `<n>` the number of counting builds and `, <need> needed by <n> builds` left out when `need` is `null`, or `disk -  <state>` when the system gave no disk reading. A size in the text and the rows SHALL be written as a number and a unit, `B`, `K`, `M`, `G` or `T`: the bytes divided by 1024 once for each unit after `B`, stopping at the first unit where the number, before it is rounded, is under 1000, or at `T`; then rounded to one decimal when the unit isn't `B` and the number before rounding is under 9.95, and to none otherwise, as in `512B`, `850M`, `12G` and `1.3T`, so 1,023,999 bytes read `1000K`.

CPU SHALL be judged on the load averages over the logical cores: `critical` when the 5-minute one is at least 1.25, else `warn` when the 1-minute one is at least 1.25, else `ok`. Memory SHALL be judged on the available memory, never the free memory: `critical` below 10% of total, `warn` below 20%, and `unknown` for a reading whose total is 0, whose numbers are printed all the same. Disk SHALL be judged against the builds' measured `peak` when the record holds one: the *growth* is the sum over the counting builds of `peak` less the build's last measured size, when positive, a build not measured owing the whole `peak`; `free_places` is `max_builds` less the number of counting builds, at least 0; the *need* is the growth plus `peak` times `free_places`; and the state is `critical` when free space is below the growth, else `warn` when it is below the need, else `ok`. With no `peak` it SHALL be `critical` below 5% of total and `warn` below 10%, a disk whose total is 0 being `critical`. A reading the system doesn't give SHALL be `unknown`, and with no disk reading `need` and `room` are `null` whatever the record holds. `judge(readings, record, counting, max_builds)` SHALL return the same states, `need`, `room` and `free_places` from those four alone.

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

#### Scenario: a total of 0
- GIVEN readings whose memory is 0 bytes available of a total of 0, and whose disk is 4096 bytes free of a total of 0, and a record with no `peak`
- WHEN they are judged, and the report is printed as text
- THEN `memory` is `unknown` and the text's second line is `mem  0B available of 0B  unknown`, and `disk` is `critical`

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
