use jiff::Timestamp;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use domain::job::{JobId, JobLogLevel};
use persistence::job_log::FsJobLogStore;

const JOB_SPAN: &str = "job";

struct JobTag(JobId);

#[derive(Default)]
struct JobIdVisitor(Option<String>);

impl Visit for JobIdVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "job_id" {
            self.0 = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "job_id" && self.0.is_none() {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

#[derive(Default)]
struct MessageVisitor(Option<String>);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

pub struct JobLogLayer {
    store: FsJobLogStore,
}

impl JobLogLayer {
    pub fn new(store: FsJobLogStore) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for JobLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != JOB_SPAN {
            return;
        }
        let mut visitor = JobIdVisitor::default();
        attrs.record(&mut visitor);
        if let Some(job_id) = visitor.0
            && let Some(span) = ctx.span(id)
        {
            span.extensions_mut().insert(JobTag(JobId(job_id)));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let level = match *event.metadata().level() {
            Level::ERROR => JobLogLevel::Error,
            Level::WARN => JobLogLevel::Warn,
            Level::INFO => JobLogLevel::Info,
            Level::DEBUG => JobLogLevel::Debug,
            _ => return,
        };
        let Some(scope) = ctx.event_scope(event) else {
            return;
        };
        let job = scope
            .from_root()
            .find_map(|span| span.extensions().get::<JobTag>().map(|tag| tag.0.clone()));
        let Some(job) = job else {
            return;
        };
        let mut message = MessageVisitor::default();
        event.record(&mut message);
        let Some(message) = message.0 else {
            return;
        };
        let _ = self.store.append_blocking(&job, Timestamp::now(), level, &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobLogStore;
    use tracing_subscriber::Registry;
    use tracing_subscriber::layer::SubscriberExt;

    fn subscriber(store: FsJobLogStore) -> impl Subscriber + Send + Sync {
        Registry::default().with(JobLogLayer::new(store))
    }

    #[tokio::test]
    async fn captures_leveled_events_within_a_job_span() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsJobLogStore::new(dir.path());
        tracing::subscriber::with_default(subscriber(store.clone()), || {
            let span = tracing::info_span!("job", job_id = %"job-1", kind = "LibraryScan");
            let _enter = span.enter();
            tracing::info!("scanning library");
            tracing::warn!("slow disk");
            tracing::debug!("inspecting file");
            tracing::trace!("dropped as too noisy");
            tracing::info!(count = 5);
            tracing::error!("boom");
        });

        let lines = store.read(&JobId("job-1".into()), None).await.unwrap();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("INFO scanning library"));
        assert!(lines[1].contains("WARN slow disk"));
        assert!(lines[2].contains("DEBUG inspecting file"));
        assert!(lines[3].contains("ERROR boom"));
    }

    #[tokio::test]
    async fn accepts_a_string_job_id_field() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsJobLogStore::new(dir.path());
        tracing::subscriber::with_default(subscriber(store.clone()), || {
            let span = tracing::info_span!("job", job_id = "job-2");
            let _enter = span.enter();
            tracing::info!("hello");
        });

        let lines = store.read(&JobId("job-2".into()), None).await.unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("INFO hello"));
    }

    #[tokio::test]
    async fn ignores_events_outside_a_job_span() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsJobLogStore::new(dir.path());
        tracing::subscriber::with_default(subscriber(store.clone()), || {
            tracing::info!("orphan event");
            let other = tracing::info_span!("not_a_job", job_id = %"job-x");
            let _enter = other.enter();
            tracing::info!("wrong span");
        });

        assert!(store.read(&JobId("job-x".into()), None).await.unwrap().is_empty());
    }
}
