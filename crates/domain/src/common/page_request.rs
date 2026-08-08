#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    pub offset: u32,
    pub limit: u32,
}

impl PageRequest {
    pub const ALL: Self = Self {
        offset: 0,
        limit: u32::MAX,
    };
}
