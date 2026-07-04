//! GT06 timestamps are plain binary (not BCD) year/month/day/hour/minute/second
//! fields, always in UTC, with a 2-digit year offset from 2000.

/// Converts a GT06 date/time field to a Unix timestamp (seconds since epoch, UTC).
pub fn unix_timestamp(yy: u8, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> i64 {
    let days = days_from_civil(2000 + yy as i64, month as u32, day as u32);
    days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64
}

/// Days since the Unix epoch for a given proleptic Gregorian civil date.
/// Howard Hinnant's `days_from_civil` algorithm.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_fix_time() {
        // 2024-06-15 10:30:45 UTC -> 1718447445 (verified with `date -u`)
        assert_eq!(unix_timestamp(24, 6, 15, 10, 30, 45), 1718447445);
    }

    #[test]
    fn epoch_start() {
        assert_eq!(unix_timestamp(0, 1, 1, 0, 0, 0), 946_684_800);
    }

    #[test]
    fn leap_day() {
        // 2024-02-29 00:00:00 UTC
        assert_eq!(unix_timestamp(24, 2, 29, 0, 0, 0), 1_709_164_800);
    }

    #[test]
    fn year_end_rollover() {
        // 2023-12-31 23:59:59 UTC
        assert_eq!(unix_timestamp(23, 12, 31, 23, 59, 59), 1_704_067_199);
    }
}
