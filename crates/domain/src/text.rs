use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn normalize_title(title: &str) -> String {
    let lowered = title.to_lowercase().replace('ς', "σ");
    let mut on_letter = false;
    let mapped: String = fold_latin_accents(&lowered)
        .map(|c| {
            let kept = if is_combining_mark(c) {
                on_letter
            } else {
                on_letter = c.is_alphabetic();
                c.is_alphanumeric()
            };
            if kept { c } else { ' ' }
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fold_latin_accents(text: &str) -> impl Iterator<Item = char> + '_ {
    let mut on_latin = false;
    text.nfd()
        .filter(move |&c| {
            if is_combining_mark(c) {
                return !on_latin;
            }
            on_latin = c.is_ascii_alphabetic();
            true
        })
        .nfc()
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
    let normalized = normalize_title(title);
    let Some((first, rest)) = normalized.split_once(' ') else {
        return normalized;
    };
    if articles.iter().any(|article| normalize_title(article) == first) {
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
    fn normalize_keeps_letters_of_every_script() {
        assert_eq!(normalize_title("霓虹港"), "霓虹港");
        assert_ne!(normalize_title("霓虹港"), normalize_title("纸天空"));
        assert_ne!(normalize_title("霓虹港 2"), normalize_title("纸天空 2"));
        assert_eq!(normalize_title("Дрейф"), "дрейф");
        assert_eq!(normalize_title("Deep.Field.2½"), "deep field 2½");
    }

    #[test]
    fn normalize_keeps_the_marks_other_scripts_spell_with() {
        assert_eq!(normalize_title("ガンダム"), "ガンダム");
        assert_ne!(normalize_title("ガ"), normalize_title("カ"));
        assert_ne!(normalize_title("कल"), normalize_title("काल"));
        assert_eq!(normalize_title("Дре\u{438}\u{306}ф"), "дрейф");
    }

    #[test]
    fn normalize_reads_a_word_final_sigma_whatever_follows_it() {
        assert_eq!(normalize_title("ΟΔΟΣ:ΧΑ"), normalize_title("ΟΔΟΣ ΧΑ"));
        assert_eq!(normalize_title("ΟΔΟΣ.ΧΑ"), "οδοσ χα");
    }

    #[test]
    fn normalize_drops_a_mark_left_without_its_letter() {
        assert_eq!(normalize_title("Lantern \u{2764}\u{fe0f}"), "lantern");
        assert_eq!(normalize_title("Lantern \u{2764}"), "lantern");
        assert_eq!(normalize_title("Lantern 1\u{fe0f}\u{20e3}"), "lantern 1");
        assert_eq!(normalize_title("Lantern . \u{301}"), "lantern");
    }

    #[test]
    fn sort_title_keeps_the_marks_other_scripts_spell_with() {
        assert_eq!(sort_title("ガンダム", &[]), "ガンダム");
        assert_eq!(sort_title("Дрейф", &[]), "дрейф");
    }

    #[test]
    fn normalize_folds_accents_whichever_way_they_are_written() {
        assert_eq!(normalize_title("Lant\u{e9}rn"), "lantern");
        assert_eq!(normalize_title("Lante\u{301}rn"), "lantern");
        assert_eq!(normalize_title("Lantern"), "lantern");
        assert_eq!(normalize_title("\u{130}zmir"), "izmir");
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
        assert_eq!(sort_title("霓虹港", &[]), "霓虹港");
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
        fn an_ascii_title_keeps_the_key_it_had_before_other_scripts(input in "[ -~]*") {
            let before: String = input
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
                .collect();
            let before = before.split_whitespace().collect::<Vec<_>>().join(" ");
            prop_assert_eq!(normalize_title(&input), before);
        }

        #[test]
        fn both_unicode_forms_of_a_title_give_one_key(input in "\\PC*") {
            let composed: String = input.nfc().collect();
            let decomposed: String = input.nfd().collect();
            prop_assert_eq!(normalize_title(&composed), normalize_title(&decomposed));
        }

        #[test]
        fn search_text_is_idempotent(input in "\\PC*") {
            let once = search_text(&input);
            prop_assert_eq!(search_text(&once), once);
        }
    }
}
