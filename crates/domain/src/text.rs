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

pub fn search_text(text: &str) -> String {
    search_chunks(text).concat().join(" ")
}

pub fn joined_terms(text: &str) -> Vec<String> {
    let mut joined: Vec<String> = search_chunks(text)
        .into_iter()
        .filter(|parts| parts.len() > 1)
        .map(|parts| parts.concat())
        .collect();
    joined.sort();
    joined.dedup();
    joined
}

pub fn typo_allowance(word: &str) -> usize {
    if is_number(word) {
        return 0;
    }
    match word.chars().count() {
        0..=4 => 0,
        5..=8 => 1,
        _ => 2,
    }
}

pub fn is_typo_of(word: &str, term: &str) -> bool {
    let allowed = typo_allowance(word);
    allowed > 0
        && !is_number(term)
        && word.chars().count().abs_diff(term.chars().count()) <= allowed
        && strsim::osa_distance(word, term) <= allowed
}

fn is_number(word: &str) -> bool {
    word.chars().all(|c| c.is_ascii_digit())
}

fn search_chunks(text: &str) -> Vec<Vec<String>> {
    fold_diacritics(&text.replace('&', " and "))
        .to_lowercase()
        .split_whitespace()
        .map(|chunk| {
            chunk
                .split(|c: char| !c.is_alphanumeric())
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
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

    #[test]
    fn search_text_folds_accents_and_keeps_letters_of_any_script() {
        assert_eq!(search_text("Amélie"), "amelie");
        assert_eq!(search_text("  Spider-Man: Far From Home "), "spider man far from home");
        assert_eq!(search_text("Ocean's Eleven"), "ocean s eleven");
        assert_eq!(search_text("Брат 2"), "брат 2");
        assert_eq!(search_text("大话西游"), "大话西游");
        assert_eq!(search_text("a%e_b"), "a e b");
        assert_eq!(search_text(" .:- "), "");
    }

    #[test]
    fn search_text_reads_an_ampersand_as_and() {
        assert_eq!(search_text("Fast & Furious"), "fast and furious");
        assert_eq!(search_text("Tom&Jerry"), "tom and jerry");
    }

    #[test]
    fn a_word_split_by_punctuation_is_also_a_joined_term() {
        assert_eq!(joined_terms("Spider-Man: Far From Home"), ["spiderman"]);
        assert_eq!(joined_terms("Marvel's Agents of S.H.I.E.L.D."), ["marvels", "shield"]);
        assert_eq!(joined_terms("M*A*S*H"), ["mash"]);
        assert_eq!(joined_terms("WALL·E and WALL-E"), ["walle"]);
        assert!(joined_terms("The Matrix").is_empty());
        assert!(joined_terms("Mr. & Mrs. Smith").is_empty());
    }

    #[test]
    fn a_word_of_five_letters_may_be_one_edit_off_and_of_nine_two() {
        assert!(is_typo_of("matirx", "matrix"), "swapped letters are one edit");
        assert!(is_typo_of("matrx", "matrix"));
        assert!(is_typo_of("interstelar", "interstellar"));
        assert!(is_typo_of("intrestelar", "interstellar"));
        assert!(!is_typo_of("intresteler", "interstellar"));
        assert!(!is_typo_of("jaws", "jams"), "a word of four letters has to be typed right");
        assert!(!is_typo_of("matrix", "mattress"));
        assert!(!is_typo_of("house", "houseboat"));
        assert_eq!(typo_allowance("амели"), 1, "letters are counted, not bytes");
    }

    #[test]
    fn a_number_is_never_a_typo_of_another() {
        assert!(!is_typo_of("12345", "12346"), "a number one digit off is another episode");
        assert!(!is_typo_of("a1234", "1234"), "nor is a number offered as a correction");
        assert!(is_typo_of("2fast2furios", "2fast2furious"));
    }

    proptest! {
        #[test]
        fn normalize_title_is_idempotent(input in "\\PC*") {
            let once = normalize_title(&input);
            prop_assert_eq!(normalize_title(&once), once);
        }

        #[test]
        fn search_text_is_idempotent(input in "\\PC*") {
            let once = search_text(&input);
            prop_assert_eq!(search_text(&once), once);
        }
    }
}
