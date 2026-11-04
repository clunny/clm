---
component: vcs
---

## ADDED Requirements

### Requirement: files-to-commit
`files_to_commit(root: &Path) -> Result<Vec<PathBuf>, Error>` SHALL return every file the index holds under `root` and every untracked file under it that git's standard excludes don't ignore (`.gitignore` files, `.git/info/exclude` and `core.excludesFile`), as `git ls-files -z --cached --others --exclude-standard --deduplicate` run in `root` names them, each once, so a caller sees the files a `git add -A` would commit. A file deleted from the work tree that the index still holds is listed, as `tracked_files` lists it. On Unix each returned path's bytes SHALL be exactly the name git lists, whether or not it is UTF-8. `tracked_files` and `tracked_matching` keep reading the index alone.

#### Scenario: tracked and untracked, not ignored
- GIVEN a repository tracking `src/a.rs` and `.gitignore` holding `build/`, with the untracked files `notes.txt` and `build/out.o`
- WHEN `files_to_commit(root)` runs
- THEN it returns `.gitignore`, `notes.txt` and `src/a.rs`, and not `build/out.o`

#### Scenario: a file both staged and changed once
- GIVEN a repository tracking `src/a.rs`, changed in the work tree since it was staged
- WHEN `files_to_commit(root)` runs
- THEN it lists `src/a.rs` exactly once

#### Serves
- capability/adopt-clm
