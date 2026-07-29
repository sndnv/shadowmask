use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Capability {
    pub name: String,
    pub available: bool,
    pub enabled: bool,
}

impl Capability {
    pub fn new(name: impl Into<String>, available: bool, enabled: bool) -> Self {
        Self {
            name: name.into(),
            available,
            enabled,
        }
    }
}
