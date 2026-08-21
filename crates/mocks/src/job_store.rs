use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::job::{
    Job, JobId, JobKind, JobNode, JobQuery, JobStatus, RECLAIM_DEAD_LETTER_ERROR, ReclaimOutcome,
};
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

    pub fn seed(&self, job: Job) {
        self.jobs.lock().unwrap().insert(job.id.clone(), job);
    }

    fn matching(&self, query: &JobQuery) -> Vec<Job> {
        let mut all: Vec<Job> = self
            .jobs
            .lock()
            .unwrap()
            .values()
            .filter(|job| query.matches(job))
            .cloned()
            .collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
        all
    }
}

fn walk(jobs: &HashMap<JobId, Job>, parent: &JobId, depth: u32, out: &mut Vec<JobNode>) {
    let mut children: Vec<&Job> = jobs
        .values()
        .filter(|job| job.parent_id.as_ref() == Some(parent))
        .collect();
    children.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    for child in children {
        out.push(JobNode {
            job: child.clone(),
            depth,
        });
        walk(jobs, &child.id, depth + 1, out);
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

    async fn reclaim_running(
        &self,
        now: Timestamp,
        max_attempts: u32,
    ) -> Result<ReclaimOutcome, RepositoryError> {
        let mut guard = self.jobs.lock().unwrap();
        let mut outcome = ReclaimOutcome::default();
        for job in guard.values_mut() {
            if job.status == JobStatus::Running {
                job.attempts += 1;
                job.started_at = None;
                job.updated_at = now;
                if job.attempts >= max_attempts {
                    job.status = JobStatus::Failed;
                    job.last_error = Some(RECLAIM_DEAD_LETTER_ERROR.to_string());
                    job.finished_at = Some(now);
                    outcome.dead_lettered += 1;
                } else {
                    job.status = JobStatus::Queued;
                    job.available_at = now;
                    outcome.requeued += 1;
                }
            }
        }
        Ok(outcome)
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

    async fn list_page(
        &self,
        query: &JobQuery,
        page: PageRequest,
    ) -> Result<Vec<Job>, RepositoryError> {
        Ok(self
            .matching(query)
            .into_iter()
            .skip(page.offset as usize)
            .take(page.limit as usize)
            .collect())
    }

    async fn count(&self, query: &JobQuery) -> Result<u64, RepositoryError> {
        Ok(self.matching(query).len() as u64)
    }

    async fn list_descendants(
        &self,
        root: &JobId,
        page: PageRequest,
    ) -> Result<Page<JobNode>, RepositoryError> {
        let guard = self.jobs.lock().unwrap();
        let mut nodes: Vec<JobNode> = Vec::new();
        walk(&guard, root, 0, &mut nodes);
        let total = nodes.len() as u64;
        let items = nodes
            .into_iter()
            .skip(page.offset as usize)
            .take(page.limit as usize)
            .collect();
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
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

    async fn delete_finished_before(
        &self,
        cutoff: Timestamp,
    ) -> Result<Vec<JobId>, RepositoryError> {
        let mut guard = self.jobs.lock().unwrap();
        let removed: Vec<JobId> = guard
            .values()
            .filter(|job| !job.status.is_active())
            .filter(|job| job.finished_at.is_some_and(|at| at < cutoff))
            .map(|job| job.id.clone())
            .collect();
        for id in &removed {
            guard.remove(id);
        }
        Ok(removed)
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
