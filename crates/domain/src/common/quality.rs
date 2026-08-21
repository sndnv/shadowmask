#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    Sd,
    Hd,
    Fhd,
    Uhd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_ranks_from_sd_up_to_uhd() {
        let mut all = [Quality::Fhd, Quality::Sd, Quality::Uhd, Quality::Hd];
        all.sort();
        assert_eq!(all, [Quality::Sd, Quality::Hd, Quality::Fhd, Quality::Uhd]);
    }
}
