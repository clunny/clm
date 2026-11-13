//! The one time the loop reads: the `--until` of a `limit` stop, an RFC 3339 time.

/// Unix seconds of `YYYY-MM-DDTHH:MM:SS`, optional fractional seconds, then `Z` or an offset
/// `+HH:MM` or `-HH:MM`; none for text that doesn't read that way.
pub(crate) fn unix(text: &str) -> Option<u64> {
    if !text.is_ascii() || text.len() < 20 {
        return None;
    }
    let b = text.as_bytes();
    if b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = text.get(r)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let mut rest = &text[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &frac[digits..];
    }
    let offset = match rest.as_bytes() {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), ..] if rest.len() == 6 && rest.as_bytes()[3] == b':' => {
            let hours = rest.get(1..3)?.parse::<i64>().ok()?;
            let minutes = rest.get(4..6)?.parse::<i64>().ok()?;
            let secs = hours * 3600 + minutes * 60;
            if *sign == b'+' { secs } else { -secs }
        }
        _ => return None,
    };
    // Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss - offset).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rfc_3339() {
        assert_eq!(unix("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix("2024-02-29T00:00:00Z"), Some(1_709_164_800));
        assert_eq!(unix("2026-11-13T14:00:00Z"), Some(1_794_578_400));
        assert_eq!(unix("2026-11-13T15:00:00+01:00"), Some(1_794_578_400));
        assert_eq!(unix("2026-11-13T14:00:00.250Z"), Some(1_794_578_400));
        assert_eq!(unix("tomorrow"), None);
        assert_eq!(unix("2026-13-13T14:00:00Z"), None);
    }
}
