use isolang::Language;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageCode(pub String);

const BIBLIOGRAPHIC_TO_TERMINOLOGY: [(&str, &str); 22] = [
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
    ("scc", "srp"),
    ("scr", "hrv"),
    ("slo", "slk"),
    ("tib", "bod"),
    ("wel", "cym"),
];

impl LanguageCode {
    pub fn canonical(tag: &str) -> Self {
        if let Some(regional) = Self::from_regional_tag(tag) {
            return regional;
        }
        let lower = tag.to_ascii_lowercase();
        let terminology = BIBLIOGRAPHIC_TO_TERMINOLOGY
            .iter()
            .find(|(bibliographic, _)| *bibliographic == lower)
            .map_or(lower.as_str(), |(_, terminology)| terminology);
        let two_letter = Language::from_639_3(terminology).and_then(|language| language.to_639_1());
        Self(two_letter.map_or(lower, str::to_owned))
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Language::from_name_lowercase(&name.trim().to_ascii_lowercase())
            .map(|language| Self::canonical(language.to_639_3()))
    }

    pub fn from_regional_tag(tag: &str) -> Option<Self> {
        let (language, region) = tag.split_once(['-', '_'])?;
        let two_letters =
            |part: &str| part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic());
        (two_letters(language) && two_letters(region)).then(|| {
            Self(format!("{}-{}", language.to_ascii_lowercase(), region.to_ascii_uppercase()))
        })
    }

    pub fn base(&self) -> &str {
        self.0.split_once('-').map_or(self.0.as_str(), |(base, _)| base)
    }

    pub fn known(tag: &str) -> Option<Self> {
        let code = Self::canonical(tag);
        Language::from_639_1(code.base()).is_some().then_some(code)
    }

    pub fn matches(&self, have: &LanguageCode) -> bool {
        let regionless = !self.0.contains('-') || !have.0.contains('-');
        let combined = self.0.contains('+') || have.0.contains('+');
        self.0.eq_ignore_ascii_case(&have.0)
            || (regionless && !combined && self.base().eq_ignore_ascii_case(have.base()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(raw: &str) -> String {
        LanguageCode::canonical(raw).0
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
        assert_eq!(tag("scc"), "sr");
        assert_eq!(tag("SCR"), "hr");
    }

    #[test]
    fn every_bibliographic_tag_has_a_two_letter_form() {
        for (bibliographic, _) in BIBLIOGRAPHIC_TO_TERMINOLOGY {
            assert_eq!(tag(bibliographic).len(), 2, "{bibliographic}");
        }
    }

    #[test]
    fn a_tag_without_a_two_letter_form_is_kept_in_lower_case() {
        assert_eq!(tag("fil"), "fil");
        assert_eq!(tag("und"), "und");
        assert_eq!(tag("xyz"), "xyz");
        assert_eq!(tag("UND"), "und");
        assert_eq!(tag("Fil"), "fil");
    }

    #[test]
    fn a_two_letter_or_regional_tag_is_kept() {
        assert_eq!(tag("en"), "en");
        assert_eq!(tag("EN"), "en");
        assert_eq!(tag("pt-BR"), "pt-BR");
    }

    #[test]
    fn a_regional_tag_is_written_one_way() {
        assert_eq!(tag("pt_BR"), "pt-BR");
        assert_eq!(tag("pt-br"), "pt-BR");
        assert_eq!(tag("PT_br"), "pt-BR");
        assert_eq!(tag("zh-TW"), "zh-TW");
    }

    #[test]
    fn an_english_language_name_becomes_its_code() {
        let named = |name: &str| LanguageCode::from_name(name).map(|code| code.0);
        assert_eq!(named("English").as_deref(), Some("en"));
        assert_eq!(named(" danish ").as_deref(), Some("da"));
        assert_eq!(named("KOREAN").as_deref(), Some("ko"));
        assert_eq!(named("Filipino").as_deref(), Some("fil"));
        assert_eq!(named("Greek"), None);
        assert_eq!(named("Portuguese Brazilian"), None);
        assert_eq!(named(""), None);
    }

    #[test]
    fn only_two_letters_on_each_side_make_a_regional_tag() {
        assert_eq!(LanguageCode::from_regional_tag("pt"), None);
        assert_eq!(LanguageCode::from_regional_tag("por-BR"), None);
        assert_eq!(LanguageCode::from_regional_tag("es-419"), None);
        assert_eq!(LanguageCode::from_regional_tag("x2-BR"), None);
        assert_eq!(LanguageCode::from_regional_tag("pt-B"), None);
    }

    fn code(raw: &str) -> LanguageCode {
        LanguageCode(raw.to_owned())
    }

    #[test]
    fn the_base_drops_the_region() {
        assert_eq!(code("pt-BR").base(), "pt");
        assert_eq!(code("pt").base(), "pt");
        assert_eq!(code("fil").base(), "fil");
    }

    #[test]
    fn a_bare_code_accepts_every_region_of_its_language() {
        assert!(code("pt").matches(&code("pt")));
        assert!(code("pt").matches(&code("pt-BR")));
        assert!(code("pt").matches(&code("pt-PT")));
        assert!(!code("pt").matches(&code("es")));
        assert!(!code("pt").matches(&code("es-PT")));
    }

    #[test]
    fn a_regional_code_accepts_itself_and_its_bare_language_only() {
        assert!(code("pt-BR").matches(&code("pt-BR")));
        assert!(code("pt-BR").matches(&code("pt")));
        assert!(!code("pt-BR").matches(&code("pt-PT")));
        assert!(!code("zh-TW").matches(&code("zh-CN")));
        assert!(!code("pt-BR").matches(&code("es")));
    }

    #[test]
    fn matching_ignores_case() {
        assert!(code("EN").matches(&code("en")));
        assert!(code("pt-br").matches(&code("pt-BR")));
        assert!(code("PT").matches(&code("pt-BR")));
    }

    #[test]
    fn a_longer_code_is_not_a_region_of_a_shorter_one() {
        assert!(!code("fi").matches(&code("fil")));
        assert!(!code("en").matches(&code("en+fr")));
    }

    #[test]
    fn a_combined_code_matches_only_itself() {
        assert!(!code("pt").matches(&code("pt-BR+en")));
        assert!(!code("zh").matches(&code("zh-TW+en")));
        assert!(!code("pt-BR+en").matches(&code("pt")));
        assert!(code("pt-BR+en").matches(&code("pt-br+EN")));
    }

    #[test]
    fn a_known_code_is_one_iso_has_in_two_letters() {
        let known = |raw: &str| LanguageCode::known(raw).map(|code| code.0);
        assert_eq!(known("en").as_deref(), Some("en"));
        assert_eq!(known("ENG").as_deref(), Some("en"));
        assert_eq!(known("ger").as_deref(), Some("de"));
        assert_eq!(known("pt_br").as_deref(), Some("pt-BR"));
        assert_eq!(known("hi").as_deref(), Some("hi"));
        for word in
            ["the", "web", "dts", "up", "tv", "hd-TV", "re-cut", "fil", "und", "English", ""]
        {
            assert_eq!(known(word), None, "{word}");
        }
    }
}
