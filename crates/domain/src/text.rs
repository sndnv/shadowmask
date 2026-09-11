use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn normalize_title(title: &str) -> String {
    let mapped: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn fold_diacritics(title: &str) -> String {
    title.nfd().filter(|c| !is_combining_mark(*c)).collect()
}

pub fn sort_title(title: &str, articles: &[String]) -> String {
    let normalized = normalize_title(&fold_diacritics(title));
    let Some((first, rest)) = normalized.split_once(' ') else {
        return normalized;
    };
    if articles.iter().any(|article| normalize_title(&fold_diacritics(article)) == first) {
        format!("{rest}, {first}")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn normalize_collapses_case_and_punctuation() {
        assert_eq!(normalize_title("The Matrix"), "the matrix");
        assert_eq!(normalize_title("The.Matrix!!"), "the matrix");
        assert_eq!(normalize_title("  Spider-Man  "), "spider man");
    }

    #[test]
    fn fold_diacritics_strips_combining_marks() {
        assert_eq!(fold_diacritics("Élite"), "Elite");
        assert_eq!(fold_diacritics("Amélie"), "Amelie");
        assert_eq!(fold_diacritics("Matrix"), "Matrix");
    }

    fn english() -> Vec<String> {
        vec!["the".to_owned(), "a".to_owned(), "an".to_owned()]
    }

    #[test]
    fn sort_title_folds_case_and_accents_without_any_articles() {
        assert_eq!(sort_title("The Expanse", &[]), "the expanse");
        assert_eq!(sort_title("Élite", &[]), "elite");
        assert_eq!(sort_title("Spider-Man", &[]), "spider man");
        assert_eq!(sort_title("2001: A Space Odyssey", &[]), "2001 a space odyssey");
    }

    #[test]
    fn sort_title_moves_a_configured_leading_article_to_the_end() {
        assert_eq!(sort_title("The Expanse", &english()), "expanse, the");
        assert_eq!(sort_title("the expanse", &english()), "expanse, the");
        assert_eq!(sort_title("A Quiet Place", &english()), "quiet place, a");
        assert_eq!(sort_title("An Education", &english()), "education, an");
    }

    #[test]
    fn sort_title_matches_articles_case_and_accent_insensitively() {
        let german = vec!["Der".to_owned(), "Die".to_owned(), "Das".to_owned()];
        assert_eq!(sort_title("Das Boot", &german), "boot, das");
        let spanish = vec!["Él".to_owned()];
        assert_eq!(sort_title("el laberinto", &spanish), "laberinto, el");
    }

    #[test]
    fn sort_title_leaves_a_bare_article_a_blank_and_a_prefix_alone() {
        assert_eq!(sort_title("The", &english()), "the");
        assert_eq!(sort_title("   ", &english()), "");
        assert_eq!(sort_title("Theater", &english()), "theater");
        assert_eq!(sort_title("Das Boot", &english()), "das boot");
    }

    proptest! {
        #[test]
        fn normalize_title_is_idempotent(input in "\\PC*") {
            let once = normalize_title(&input);
            prop_assert_eq!(normalize_title(&once), once);
        }
    }
}
