//! GT06 timestamps are plain binary (not BCD) year/month/day/hour/minute/second
//! fields, always in UTC, with a 2-digit year offset from 2000.

/// Converts a GT06 date/time field to a Unix timestamp (seconds since epoch, UTC).
pub fn unix_timestamp(yy: u8, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> i64 {
    let _ = (yy, month, day, hour, minute, second);
    todo!()
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
