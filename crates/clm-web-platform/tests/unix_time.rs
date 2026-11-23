//! `install::unix_time`: the instant an RFC 3339 date-time names, or none.

use clm_web_platform::install::unix_time;

/// 10:00:00 UTC on 2026-10-01.
const OCT_1_10H: u64 = 1_790_848_800;
/// Midnight UTC on 2028-02-29.
const FEB_29_2028: u64 = 1_835_395_200;

#[test]
fn scenario_times_github_gives() {
    // req:web.worker.platform/unix-time@add5963f
    for text in [
        "2026-10-01T10:00:00Z",
        "2026-10-01T12:00:00+02:00",
        "2026-10-01t08:30:00.75-01:30",
        "2026-10-01T10:00:00z",
    ] {
        assert_eq!(unix_time(text), Some(OCT_1_10H), "{text}");
    }
    assert_eq!(unix_time("2028-02-29T00:00:00Z"), Some(FEB_29_2028));
}

#[test]
fn scenario_text_that_isnt_such_a_time() {
    // req:web.worker.platform/unix-time@add5963f
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
    ] {
        assert_eq!(unix_time(text), None, "{text}");
    }
}

#[test]
fn each_field_is_bounded_and_a_leap_second_counts() {
    // req:web.worker.platform/unix-time@add5963f
    for text in [
        "2026-13-01T12:00:00Z",
        "2026-00-01T12:00:00Z",
        "2026-10-00T12:00:00Z",
        "2026-04-31T12:00:00Z",
        "2026-10-01T12:60:00Z",
        "2026-10-01T12:00:61Z",
        "2026-10-01T12:00:00+02:60",
        "2026-10-01T12:00:00+0200",
        "2026-10-01T12:00:00+02x00",
        "2026x10-01T12:00:00Z",
        "2026-10x01T12:00:00Z",
        "2026-10-01T12x00:00Z",
        "2026-10-01T12:00x00Z",
        "2026-10-01 12:00:00Z",
        "+026-10-01T12:00:00Z",
        "2026-10-01T12:00:00.5x5Z",
        "2100-02-29T00:00:00Z",
        "1970-01-01T00:00:00+00:01",
    ] {
        assert_eq!(unix_time(text), None, "{text}");
    }
    assert_eq!(unix_time("2000-02-29T00:00:00Z"), Some(951_782_400));
    assert!(unix_time("2026-10-01T09:59:60Z").is_some());
    assert_eq!(unix_time("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(unix_time("2026-10-01T10:00:00-00:00"), Some(OCT_1_10H));
}
