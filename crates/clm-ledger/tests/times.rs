//! `rfc3339` and `unix_time`, with or without the `git` feature (`ledger/record-times`).

use clm_ledger::{rfc3339, unix_time};

/// 10:00:00 UTC on 2026-10-01.
const OCT_1_10H: u64 = 1_790_848_800;
/// Midnight UTC on 2028-02-29.
const FEB_29_2028: u64 = 1_835_395_200;

#[test]
fn times_github_gives() {
    // req:ledger/record-times@62673566
    for text in [
        "2026-10-01T10:00:00Z",
        "2026-10-01T12:00:00+02:00",
        "2026-10-01t08:30:00.75-01:30",
        "2026-10-01T10:00:00z",
        "2026-10-01T10:00:00-00:00",
    ] {
        assert_eq!(unix_time(text), Some(OCT_1_10H), "{text}");
    }
    assert_eq!(unix_time("2028-02-29T00:00:00Z"), Some(FEB_29_2028));
    assert_eq!(unix_time("2000-02-29T00:00:00Z"), Some(951_782_400));
    assert_eq!(unix_time("1970-01-01T00:00:00Z"), Some(0));
    // The leap second counts as the first second of the next minute.
    assert_eq!(unix_time("2026-10-01T09:59:60Z"), Some(OCT_1_10H));
}

#[test]
fn text_that_isnt_such_a_time() {
    // req:ledger/record-times@62673566
    for text in [
        "2026-10-01T12:00:00",
        "2026-10-01T12:00:00+02:00x",
        "2026-10-01T12:00:00Zx",
        "2026-1a-01T12:00:00Z",
        "2026-02-30T12:00:00Z",
        "2026-02-29T12:00:00Z",
        "2026-10-01T12:00:00+24:00",
        "2026-10-01T24:00:00Z",
        "2026-10-01T12:00:00+2:00",
        "2026-10-01T12:00:00.Z",
        "2026-10-01",
        "1969-12-31T23:59:59Z",
        "2026-13-01T12:00:00Z",
        "2026-00-01T12:00:00Z",
        "2026-10-00T12:00:00Z",
        "2026-04-31T12:00:00Z",
        "2026-10-01T12:60:00Z",
        "2026-10-01T12:00:61Z",
        "2026-10-01T12:00:00+02:60",
        "2026-10-01T12:00:00+0200",
        "2026-10-01 12:00:00Z",
        "+026-10-01T12:00:00Z",
        "2026-10-01T12:00:00.5x5Z",
        "2100-02-29T00:00:00Z",
        "1970-01-01T00:00:00+00:01",
        // A non-digit or a wrong mark where each separator goes.
        "2026-10-01T12:00:00+02x00",
        "2026x10-01T12:00:00Z",
        "2026-10x01T12:00:00Z",
        "2026-10-01T12x00:00Z",
        "2026-10-01T12:00x00Z",
    ] {
        assert_eq!(unix_time(text), None, "{text}");
    }
}

#[test]
fn a_records_time() {
    // req:ledger/record-times@62673566
    for (secs, millis, text) in [
        (0, 0, "1970-01-01T00:00:00.000Z"),
        (951_782_400, 5, "2000-02-29T00:00:00.005Z"),
        (1_800_000_000, 999, "2027-01-15T08:00:00.999Z"),
    ] {
        assert_eq!(rfc3339(secs, millis), text);
        assert_eq!(unix_time(&rfc3339(secs, millis)), Some(secs), "{text}");
    }
}
