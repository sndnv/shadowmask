use jiff::Timestamp;

#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: Option<Timestamp>,
}
