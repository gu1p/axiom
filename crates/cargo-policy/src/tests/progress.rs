use core::time::Duration;

use super::{clock, compact_elapsed};

#[test]
fn clock_displays_minutes_seconds_and_tenths() {
    assert_eq!(clock(Duration::ZERO), "00:00.0");
    assert_eq!(clock(Duration::from_millis(62_349)), "01:02.3");
    assert_eq!(clock(Duration::from_secs(3_661)), "61:01.0");
}

#[test]
fn elapsed_time_is_compact_for_completion_lines() {
    assert_eq!(compact_elapsed(Duration::from_millis(87)), "87ms");
    assert_eq!(compact_elapsed(Duration::from_millis(1_250)), "1.2s");
}
