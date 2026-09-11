use serde::Deserialize;

use domain::user::NewUser;

use super::RoleDto;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: RoleDto,
}

impl From<CreateUserRequest> for NewUser {
    fn from(r: CreateUserRequest) -> Self {
        NewUser { username: r.username, password: r.password, role: r.role.into() }
    }
}
