use crate::user::UserId;

#[derive(Debug, Clone)]
pub struct Audience {
    pub user: UserId,
    pub age_limit: Option<u8>,
}
