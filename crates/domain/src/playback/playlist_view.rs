use crate::catalog::TitleCard;
use crate::playback::{Playlist, PlaylistItem, QueueEntry};

#[derive(Debug, Clone)]
pub struct PlaylistSummary {
    pub playlist: Playlist,
    pub item_count: usize,
}

#[derive(Debug, Clone)]
pub struct PlaylistCard {
    pub item: PlaylistItem,
    pub card: TitleCard,
}

#[derive(Debug, Clone)]
pub struct PlaylistDetail {
    pub playlist: Playlist,
    pub items: Vec<PlaylistCard>,
}

#[derive(Debug, Clone)]
pub struct QueueCard {
    pub entry: QueueEntry,
    pub card: TitleCard,
}
