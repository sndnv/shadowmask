use std::time::Instant;

use domain::job::JobKind;

pub(crate) struct ActiveJob {
    job: &'static str,
    pool: String,
    start: Instant,
}

impl ActiveJob {
    pub(crate) fn start(kind: JobKind, pool: &str) -> Self {
        metrics::gauge!("jobs_active", "pool" => pool.to_owned()).increment(1.0);
        Self { job: kind.slug(), pool: pool.to_owned(), start: Instant::now() }
    }

    pub(crate) fn finish(self, succeeded: bool) {
        let outcome = if succeeded { "succeeded" } else { "failed" };
        metrics::histogram!("job_duration_milliseconds", "job" => self.job)
            .record(self.start.elapsed().as_nanos() as f64 / 1_000_000.0);
        metrics::counter!("jobs_total", "job" => self.job, "outcome" => outcome).increment(1);
    }
}

impl Drop for ActiveJob {
    fn drop(&mut self) {
        metrics::gauge!("jobs_active", "pool" => self.pool.clone()).decrement(1.0);
    }
}
