use isolang::Language;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageCode(pub String);

const BIBLIOGRAPHIC_TO_TERMINOLOGY: [(&str, &str); 20] = [
    ("alb", "sqi"),
    ("arm", "hye"),
    ("baq", "eus"),
    ("bur", "mya"),
    ("chi", "zho"),
    ("cze", "ces"),
    ("dut", "nld"),
    ("fre", "fra"),
    ("geo", "kat"),
    ("ger", "deu"),
    ("gre", "ell"),
    ("ice", "isl"),
    ("mac", "mkd"),
    ("mao", "mri"),
    ("may", "msa"),
    ("per", "fas"),
    ("rum", "ron"),
    ("slo", "slk"),
    ("tib", "bod"),
    ("wel", "cym"),
];

impl LanguageCode {
    pub fn from_file_tag(tag: &str) -> Self {
        let lower = tag.to_ascii_lowercase();
        let terminology = BIBLIOGRAPHIC_TO_TERMINOLOGY
            .iter()
            .find(|(bibliographic, _)| *bibliographic == lower)
            .map_or(lower.as_str(), |(_, terminology)| terminology);
        let two_letter = Language::from_639_3(terminology).and_then(|language| language.to_639_1());
        Self(two_letter.map_or_else(|| tag.to_owned(), str::to_owned))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(raw: &str) -> String {
        LanguageCode::from_file_tag(raw).0
    }

    #[test]
    fn a_three_letter_tag_becomes_two_letters() {
        assert_eq!(tag("eng"), "en");
        assert_eq!(tag("spa"), "es");
        assert_eq!(tag("ENG"), "en");
    }

    #[test]
    fn a_bibliographic_tag_becomes_two_letters() {
        assert_eq!(tag("fre"), "fr");
        assert_eq!(tag("ger"), "de");
        assert_eq!(tag("chi"), "zh");
    }

    #[test]
    fn every_bibliographic_tag_has_a_two_letter_form() {
        for (bibliographic, _) in BIBLIOGRAPHIC_TO_TERMINOLOGY {
            assert_eq!(tag(bibliographic).len(), 2, "{bibliographic}");
        }
    }

    #[test]
    fn a_tag_without_a_two_letter_form_is_kept() {
        assert_eq!(tag("fil"), "fil");
        assert_eq!(tag("und"), "und");
        assert_eq!(tag("xyz"), "xyz");
    }

    #[test]
    fn a_two_letter_or_regional_tag_is_kept() {
        assert_eq!(tag("en"), "en");
        assert_eq!(tag("pt-BR"), "pt-BR");
    }
}
