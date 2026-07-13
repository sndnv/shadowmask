use crate::catalog::TitleId;
use crate::metadata::ExternalId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveTarget {
    Existing(TitleId),
    Provider(ExternalId),
}
