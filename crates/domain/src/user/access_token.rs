#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_in_s: u32,
}
