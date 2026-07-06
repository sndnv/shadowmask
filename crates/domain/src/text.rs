pub fn normalize_title(title: &str) -> String {
    let mapped: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
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

    proptest! {
        #[test]
        fn normalize_title_is_idempotent(input in "\\PC*") {
            let once = normalize_title(&input);
            prop_assert_eq!(normalize_title(&once), once);
        }
    }
}
