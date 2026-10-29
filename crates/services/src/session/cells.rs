use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::SessionError;
use domain::session::SessionId;
use futures::future::{AbortHandle, abortable};
use futures::lock::{Mutex as CellLock, MutexGuard};

use super::LaunchContext;

pub(super) struct Live {
    pub(super) ctx: LaunchContext,
    pub(super) generation: u32,
    pub(super) ended: bool,
}

#[derive(Default)]
pub(super) struct SessionCells {
    cells: Mutex<HashMap<SessionId, Arc<Cell>>>,
}

struct Cell {
    live: CellLock<Live>,
    stop: Mutex<Stop>,
}

#[derive(Default)]
struct Stop {
    ending: bool,
    running: Option<AbortHandle>,
}

impl Cell {
    fn new(live: Live) -> Self {
        Self { live: CellLock::new(live), stop: Mutex::default() }
    }

    fn arm(&self, running: AbortHandle) -> Result<(), SessionError> {
        let mut stop = self.stop.lock().unwrap();
        if stop.ending {
            return Err(SessionError::NotFound);
        }
        stop.running = Some(running);
        Ok(())
    }

    fn halt(&self) {
        let mut stop = self.stop.lock().unwrap();
        stop.ending = true;
        if let Some(running) = stop.running.take() {
            running.abort();
        }
    }

    fn ending(&self) -> bool {
        self.stop.lock().unwrap().ending
    }
}

struct Turn<'a> {
    cells: &'a SessionCells,
    id: &'a SessionId,
    live: MutexGuard<'a, Live>,
    kept: bool,
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        if !self.kept {
            self.live.ended = true;
        }
        if self.live.ended {
            self.cells.cells.lock().unwrap().remove(self.id);
        }
    }
}

impl SessionCells {
    pub(super) async fn create<T>(
        &self,
        id: &SessionId,
        live: Live,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let cell = Arc::new(Cell::new(live));
        let live = cell.live.lock().await;
        self.cells.lock().unwrap().insert(id.clone(), Arc::clone(&cell));
        let mut turn = Turn { cells: self, id, live, kept: false };
        let result = op(&mut turn.live).await;
        turn.kept = result.is_ok();
        result
    }

    pub(super) async fn run<T>(
        &self,
        id: &SessionId,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let cell = self.cell(id).ok_or(SessionError::NotFound)?;
        self.within(id, cell.live.lock().await, op).await
    }

    pub(super) async fn run_abortable<T>(
        &self,
        id: &SessionId,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let cell = self.cell(id).ok_or(SessionError::NotFound)?;
        let mut turn = self.enter(id, cell.live.lock().await)?;
        let (op, running) = abortable(op(&mut turn.live));
        cell.arm(running)?;
        op.await.unwrap_or(Err(SessionError::NotFound))
    }

    pub(super) async fn try_run<T>(
        &self,
        id: &SessionId,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Option<Result<T, SessionError>> {
        let Some(cell) = self.cell(id) else {
            return Some(Err(SessionError::NotFound));
        };
        let live = cell.live.try_lock().filter(|_| !cell.ending())?;
        Some(self.within(id, live, op).await)
    }

    pub(super) async fn end<T>(
        &self,
        id: &SessionId,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let cell = self.cell(id).ok_or(SessionError::NotFound)?;
        cell.halt();
        let mut turn = self.enter(id, cell.live.lock().await)?;
        turn.kept = false;
        op(&mut turn.live).await
    }

    fn cell(&self, id: &SessionId) -> Option<Arc<Cell>> {
        self.cells.lock().unwrap().get(id).cloned()
    }

    async fn within<T>(
        &self,
        id: &SessionId,
        live: MutexGuard<'_, Live>,
        op: impl AsyncFnOnce(&mut Live) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let mut turn = self.enter(id, live)?;
        op(&mut turn.live).await
    }

    fn enter<'a>(
        &'a self,
        id: &'a SessionId,
        live: MutexGuard<'a, Live>,
    ) -> Result<Turn<'a>, SessionError> {
        let turn = Turn { cells: self, id, live, kept: true };
        if turn.live.ended {
            return Err(SessionError::NotFound);
        }
        Ok(turn)
    }

    #[cfg(test)]
    pub(super) fn holds(&self, id: &SessionId) -> bool {
        self.cells.lock().unwrap().contains_key(id)
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.cells.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::VersionId;
    use domain::session::{ClientCapabilities, DeliveryPreference};
    use domain::user::UserId;
    use futures::channel::oneshot;
    use futures::{FutureExt, future};
    use std::sync::LazyLock;

    fn live() -> Live {
        Live {
            ctx: LaunchContext {
                user: UserId("u1".to_owned()),
                version: VersionId("v1".to_owned()),
                capabilities: ClientCapabilities {
                    platform: "web".to_owned(),
                    profile_version: 1,
                    max_bitrate: None,
                    decoding: None,
                    tracks: domain::session::TrackHandling::default(),
                },
                requested_audio: None,
                requested_subtitle: None,
                remembered_audio: None,
                remembered_subtitle: None,
                bitrate_cap: None,
                target_height: None,
                force_burn: false,
                downmix_stereo: false,
                sound: domain::session::SoundSettings::default(),
                delivery: DeliveryPreference::Auto,
            },
            generation: 1,
            ended: false,
        }
    }

    static ID: LazyLock<SessionId> = LazyLock::new(|| SessionId("s1".to_owned()));

    async fn noop(_: &mut Live) -> Result<(), SessionError> {
        Ok(())
    }

    async fn generation(live: &mut Live) -> Result<u32, SessionError> {
        if live.generation == 0 {
            future::pending::<()>().await;
        }
        Ok(live.generation)
    }

    async fn end_session(live: &mut Live) -> Result<(), SessionError> {
        live.ended = true;
        generation(live).await.map(drop)
    }

    fn stuck() -> Live {
        Live { generation: 0, ..live() }
    }

    async fn opened() -> SessionCells {
        let cells = SessionCells::default();
        cells.create(&ID, live(), generation).await.unwrap();
        cells
    }

    #[tokio::test]
    async fn an_operation_waits_for_the_one_in_flight() {
        let cells = opened().await;
        let (release, released) = oneshot::channel::<()>();
        let seen = Mutex::new(Vec::new());

        let (first, second, ()) = futures::join!(
            cells.run(&ID, async |live| {
                released.await.unwrap();
                live.generation += 1;
                seen.lock().unwrap().push(("first", live.generation));
                Ok(())
            }),
            cells.run(&ID, async |live| {
                live.generation += 1;
                seen.lock().unwrap().push(("second", live.generation));
                Ok(())
            }),
            async { release.send(()).unwrap() },
        );

        assert!(first.is_ok() && second.is_ok());
        assert_eq!(*seen.lock().unwrap(), vec![("first", 2), ("second", 3)]);
    }

    #[tokio::test]
    async fn a_session_that_is_not_there_is_not_found() {
        let cells = SessionCells::default();

        let ran = cells.run(&ID, noop).await;
        let tried = cells.try_run(&ID, noop).await;

        assert!(matches!(ran, Err(SessionError::NotFound)));
        assert!(matches!(tried, Some(Err(SessionError::NotFound))));
    }

    #[tokio::test]
    async fn an_operation_waiting_on_a_session_that_ends_finds_it_gone() {
        let cells = opened().await;
        let (release, released) = oneshot::channel::<()>();

        let (ending, waiting, ()) = futures::join!(
            cells.run(&ID, async |live| {
                released.await.unwrap();
                live.ended = true;
                Ok(())
            }),
            cells.run(&ID, generation),
            async { release.send(()).unwrap() },
        );

        assert!(ending.is_ok());
        assert!(
            matches!(waiting, Err(SessionError::NotFound)),
            "nothing runs on a session after its end"
        );
        assert!(cells.is_empty());
    }

    #[tokio::test]
    async fn nothing_runs_on_a_session_before_its_create_finishes() {
        let cells = SessionCells::default();
        let (release, released) = oneshot::channel::<()>();

        let (created, busy, generation, ()) = futures::join!(
            cells.create(&ID, live(), async |live| {
                released.await.unwrap();
                live.generation = 5;
                Ok(())
            }),
            cells.try_run(&ID, noop),
            cells.run(&ID, generation),
            async { release.send(()).unwrap() },
        );

        assert!(created.is_ok());
        assert!(busy.is_none(), "a session still starting is busy");
        assert_eq!(generation.unwrap(), 5);
    }

    #[tokio::test]
    async fn a_create_that_fails_leaves_nothing_behind() {
        let cells = SessionCells::default();

        let created: Result<(), SessionError> =
            cells.create(&ID, live(), async |_| Err(SessionError::NegotiationFailed)).await;

        assert!(matches!(created, Err(SessionError::NegotiationFailed)));
        assert!(!cells.holds(&ID));
        assert!(matches!(cells.run(&ID, noop).await, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn an_operation_that_fails_keeps_the_session() {
        let cells = opened().await;

        let failed: Result<(), SessionError> =
            cells.run(&ID, async |_| Err(SessionError::NegotiationFailed)).await;

        assert!(matches!(failed, Err(SessionError::NegotiationFailed)));
        assert!(cells.run(&ID, noop).await.is_ok(), "the session is still there");
    }

    #[tokio::test]
    async fn a_busy_session_is_skipped_by_a_try_and_taken_once_free() {
        let cells = opened().await;
        let (release, released) = oneshot::channel::<()>();

        let (held, busy, ()) = futures::join!(
            cells.run(&ID, async |_| {
                released.await.unwrap();
                Ok(())
            }),
            cells.try_run(&ID, noop),
            async { release.send(()).unwrap() },
        );
        let free = cells.try_run(&ID, noop).await;

        assert!(held.is_ok());
        assert!(busy.is_none());
        assert!(matches!(free, Some(Ok(()))));
    }

    #[tokio::test]
    async fn an_end_dropped_part_way_still_clears_the_session() {
        let cells = SessionCells::default();
        cells.create(&ID, stuck(), noop).await.unwrap();

        let ending = cells.run(&ID, end_session);

        assert!(ending.now_or_never().is_none());
        assert!(!cells.holds(&ID));
    }

    #[tokio::test]
    async fn an_operation_dropped_part_way_keeps_the_session() {
        let cells = SessionCells::default();
        cells.create(&ID, stuck(), noop).await.unwrap();

        let waiting = cells.run(&ID, generation);

        assert!(waiting.now_or_never().is_none());
        assert!(cells.run(&ID, noop).await.is_ok(), "the session is still there");
    }

    #[tokio::test]
    async fn a_create_dropped_part_way_leaves_nothing_behind() {
        let cells = SessionCells::default();

        let creating = cells.create(&ID, stuck(), generation);

        assert!(creating.now_or_never().is_none());
        assert!(!cells.holds(&ID));
    }

    #[tokio::test]
    async fn a_try_on_an_ended_session_is_not_found() {
        let cells = opened().await;
        cells.run(&ID, end_session).await.unwrap();

        let tried = cells.try_run(&ID, noop).await;

        assert!(matches!(tried, Some(Err(SessionError::NotFound))));
    }

    #[tokio::test]
    async fn an_end_cancels_the_abortable_operation_holding_the_session() {
        let cells = SessionCells::default();
        cells.create(&ID, stuck(), noop).await.unwrap();

        let (held, ended) =
            futures::join!(cells.run_abortable(&ID, generation), cells.end(&ID, noop));

        assert!(matches!(held, Err(SessionError::NotFound)), "the stuck operation was cancelled");
        assert!(ended.is_ok());
        assert!(cells.is_empty());
    }

    #[tokio::test]
    async fn an_end_waits_for_an_operation_it_may_not_cancel_and_refuses_what_comes_after() {
        let cells = opened().await;
        let (release, released) = oneshot::channel::<()>();

        let (held, late, ended, ()) = futures::join!(
            cells.run(&ID, async |_| {
                released.await.unwrap();
                Ok(())
            }),
            cells.run_abortable(&ID, generation),
            cells.end(&ID, noop),
            async { release.send(()).unwrap() },
        );

        assert!(held.is_ok(), "the end waited for it");
        assert!(ended.is_ok());
        assert!(matches!(late, Err(SessionError::NotFound)), "nothing launches after an end");
        assert!(cells.is_empty());
    }

    #[tokio::test]
    async fn a_second_end_does_not_cancel_the_first() {
        let cells = opened().await;
        let (release, released) = oneshot::channel::<()>();

        let (first, second, ()) = futures::join!(
            cells.end(&ID, async |_| {
                released.await.unwrap();
                Ok(())
            }),
            cells.end(&ID, noop),
            async { release.send(()).unwrap() },
        );

        assert!(first.is_ok(), "the end in flight runs to its end");
        assert!(matches!(second, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn an_ending_session_refuses_a_new_abortable_operation_and_is_passed_over_by_a_try() {
        let cells = opened().await;
        let cell = cells.cell(&ID).unwrap();
        let (running, _) = AbortHandle::new_pair();

        cell.halt();

        assert!(matches!(cell.arm(running), Err(SessionError::NotFound)));
        assert!(cells.try_run(&ID, noop).await.is_none(), "the end will finish it");
    }
}
