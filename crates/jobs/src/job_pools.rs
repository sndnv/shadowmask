use std::collections::BTreeMap;

use domain::job::JobKind;

pub const DEFAULT_POOL: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRequest {
    pub concurrency: usize,
    pub kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPool {
    pub name: String,
    pub concurrency: usize,
    pub kinds: Vec<JobKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PoolError {
    #[error("job pool [{pool}] lists unknown job kind [{kind}]; known kinds are [{known}]")]
    UnknownKind { pool: String, kind: String, known: String },
    #[error(
        "job kind [{kind}] is claimed by both pool [{first}] and pool [{second}]; a kind runs on one pool"
    )]
    DuplicateKind { kind: String, first: String, second: String },
    #[error("job pool [{pool}] has a concurrency of zero and would never run anything")]
    ZeroConcurrency { pool: String },
    #[error("no [default] job pool is configured; it is where every unassigned kind runs")]
    MissingDefault,
}

pub fn default_pools() -> BTreeMap<String, PoolRequest> {
    BTreeMap::from([
        (DEFAULT_POOL.to_owned(), PoolRequest { concurrency: 4, kinds: Vec::new() }),
        (
            "trickplay".to_owned(),
            PoolRequest { concurrency: 1, kinds: vec![JobKind::Trickplay.slug().to_owned()] },
        ),
        (
            "enrichment".to_owned(),
            PoolRequest {
                concurrency: 1,
                kinds: vec![
                    JobKind::Transcription.slug().to_owned(),
                    JobKind::Translation.slug().to_owned(),
                    JobKind::Upscale.slug().to_owned(),
                ],
            },
        ),
        (
            "fetch".to_owned(),
            PoolRequest { concurrency: 1, kinds: vec![JobKind::Fetch.slug().to_owned()] },
        ),
    ])
}

pub fn resolve_pools(
    requested: &BTreeMap<String, PoolRequest>,
    parked: &[JobKind],
) -> Result<Vec<JobPool>, PoolError> {
    if !requested.contains_key(DEFAULT_POOL) {
        return Err(PoolError::MissingDefault);
    }
    let mut claimed: BTreeMap<&'static str, String> = BTreeMap::new();
    let mut pools: Vec<JobPool> = Vec::new();
    for (name, request) in requested {
        if request.concurrency == 0 {
            return Err(PoolError::ZeroConcurrency { pool: name.clone() });
        }
        let mut kinds = Vec::new();
        for slug in &request.kinds {
            let kind = JobKind::from_slug(slug).ok_or_else(|| PoolError::UnknownKind {
                pool: name.clone(),
                kind: slug.clone(),
                known: known_kinds(),
            })?;
            if let Some(first) = claimed.get(kind.slug()) {
                return Err(PoolError::DuplicateKind {
                    kind: kind.slug().to_owned(),
                    first: first.clone(),
                    second: name.clone(),
                });
            }
            claimed.insert(kind.slug(), name.clone());
            if !parked.contains(&kind) {
                kinds.push(kind);
            }
        }
        pools.push(JobPool { name: name.clone(), concurrency: request.concurrency, kinds });
    }
    let unassigned = JobKind::ALL
        .into_iter()
        .filter(|kind| !claimed.contains_key(kind.slug()))
        .filter(|kind| !parked.contains(kind));
    if let Some(default) = pools.iter_mut().find(|pool| pool.name == DEFAULT_POOL) {
        default.kinds.extend(unassigned);
    }
    pools.retain(|pool| pool.name == DEFAULT_POOL || !pool.kinds.is_empty());
    Ok(pools)
}

fn known_kinds() -> String {
    JobKind::ALL.iter().map(|kind| kind.slug()).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(concurrency: usize, kinds: &[&str]) -> PoolRequest {
        PoolRequest { concurrency, kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect() }
    }

    fn pool<'a>(pools: &'a [JobPool], name: &str) -> &'a JobPool {
        pools.iter().find(|pool| pool.name == name).expect("pool")
    }

    #[test]
    fn the_shipped_defaults_cover_every_kind_exactly_once() {
        let pools = resolve_pools(&default_pools(), &[]).expect("resolve");

        assert_eq!(pools.len(), 4);
        assert_eq!(pool(&pools, "trickplay").kinds, vec![JobKind::Trickplay]);
        assert_eq!(pool(&pools, "fetch").kinds, vec![JobKind::Fetch]);
        assert_eq!(
            pool(&pools, "enrichment").kinds,
            vec![JobKind::Transcription, JobKind::Translation, JobKind::Upscale]
        );
        let total: usize = pools.iter().map(|pool| pool.kinds.len()).sum();
        assert_eq!(total, JobKind::ALL.len());
        assert_eq!(pool(&pools, DEFAULT_POOL).kinds.len(), JobKind::ALL.len() - 5);
        assert_eq!(pool(&pools, DEFAULT_POOL).concurrency, 4);
    }

    #[test]
    fn a_kind_named_by_no_pool_runs_on_the_default_pool() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("heavy".to_owned(), request(1, &["upscale"])),
        ]);

        let pools = resolve_pools(&requested, &[]).expect("resolve");

        assert_eq!(pools.len(), 2);
        assert!(pool(&pools, DEFAULT_POOL).kinds.contains(&JobKind::Trickplay));
        assert!(!pool(&pools, DEFAULT_POOL).kinds.contains(&JobKind::Upscale));
    }

    #[test]
    fn kinds_listed_on_the_default_pool_stay_there_without_duplicating() {
        let requested = BTreeMap::from([(DEFAULT_POOL.to_owned(), request(2, &["trickplay"]))]);

        let pools = resolve_pools(&requested, &[]).expect("resolve");

        let kinds = &pool(&pools, DEFAULT_POOL).kinds;
        assert_eq!(kinds.len(), JobKind::ALL.len());
        assert_eq!(kinds.iter().filter(|kind| **kind == JobKind::Trickplay).count(), 1);
    }

    #[test]
    fn a_pool_with_no_kinds_of_its_own_is_dropped() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("trickplay".to_owned(), request(1, &[])),
        ]);

        let pools = resolve_pools(&requested, &[]).expect("resolve");

        assert_eq!(pools.len(), 1);
        assert!(pool(&pools, DEFAULT_POOL).kinds.contains(&JobKind::Trickplay));
    }

    #[test]
    fn a_parked_kind_is_claimed_by_no_pool_at_all() {
        let pools = resolve_pools(&default_pools(), &[JobKind::Transcription, JobKind::Artwork])
            .expect("resolve");

        assert!(pools.iter().all(|pool| !pool.kinds.contains(&JobKind::Transcription)));
        assert!(pools.iter().all(|pool| !pool.kinds.contains(&JobKind::Artwork)));
        assert_eq!(pool(&pools, "enrichment").kinds, vec![JobKind::Translation, JobKind::Upscale]);
        let total: usize = pools.iter().map(|pool| pool.kinds.len()).sum();
        assert_eq!(total, JobKind::ALL.len() - 2);
    }

    #[test]
    fn a_pool_whose_only_kind_is_parked_is_dropped() {
        let pools = resolve_pools(&default_pools(), &[JobKind::Trickplay]).expect("resolve");

        assert!(pools.iter().all(|pool| pool.name != "trickplay"));
    }

    #[test]
    fn parking_a_kind_does_not_excuse_claiming_it_twice() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("a_pool".to_owned(), request(1, &["upscale"])),
            ("b_pool".to_owned(), request(1, &["upscale"])),
        ]);

        let error = resolve_pools(&requested, &[JobKind::Upscale]).unwrap_err();

        assert!(matches!(error, PoolError::DuplicateKind { .. }));
    }

    #[test]
    fn an_unknown_kind_names_itself_and_the_known_ones() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("typo".to_owned(), request(1, &["trikplay"])),
        ]);

        let error = resolve_pools(&requested, &[]).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("trikplay"), "{message}");
        assert!(message.contains("trickplay"), "{message}");
        assert!(message.contains("typo"), "{message}");
    }

    #[test]
    fn a_kind_claimed_twice_names_both_pools() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("a_pool".to_owned(), request(1, &["trickplay"])),
            ("b_pool".to_owned(), request(1, &["trickplay"])),
        ]);

        let error = resolve_pools(&requested, &[]).unwrap_err();

        assert_eq!(
            error,
            PoolError::DuplicateKind {
                kind: "trickplay".to_owned(),
                first: "a_pool".to_owned(),
                second: "b_pool".to_owned(),
            }
        );
    }

    #[test]
    fn a_pool_that_can_never_run_anything_is_rejected() {
        let requested = BTreeMap::from([
            (DEFAULT_POOL.to_owned(), request(2, &[])),
            ("idle".to_owned(), request(0, &["trickplay"])),
        ]);

        assert_eq!(
            resolve_pools(&requested, &[]).unwrap_err(),
            PoolError::ZeroConcurrency { pool: "idle".to_owned() }
        );
    }

    #[test]
    fn losing_the_default_pool_is_rejected() {
        let requested = BTreeMap::from([("trickplay".to_owned(), request(1, &["trickplay"]))]);

        assert_eq!(resolve_pools(&requested, &[]).unwrap_err(), PoolError::MissingDefault);
    }
}
