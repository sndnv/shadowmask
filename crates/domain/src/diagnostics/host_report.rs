#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostReport {
    pub cpu: Option<String>,
    pub cores: Option<usize>,
    pub threads: usize,
}
