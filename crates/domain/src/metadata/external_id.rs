#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalId {
    pub source: String,
    pub value: String,
}
