use crate::library::LibraryId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LibraryAccess {
    pub user: UserId,
    pub library: LibraryId,
}
