use std::collections::HashSet;

use domain::catalog::{TitleSet, VersionId};
use domain::error::RepositoryError;
use sqlx::SqlitePool;

use crate::pool::backend;

pub(super) struct Forget<'a> {
    pub(super) by_title: &'a [&'static str],
    pub(super) by_version: &'a [&'static str],
}

impl Forget<'_> {
    pub(super) async fn run(
        &self,
        pool: &SqlitePool,
        titles: &TitleSet,
        versions: &HashSet<VersionId>,
    ) -> Result<(), RepositoryError> {
        let mut tx = pool.begin().await.map_err(backend)?;
        let rows = titles
            .movies
            .iter()
            .map(|id| ("movie", id.0.as_str()))
            .chain(titles.episodes.iter().map(|id| ("episode", id.0.as_str())));
        for (kind, id) in rows {
            for sql in self.by_title {
                sqlx::query(*sql).bind(kind).bind(id).execute(&mut *tx).await.map_err(backend)?;
            }
        }
        for version in versions {
            for sql in self.by_version {
                sqlx::query(*sql)
                    .bind(version.0.as_str())
                    .execute(&mut *tx)
                    .await
                    .map_err(backend)?;
            }
        }
        tx.commit().await.map_err(backend)
    }
}
