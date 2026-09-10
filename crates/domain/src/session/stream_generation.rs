#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamGeneration(pub u32);

impl StreamGeneration {
    pub fn dir_name(self) -> String {
        self.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generations_order_by_number() {
        assert!(StreamGeneration(2) > StreamGeneration(1));
        assert_eq!(
            [StreamGeneration(3), StreamGeneration(1)].into_iter().max(),
            Some(StreamGeneration(3))
        );
    }

    #[test]
    fn the_directory_name_is_the_bare_number() {
        assert_eq!(StreamGeneration(0).dir_name(), "0");
        assert_eq!(StreamGeneration(42).dir_name(), "42");
    }
}
