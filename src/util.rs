use chrono::Duration;
use tracing::debug;

pub fn sleep(duration: Duration) {
    if duration > Duration::zero() {
        debug!("Sleeping for {}ms", duration.num_milliseconds());
        std::thread::sleep(duration.to_std().unwrap());
    }
}
