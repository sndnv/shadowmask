use std::future::Future;
use std::time::Duration;

use domain::error::RepositoryError;
use domain::repository::JobRepository;
use jiff::{SignedDuration, Timestamp};
use tokio::time::interval;

use crate::queue::JobQueue;
use crate::schedule::Schedule;

fn next_fire(schedule: &Schedule, now: Timestamp) -> Timestamp {
    let (due, every) = (schedule.next_fire_at, schedule.every);
    let next = due.saturating_add(every).unwrap_or(due);
    if next > now || !every.is_positive() {
        return next;
    }
    let late = now.duration_since(due).as_nanos() % every.as_nanos();
    let wait = i64::try_from(every.as_nanos() - late).map_or(every, SignedDuration::from_nanos);
    let skipped = now.saturating_add(wait).unwrap_or(now);
    #[rustfmt::skip]
    tracing::info!("schedule [{:?}] was due at [{due}] and missed the runs since; the next is at [{skipped}]", schedule.kind);
    skipped
}

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
                    .enqueue(schedule.kind, schedule.priority, schedule.payload.clone(), now)
                    .await?;
                schedule.next_fire_at = next_fire(schedule, now);
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
    use mocks::MockJobStore;

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
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(60, now));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        assert_eq!(store.list().await.unwrap().len(), 1);

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 0);
        assert_eq!(store.list().await.unwrap().len(), 1);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn a_schedule_hours_behind_fires_once_and_skips_the_runs_it_missed() {
        let now = Timestamp::now();
        let every = SignedDuration::from_hours(1);
        let behind = now.checked_sub(SignedDuration::from_mins(620)).unwrap();
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(every.as_secs(), behind));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        let next = behind.checked_add(SignedDuration::from_hours(11)).unwrap();
        assert_eq!(scheduler.schedules[0].next_fire_at, next);

        let next_tick = now.checked_add(SignedDuration::from_secs(30)).unwrap();
        assert_eq!(scheduler.tick(next_tick, &queue).await.unwrap(), 0, "no replay after a jump");
        assert_eq!(store.list().await.unwrap().len(), 1);
        assert!(logs_contain("schedule [LibraryScan] was due at"));
    }

    #[tokio::test]
    async fn a_daily_run_slept_through_keeps_its_time_of_day() {
        let now = Timestamp::now();
        let day = SignedDuration::from_hours(24);
        let due = now.checked_sub(SignedDuration::from_mins(2 * 24 * 60 + 270)).unwrap();
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(day.as_secs(), due));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        let next = due.checked_add(SignedDuration::from_hours(72)).unwrap();
        assert_eq!(scheduler.schedules[0].next_fire_at, next);
        assert_eq!(store.list().await.unwrap().len(), 1);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn a_schedule_without_a_positive_interval_reports_no_missed_runs() {
        let now = Timestamp::now();
        let queue = JobQueue::new(MockJobStore::new());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(0, now));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        assert_eq!(scheduler.schedules[0].next_fire_at, now);
        assert!(!logs_contain("missed the runs"));
    }

    #[tokio::test]
    async fn a_run_fired_a_tick_late_keeps_its_cadence() {
        let now = Timestamp::now();
        let every = SignedDuration::from_hours(1);
        let due = now.checked_sub(SignedDuration::from_secs(30)).unwrap();
        let queue = JobQueue::new(MockJobStore::new());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(every.as_secs(), due));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 1);
        assert_eq!(scheduler.schedules[0].next_fire_at, due.checked_add(every).unwrap());
    }

    #[tokio::test]
    async fn tick_skips_not_due_schedule() {
        let now = Timestamp::now();
        let future = now.saturating_add(SignedDuration::from_secs(60)).unwrap();
        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());
        let mut scheduler = Scheduler::new();
        scheduler.register(schedule(60, future));

        assert_eq!(scheduler.tick(now, &queue).await.unwrap(), 0);
        assert!(store.list().await.unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn run_ticks_until_shutdown() {
        use std::time::Duration;
        use tokio::sync::oneshot;

        let store = MockJobStore::new();
        let queue = JobQueue::new(store.clone());
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

        assert!(!store.list().await.unwrap().is_empty());
    }
}
