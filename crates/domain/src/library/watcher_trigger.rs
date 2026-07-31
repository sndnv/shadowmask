use crate::error::WatchPlanError;
use crate::library::{Library, WatchPlan, WatcherStrategy};

pub fn plan_for(library: &Library) -> Result<WatchPlan, WatchPlanError> {
    match library.watcher {
        WatcherStrategy::Local => {
            require_roots(library)?;
            Ok(WatchPlan::FsEvents {
                roots: library.roots.clone(),
            })
        }
        WatcherStrategy::Polling => {
            require_roots(library)?;
            Ok(WatchPlan::Poll {
                roots: library.roots.clone(),
            })
        }
        WatcherStrategy::Scheduled => {
            require_roots(library)?;
            let expression = library
                .scan_schedule
                .clone()
                .ok_or(WatchPlanError::MissingSchedule)?;
            Ok(WatchPlan::Cron {
                expression,
                roots: library.roots.clone(),
            })
        }
        WatcherStrategy::Manual => Ok(WatchPlan::Manual),
    }
}

fn require_roots(library: &Library) -> Result<(), WatchPlanError> {
    if library.roots.is_empty() {
        Err(WatchPlanError::NoRoots)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{LibraryId, LibraryKind, LibraryOrigin};
    use jiff::Timestamp;

    fn library(watcher: WatcherStrategy, roots: &[&str], schedule: Option<&str>) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            kind: LibraryKind::Movie,
            origin: LibraryOrigin::Local,
            roots: roots.iter().map(|r| (*r).into()).collect(),
            watcher,
            scan_schedule: schedule.map(Into::into),
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn local_with_roots_is_fs_events() {
        let plan = plan_for(&library(WatcherStrategy::Local, &["/m"], None)).unwrap();
        assert_eq!(
            plan,
            WatchPlan::FsEvents {
                roots: vec!["/m".into()]
            }
        );
    }

    #[test]
    fn polling_with_roots_is_poll() {
        let plan = plan_for(&library(WatcherStrategy::Polling, &["/m"], None)).unwrap();
        assert_eq!(
            plan,
            WatchPlan::Poll {
                roots: vec!["/m".into()]
            }
        );
    }

    #[test]
    fn scheduled_with_schedule_is_cron() {
        let plan = plan_for(&library(
            WatcherStrategy::Scheduled,
            &["/m"],
            Some("0 0 3 * * * *"),
        ))
        .unwrap();
        assert_eq!(
            plan,
            WatchPlan::Cron {
                expression: "0 0 3 * * * *".into(),
                roots: vec!["/m".into()]
            }
        );
    }

    #[test]
    fn scheduled_without_schedule_is_missing_schedule() {
        let err = plan_for(&library(WatcherStrategy::Scheduled, &["/m"], None)).unwrap_err();
        assert_eq!(err, WatchPlanError::MissingSchedule);
    }

    #[test]
    fn manual_is_always_valid_even_without_roots() {
        let plan = plan_for(&library(WatcherStrategy::Manual, &[], None)).unwrap();
        assert_eq!(plan, WatchPlan::Manual);
    }

    #[test]
    fn event_strategies_require_roots() {
        for watcher in [
            WatcherStrategy::Local,
            WatcherStrategy::Polling,
            WatcherStrategy::Scheduled,
        ] {
            let err = plan_for(&library(watcher, &[], Some("0 0 3 * * * *"))).unwrap_err();
            assert_eq!(err, WatchPlanError::NoRoots);
        }
    }
}
