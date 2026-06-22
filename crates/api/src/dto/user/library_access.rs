use serde::Serialize;

use domain::user::LibraryAccess;

#[derive(Debug, Serialize)]
pub struct LibraryAccessResponse {
    pub user_id: String,
    pub library_id: String,
}

impl From<LibraryAccess> for LibraryAccessResponse {
    fn from(a: LibraryAccess) -> Self {
        LibraryAccessResponse {
            user_id: a.user.0,
            library_id: a.library.0,
        }
    }
}
