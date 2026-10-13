use std::time::{SystemTime, UNIX_EPOCH};

/// Wall clock in epoch ms: the clock the parser stamps hits with.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}
