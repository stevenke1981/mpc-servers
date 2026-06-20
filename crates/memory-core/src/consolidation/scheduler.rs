use crate::consolidation::ConsolidationEngine;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info};

/// Background decay scheduler that runs Ebbinghaus decay on a timer.
/// Spawned as a Tokio task, runs every `interval` (default 24 hours).
pub struct DecayScheduler {
    engine: Arc<ConsolidationEngine>,
    interval: Duration,
}

impl DecayScheduler {
    pub fn new(engine: Arc<ConsolidationEngine>, interval: Duration) -> Self {
        Self { engine, interval }
    }

    /// Compute a simple jitter offset (0 to max_secs seconds) from the host identity.
    fn startup_jitter(max_secs: u64) -> Duration {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let offset = (seed % (max_secs as u128 * 1_000_000_000)) as u64 / 1_000_000;
        Duration::from_millis(offset)
    }

    /// Start the background decay loop. This never returns (runs forever).
    /// The first run is delayed by a jitter based on host time to avoid
    /// multiple replicas stampeding the database simultaneously.
    pub async fn run(self) {
        let jitter = Self::startup_jitter(3600);
        info!("DecayScheduler: initial sleep with {jitter:?} jitter");
        tokio::time::sleep(jitter).await;

        loop {
            info!("DecayScheduler: starting batch consolidation");
            match self.engine.batch_consolidate(None, None).await {
                Ok(_) => info!("DecayScheduler: batch consolidation completed"),
                Err(e) => error!("DecayScheduler: batch consolidation failed: {e}"),
            }

            tokio::time::sleep(self.interval).await;
        }
    }
}
