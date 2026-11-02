#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRating {
    pub system: String,
    pub code: String,
}

static AGE_TABLE: &[(&str, &str, u8)] = &[
    ("mpaa", "g", 0),
    ("mpaa", "pg", 8),
    ("mpaa", "pg-13", 13),
    ("mpaa", "r", 17),
    ("mpaa", "nc-17", 18),
    ("us-tv", "tv-y", 0),
    ("us-tv", "tv-y7", 7),
    ("us-tv", "tv-y7-fv", 7),
    ("us-tv", "tv-g", 0),
    ("us-tv", "tv-pg", 8),
    ("us-tv", "tv-14", 14),
    ("us-tv", "tv-ma", 17),
    ("bbfc", "u", 0),
    ("bbfc", "pg", 8),
    ("bbfc", "12", 12),
    ("bbfc", "12a", 12),
    ("bbfc", "15", 15),
    ("bbfc", "18", 18),
    ("de-fsk", "0", 0),
    ("de-fsk", "6", 6),
    ("de-fsk", "12", 12),
    ("de-fsk", "16", 16),
    ("de-fsk", "18", 18),
];

impl ContentRating {
    pub const OLDEST_AGE: u8 = 18;

    pub fn age_floor(&self) -> Option<u8> {
        let system = self.system.to_ascii_lowercase();
        let code = self.code.to_ascii_lowercase();
        AGE_TABLE
            .iter()
            .find(|entry| entry.0 == system.as_str() && entry.1 == code.as_str())
            .map(|entry| entry.2)
    }

    pub fn known_systems() -> Vec<(&'static str, Vec<&'static str>)> {
        let mut systems: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
        for (system, code, _) in AGE_TABLE {
            match systems.iter_mut().find(|(name, _)| name == system) {
                Some((_, codes)) => codes.push(code),
                None => systems.push((system, vec![code])),
            }
        }
        systems
    }

    pub fn allowed_up_to(age: u8) -> Vec<ContentRating> {
        AGE_TABLE
            .iter()
            .filter(|entry| entry.2 <= age)
            .map(|entry| ContentRating { system: entry.0.to_owned(), code: entry.1.to_owned() })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rating(system: &str, code: &str) -> ContentRating {
        ContentRating { system: system.to_owned(), code: code.to_owned() }
    }

    #[test]
    fn known_ratings_map_to_an_age_floor() {
        assert_eq!(rating("MPAA", "PG-13").age_floor(), Some(13));
        assert_eq!(rating("US-TV", "TV-MA").age_floor(), Some(17));
        assert_eq!(rating("US-TV", "TV-Y7-FV").age_floor(), Some(7));
        assert_eq!(rating("BBFC", "18").age_floor(), Some(18));
        assert_eq!(rating("de-fsk", "6").age_floor(), Some(6));
    }

    #[test]
    fn known_systems_group_their_codes_in_table_order() {
        let systems = ContentRating::known_systems();

        let names: Vec<&str> = systems.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec!["mpaa", "us-tv", "bbfc", "de-fsk"]);
        let mpaa = &systems.first().unwrap().1;
        assert_eq!(mpaa, &vec!["g", "pg", "pg-13", "r", "nc-17"]);
        assert!(systems.iter().all(|(_, codes)| !codes.is_empty()));
    }

    #[test]
    fn unknown_rating_has_no_age_floor() {
        assert_eq!(rating("MPAA", "Not Rated").age_floor(), None);
        assert_eq!(rating("XYZ", "42").age_floor(), None);
    }

    #[test]
    fn allowed_up_to_lists_every_rating_at_or_below_the_age() {
        let allowed = ContentRating::allowed_up_to(12);
        assert!(allowed.iter().all(|r| r.age_floor().unwrap() <= 12));
        assert!(allowed.contains(&rating("mpaa", "pg")));
        assert!(allowed.contains(&rating("bbfc", "12a")));
        assert!(allowed.contains(&rating("us-tv", "tv-y7")));
        assert!(!allowed.contains(&rating("mpaa", "pg-13")));
        assert!(!allowed.contains(&rating("de-fsk", "16")));
    }

    #[test]
    fn allowed_up_to_the_youngest_age_keeps_the_all_ages_ratings() {
        assert_eq!(
            ContentRating::allowed_up_to(0),
            vec![
                rating("mpaa", "g"),
                rating("us-tv", "tv-y"),
                rating("us-tv", "tv-g"),
                rating("bbfc", "u"),
                rating("de-fsk", "0"),
            ]
        );
    }

    #[test]
    fn allowed_up_to_the_oldest_age_lists_the_whole_table() {
        assert_eq!(
            ContentRating::allowed_up_to(ContentRating::OLDEST_AGE).len(),
            ContentRating::known_systems().iter().map(|(_, codes)| codes.len()).sum::<usize>()
        );
    }
}
