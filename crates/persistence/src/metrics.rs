use std::time::Instant;

pub(crate) struct DbOpGuard {
    store: &'static str,
    operation: &'static str,
    start: Instant,
}

impl DbOpGuard {
    pub(crate) fn new(store: &'static str, operation: &'static str) -> Self {
        Self { store, operation, start: Instant::now() }
    }
}

impl Drop for DbOpGuard {
    fn drop(&mut self) {
        metrics::histogram!(
            "db_operation_duration_milliseconds",
            "store" => self.store,
            "operation" => self.operation,
        )
        .record(self.start.elapsed().as_nanos() as f64 / 1_000_000.0);
        metrics::counter!(
            "db_operations_total",
            "store" => self.store,
            "operation" => self.operation,
        )
        .increment(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics_exporter_prometheus::PrometheusBuilder;

    #[test]
    fn guard_records_counter_and_duration_on_drop() {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let guard = DbOpGuard::new("catalog", "insert_movie");
            drop(guard);
        });
        let rendered = handle.render();
        assert!(rendered.contains("db_operations_total"));
        assert!(rendered.contains("db_operation_duration_milliseconds"));
        assert!(rendered.contains("store=\"catalog\""));
        assert!(rendered.contains("operation=\"insert_movie\""));
    }
}
