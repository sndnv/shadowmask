use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaylistId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
