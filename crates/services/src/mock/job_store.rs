use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;

#[derive(Clone, Default)]
pub struct MockJobStore {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
}

impl MockJobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobRepository for MockJobStore {
    async fn enqueue(&self, job: Job) -> Result<(), RepositoryError> {
        self.jobs.lock().unwrap().insert(job.id.clone(), job);
        Ok(())
    }

    async fn claim_ready(
        &self,
        now: Timestamp,
        limit: usize,
        kinds: Vec<JobKind>,
    ) -> Result<Vec<Job>, RepositoryError> {
        let mut guard = self.jobs.lock().unwrap();
        let mut ready: Vec<Job> = guard
            .values()
            .filter(|job| {
                job.status == JobStatus::Queued
                    && job.available_at <= now
                    && kinds.contains(&job.kind)
            })
            .cloned()
            .collect();
        ready.sort_by(|a, b| {
            Reverse(a.priority)
                .cmp(&Reverse(b.priority))
                .then(a.created_at.cmp(&b.created_at))
                .then(a.id.cmp(&b.id))
        });
        ready.truncate(limit);
        for job in &mut ready {
            job.status = JobStatus::Running;
            job.started_at = Some(now);
            guard.insert(job.id.clone(), job.clone());
        }
        Ok(ready)
    }

    async fn reclaim_running(&self, now: Timestamp) -> Result<usize, RepositoryError> {
        let mut guard = self.jobs.lock().unwrap();
        let mut count = 0;
        for job in guard.values_mut() {
            if job.status == JobStatus::Running {
                job.status = JobStatus::Queued;
                job.available_at = now;
                job.started_at = None;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn update(&self, job: Job) -> Result<(), RepositoryError> {
        self.jobs.lock().unwrap().insert(job.id.clone(), job);
        Ok(())
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, RepositoryError> {
        Ok(self.jobs.lock().unwrap().get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<Job>, RepositoryError> {
        let mut all: Vec<Job> = self.jobs.lock().unwrap().values().cloned().collect();
        all.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Ok(all)
    }

    async fn cancel(&self, id: &JobId, now: Timestamp) -> Result<bool, RepositoryError> {
        let mut guard = self.jobs.lock().unwrap();
        match guard.get_mut(id) {
            Some(job) if job.status == JobStatus::Queued => {
                job.status = JobStatus::Cancelled;
                job.finished_at = Some(now);
                job.updated_at = now;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobKind, JobPriority};
    use jiff::SignedDuration;

    fn job(id: &str, priority: JobPriority, available_at: Timestamp, created_at: Timestamp) -> Job {
        Job {
            id: JobId(id.into()),
            kind: JobKind::LibraryScan,
            status: JobStatus::Queued,
            priority,
            payload: String::new(),
            attempts: 0,
            progress: 0.0,
            available_at,
            last_error: None,
            created_at,
            updated_at: created_at,
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    #[tokio::test]
    async fn claim_orders_by_priority_then_age() {
        let now = Timestamp::now();
        let older = now.saturating_sub(SignedDuration::from_secs(10)).unwrap();
        let store = MockJobStore::new();
        store
            .enqueue(job("low", JobPriority::Low, now, now))
            .await
            .unwrap();
        store
            .enqueue(job("high", JobPriority::High, now, now))
            .await
            .unwrap();
        store
            .enqueue(job("normal-old", JobPriority::Normal, now, older))
            .await
            .unwrap();
        store
            .enqueue(job("normal-new", JobPriority::Normal, now, now))
            .await
            .unwrap();

        let claimed = store
            .claim_ready(now, 10, vec![JobKind::LibraryScan])
            .await
            .unwrap();
        let ids: Vec<&str> = claimed.iter().map(|j| j.id.0.as_str()).collect();
        assert_eq!(ids, ["high", "normal-old", "normal-new", "low"]);
        assert!(claimed.iter().all(|j| j.status == JobStatus::Running));
    }

    #[tokio::test]
    async fn claim_respects_limit_and_marks_running() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now, now))
            .await
            .unwrap();
        store
            .enqueue(job("b", JobPriority::Normal, now, now))
            .await
            .unwrap();

        let claimed = store
            .claim_ready(now, 1, vec![JobKind::LibraryScan])
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        let running = store
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|j| j.status == JobStatus::Running)
            .count();
        assert_eq!(running, 1);
    }

    #[tokio::test]
    async fn claim_excludes_not_ready_and_non_queued() {
        let now = Timestamp::now();
        let future = now.saturating_add(SignedDuration::from_secs(60)).unwrap();
        let store = MockJobStore::new();
        store
            .enqueue(job("future", JobPriority::High, future, now))
            .await
            .unwrap();
        let mut running = job("running", JobPriority::High, now, now);
        running.status = JobStatus::Running;
        store.enqueue(running).await.unwrap();

        assert!(
            store
                .claim_ready(now, 10, vec![JobKind::LibraryScan])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn update_and_get_round_trip() {
        let now = Timestamp::now();
        let store = MockJobStore::new();
        store
            .enqueue(job("a", JobPriority::Normal, now, now))
            .await
            .unwrap();

        let mut updated = store.get(&JobId("a".into())).await.unwrap().unwrap();
        updated.status = JobStatus::Succeeded;
        updated.progress = 1.0;
        store.update(updated).await.unwrap();

        let stored = store.get(&JobId("a".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Succeeded);
        assert!(store.get(&JobId("missing".into())).await.unwrap().is_none());
    }
}
