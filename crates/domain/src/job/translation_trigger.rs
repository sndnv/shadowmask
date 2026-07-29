use std::future::Future;

use crate::catalog::VersionId;
use crate::error::RepositoryError;
use crate::job::JobId;

pub trait TranslationTrigger {
    fn trigger(
        &self,
        version_id: &VersionId,
        parent: Option<&JobId>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}
