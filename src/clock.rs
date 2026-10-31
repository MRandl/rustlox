use std::time::{SystemTime, UNIX_EPOCH};

pub fn seconds_since_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock is set after the Unix epoch")
        .as_secs_f64()
}
