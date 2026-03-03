use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use once_cell::sync::Lazy;
use salvo::prelude::*;
use tracing::info;

#[derive(Default)]
pub struct Metrics {
    pub messages_from_matrix: AtomicU64,
    pub messages_from_dingtalk: AtomicU64,
    pub messages_to_matrix: AtomicU64,
    pub messages_to_dingtalk: AtomicU64,
    pub bridge_errors: AtomicU64,
    pub http_requests: AtomicU64,
}

pub static GLOBAL_METRICS: Lazy<Metrics> = Lazy::new(Metrics::default);

pub fn global_metrics() -> &'static Metrics {
    &GLOBAL_METRICS
}

#[handler]
pub async fn metrics_endpoint(res: &mut Response) {
    let metrics = global_metrics();

    let output = format!(
        r#"# HELP messages_from_matrix Total messages received from Matrix
# TYPE messages_from_matrix counter
messages_from_matrix {}

# HELP messages_from_dingtalk Total messages received from DingTalk
# TYPE messages_from_dingtalk counter
messages_from_dingtalk {}

# HELP messages_to_matrix Total messages sent to Matrix
# TYPE messages_to_matrix counter
messages_to_matrix {}

# HELP messages_to_dingtalk Total messages sent to DingTalk
# TYPE messages_to_dingtalk counter
messages_to_dingtalk {}

# HELP bridge_errors Total bridge errors
# TYPE bridge_errors counter
bridge_errors {}

# HELP http_requests Total HTTP requests
# TYPE http_requests counter
http_requests {}
"#,
        metrics.messages_from_matrix.load(Ordering::Relaxed),
        metrics.messages_from_dingtalk.load(Ordering::Relaxed),
        metrics.messages_to_matrix.load(Ordering::Relaxed),
        metrics.messages_to_dingtalk.load(Ordering::Relaxed),
        metrics.bridge_errors.load(Ordering::Relaxed),
        metrics.http_requests.load(Ordering::Relaxed),
    );

    res.add_header("Content-Type", "text/plain; version=0.0.4", true)
        .unwrap();
    res.render(Text::Plain(output));
}

pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        info!("{} took {:?}", self.name, elapsed);
    }
}
