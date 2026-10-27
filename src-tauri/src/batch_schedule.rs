use std::sync::atomic::{AtomicBool, Ordering};
pub fn now() -> Result<u64, String> {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64).map_err(|error| error.to_string())
}
pub fn validate(at: Option<u64>, now: u64) -> Result<(), String> {
    if let Some(at) = at {
        if at <= now { return Err("Choose a future batch start time".into()); }
        if at - now > 31 * 24 * 60 * 60 * 1000 { return Err("Batch start must be within 31 days".into()); }
    }
    Ok(())
}
// Cancellation is checked again after the deadline so a cancelled wait never starts work.
pub fn wait(at: Option<u64>, cancel: &AtomicBool) -> Result<bool, String> {
    loop {
        if cancel.load(Ordering::Relaxed) { return Ok(false); }
        let remaining = at.unwrap_or(0).saturating_sub(now()?);
        if remaining == 0 { return Ok(!cancel.load(Ordering::Relaxed)); }
        std::thread::sleep(std::time::Duration::from_millis(remaining.min(100)));
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn validates_deadlines_without_clock_races() {
        assert!(validate(None, 100).is_ok());
        assert!(validate(Some(101), 100).is_ok());
        assert!(validate(Some(100), 100).is_err());
        assert!(validate(Some(99), 100).is_err());
        assert!(validate(Some(100 + 31*86400000), 100).is_ok());
        assert!(validate(Some(101 + 31*86400000), 100).is_err());
    }
    #[test] fn cancelled_wait_never_runs_even_when_due() {
        let cancel = AtomicBool::new(true);
        assert!(!wait(Some(1), &cancel).unwrap());
        assert!(!wait(None, &cancel).unwrap());
        cancel.store(false, Ordering::Relaxed);
        assert!(wait(Some(1), &cancel).unwrap());
    }
}
