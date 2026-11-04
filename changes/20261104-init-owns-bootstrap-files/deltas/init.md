---
component: init
---

## MODIFIED Requirements

### Requirement: init-root-ownership <!-- base: 8c7cd82cd88a4725 -->
The `specs/SPEC.md` that `clm init` writes SHALL set `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]`. It SHALL set `outside` to the top-level entries of the repository's other code files: of the files the bootstrap commit will hold, which are the files the index holds under the CLM repository and the untracked files under it that the `.gitignore` doesn't ignore as `clm init` leaves it (`clm-git`'s `files_to_commit`, read after `clm init` has written its `.gitignore` lines, so `clm init` writes those lines before it writes `specs/SPEC.md`), other than those under `specs/`, `changes/`, `backlog/` or `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and the three files in `paths`, the first component of each path, written `<component>/**` for a file in a directory and as the file's name for a file at the top, each once, sorted by bytes, each written as a YAML double-quoted string with `\` and `"` escaped; and `outside: []` when there is none, so path ownership is always on in a repository `clm init` sets up. So the committed result of `clm init` in a repository with code passes `clm check`'s path ownership (`check/unowned-files`), even when it commits a file that was untracked when `clm init` ran, such as the `.claude/settings.json` Claude Code writes when the plugins are enabled for the project, and the first spec graph narrows `outside` as its components take the code (`incremental-adoption`). `clm survey` and `clm survey --components` still read the index alone. Its tests assert the frontmatter written; the check itself is `check`'s.

#### Scenario: repository with code
- GIVEN a git repository with no `specs/` tracking `src/main.rs`, `src/lib/x.rs`, `scripts/build.sh`, `README.md` and `docs/a.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: ["README.md", "scripts/**", "src/**"]`

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: []`

#### Scenario: untracked files the bootstrap commits
- GIVEN a git repository with no `specs/`, no global or system git excludes, tracking `src/main.rs` and a `.gitignore` holding `build/`, with the untracked files `.claude/settings.json`, `build/out.o` and `.clm/pr-body.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `outside: [".claude/**", "src/**"]`, naming neither `build/**`, which the `.gitignore` ignored before, nor `.clm/**`, which the line `clm init` adds ignores

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs
- vcs/files-to-commit
