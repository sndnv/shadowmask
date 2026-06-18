#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    pub offset: u32,
    pub limit: u32,
}
