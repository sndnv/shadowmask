use std::future::Future;
use std::time::Duration;

use domain::error::RepositoryError;
use domain::repository::JobRepository;
use jiff::Timestamp;
use tokio::time::interval;

use crate::queue::JobQueue;
use crate::schedule::Schedule;

#[derive(Default)]
pub struct Scheduler {
    schedules: Vec<Schedule>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, schedule: Schedule) {
        self.schedules.push(schedule);
    }

    pub async fn tick<R: JobRepository>(
        &mut self,
        now: Timestamp,
        queue: &JobQueue<R>,
    ) -> Result<usize, RepositoryError> {
        let mut fired = 0;
        for schedule in &mut self.schedules {
            if schedule.next_fire_at <= now {
                queue
                    .enqueue(
                        schedule.kind,
                        schedule.priority,
                        schedule.payload.clone(),
                        now,
                    )
                    .await?;
                schedule.next_fire_at = schedule
                    .next_fire_at
                    .saturating_add(schedule.every)
                    .unwrap_or(schedule.next_fire_at);
                fired += 1;
            }
        }
        Ok(fired)
    }

    pub async fn run<R: JobRepository>(
        &mut self,
        period: Duration,
        queue: &JobQueue<R>,
        shutdown: impl Future<Output = ()>,
    ) -> Result<(), RepositoryError> {
        let mut ticker = interval(period);
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => return Ok(()),
                _ = ticker.tick() => {
                    self.tick(Timestamp::now(), queue).await?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobKind, JobPriority};
    use jiff::SignedDuration;
    use services::mock::MockJobStore;

    fn schedule(every_secs: i64, next_fire_at: Timestamp) -> Schedule {
        Schedule {
            kind: JobKind::LibraryScan,
            priority: JobPriority::Normal,
            payload: String::new(),
            every: SignedDuration::from_secs(every_secs),
            next_fire_at,
        }
    }

    #[tokio::test]
    async fn tick_fires_due_schedule_and_advances() {
        let now = Timestamp::now();
        let queue = JobQueue::new(MockJobStore::new());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(60, now));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        assert_eq!(queue.list().await.unwrap().len(), 1);

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 0);
        assert_eq!(queue.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tick_skips_not_due_schedule() {
        let now = Timestamp::now();
        let future = now.saturating_add(SignedDuration::from_secs(60)).unwrap();
        let queue = JobQueue::new(MockJobStore::new());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(60, future));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 0);
        assert!(queue.list().await.unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn run_ticks_until_shutdown() {
        use std::time::Duration;
        use tokio::sync::oneshot;

        let queue = JobQueue::new(MockJobStore::new());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(60, Timestamp::now()));

        let (tx, rx) = oneshot::channel::<()>();
        let driver = scheduler.run(Duration::from_millis(10), &queue, async move {
            let _ = rx.await;
        });
        let control = async {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let _ = tx.send(());
        };
        let (result, ()) = tokio::join!(driver, control);
        result.unwrap();

        assert!(!queue.list().await.unwrap().is_empty());
    }
}
