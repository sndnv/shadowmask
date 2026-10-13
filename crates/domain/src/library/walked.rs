use crate::library::WalkedEntry;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walked {
    pub entries: Vec<WalkedEntry>,
    pub unreadable: Vec<String>,
}
