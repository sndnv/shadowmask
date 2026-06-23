use serde::{Deserialize, Serialize};

use domain::user::Role;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleDto {
    Admin,
    User,
    Player,
}

impl From<Role> for RoleDto {
    fn from(r: Role) -> Self {
        match r {
            Role::Admin => RoleDto::Admin,
            Role::User => RoleDto::User,
            Role::Player => RoleDto::Player,
        }
    }
}

impl From<RoleDto> for Role {
    fn from(r: RoleDto) -> Self {
        match r {
            RoleDto::Admin => Role::Admin,
            RoleDto::User => Role::User,
            RoleDto::Player => Role::Player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_roles_both_ways() {
        assert!(matches!(RoleDto::from(Role::Admin), RoleDto::Admin));
        assert!(matches!(RoleDto::from(Role::User), RoleDto::User));
        assert!(matches!(RoleDto::from(Role::Player), RoleDto::Player));
        assert!(matches!(Role::from(RoleDto::Admin), Role::Admin));
        assert!(matches!(Role::from(RoleDto::User), Role::User));
        assert!(matches!(Role::from(RoleDto::Player), Role::Player));
    }
}
