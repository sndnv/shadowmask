use crate::catalog::CollectionId;
use crate::playback::WatchTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaylistTarget {
    Title(WatchTarget),
    Collection(CollectionId),
}
