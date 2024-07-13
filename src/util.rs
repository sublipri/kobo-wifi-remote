use chrono::Duration;
use tracing::trace;

pub fn sleep(duration: Duration) {
    if duration > Duration::zero() {
        trace!("Sleeping for {}ms", duration.num_milliseconds());
        std::thread::sleep(duration.to_std().unwrap());
    }
}
