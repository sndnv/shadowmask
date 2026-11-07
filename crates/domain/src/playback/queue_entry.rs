use crate::catalog::TitleId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueueEntryId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub title: TitleId,
}
