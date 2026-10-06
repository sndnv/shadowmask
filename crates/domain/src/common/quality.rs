#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    Sd,
    Hd,
    Fhd,
    Uhd,
}

impl Quality {
    pub fn slug(&self) -> &'static str {
        match self {
            Quality::Sd => "sd",
            Quality::Hd => "hd",
            Quality::Fhd => "fhd",
            Quality::Uhd => "uhd",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_quality_has_its_short_name() {
        let slugs = [Quality::Sd, Quality::Hd, Quality::Fhd, Quality::Uhd].map(|q| q.slug());
        assert_eq!(slugs, ["sd", "hd", "fhd", "uhd"]);
    }

    #[test]
    fn quality_ranks_from_sd_up_to_uhd() {
        let mut all = [Quality::Fhd, Quality::Sd, Quality::Uhd, Quality::Hd];
        all.sort();
        assert_eq!(all, [Quality::Sd, Quality::Hd, Quality::Fhd, Quality::Uhd]);
    }
}
