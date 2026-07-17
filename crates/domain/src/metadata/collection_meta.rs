use crate::metadata::{Artwork, ExternalId};

#[derive(Debug, Clone)]
pub struct CollectionMeta {
    pub external_id: ExternalId,
    pub name: String,
    pub artwork: Vec<Artwork>,
}
