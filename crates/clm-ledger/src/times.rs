//! A record's `at` written, and an RFC 3339 time read (`ledger/record-times`).

/// The instant `secs` Unix seconds and `millis` milliseconds (below 1000) after it, as
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`, the form a record's `at` is written in.
pub fn rfc3339(secs: u64, millis: u32) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
    )
}

/// The proleptic Gregorian date of `days` since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
pub(crate) fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Unix seconds of the instant an RFC 3339 date-time names:
/// `YYYY-MM-DD`, `T` or `t`, `HH:MM:SS`, an optional fraction (dropped),
/// and `Z`, `z` or `±HH:MM` (counted). `None` for any other text, a field
/// out of range, or an instant before 1970.
pub fn unix_time(iso: &str) -> Option<u64> {
    let b = iso.as_bytes();
    let digits = |at: usize, n: usize| -> Option<i64> {
        let s = b.get(at..at + n)?;
        s.iter()
            .all(u8::is_ascii_digit)
            .then(|| s.iter().fold(0i64, |acc, c| acc * 10 + i64::from(c - b'0')))
    };
    let sep = |at: usize, set: &[u8]| b.get(at).is_some_and(|c| set.contains(c));
    if !(sep(4, b"-") && sep(7, b"-") && sep(10, b"Tt") && sep(13, b":") && sep(16, b":")) {
        return None;
    }
    let (y, m, d) = (digits(0, 4)?, digits(5, 2)?, digits(8, 2)?);
    let (hh, mm, ss) = (digits(11, 2)?, digits(14, 2)?, digits(17, 2)?);
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if d < 1 || d > month_days || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let mut at = 19;
    if sep(at, b".") {
        let start = at + 1;
        at = start;
        while b.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    let offset = match b.get(at)? {
        b'Z' | b'z' => {
            at += 1;
            0
        }
        sign @ (b'+' | b'-') => {
            let (oh, om) = (digits(at + 1, 2)?, digits(at + 4, 2)?);
            if !sep(at + 3, b":") || oh > 23 || om > 59 {
                return None;
            }
            at += 6;
            let secs = oh * 3600 + om * 60;
            if *sign == b'+' { secs } else { -secs }
        }
        _ => return None,
    };
    if at != b.len() {
        return None;
    }
    // Days from civil (Howard Hinnant).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss - offset).ok()
}
