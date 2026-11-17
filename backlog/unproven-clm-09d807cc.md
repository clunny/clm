---
title: Tests of clm notice no change to its code
kind: bug
components: [clm]
unproven: [clm/doc-readers-marked@a5e3c957]
---

After the land `09d807cc` the mutation pass changed the code of `clm`, and the tests that cite these requirements passed with every change it tried:

- `clm/doc-readers-marked`: `crates/clm/tests/doc_readers.rs` `the_retired_terms_test_reads_the_specs`, `crates/clm/tests/doc_readers.rs` `the_root_documents_have_readers`, `crates/clm/tests/doc_readers.rs` `the_readers_of_clm_toml`, `crates/clm/tests/doc_readers.rs` `the_guides_readers`, `crates/clm/tests/doc_readers.rs` `the_spec_formats_readers`, `crates/clm/tests/doc_readers.rs` `other_crates_read_architecture_md`, `crates/clm/tests/doc_readers.rs` `the_guides_readers_in_other_crates`, `crates/clm/tests/doc_readers.rs` `the_guides_directory_readers_mark_every_page`, `crates/clm/tests/doc_readers.rs` `the_settings_pages_reader`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
