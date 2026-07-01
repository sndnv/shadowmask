use crate::catalog::VersionId;
use crate::media::Fingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintedVersion {
    pub version: VersionId,
    pub fingerprint: Fingerprint,
}
