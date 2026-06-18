use crate::user::Role;

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub role: Role,
}
