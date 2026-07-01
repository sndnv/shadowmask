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
    pub fn age_floor(&self) -> Option<u8> {
        let system = self.system.to_ascii_lowercase();
        let code = self.code.to_ascii_lowercase();
        AGE_TABLE
            .iter()
            .find(|entry| entry.0 == system.as_str() && entry.1 == code.as_str())
            .map(|entry| entry.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rating(system: &str, code: &str) -> ContentRating {
        ContentRating {
            system: system.to_owned(),
            code: code.to_owned(),
        }
    }

    #[test]
    fn known_ratings_map_to_an_age_floor() {
        assert_eq!(rating("MPAA", "PG-13").age_floor(), Some(13));
        assert_eq!(rating("US-TV", "TV-MA").age_floor(), Some(17));
        assert_eq!(rating("BBFC", "18").age_floor(), Some(18));
        assert_eq!(rating("de-fsk", "6").age_floor(), Some(6));
    }

    #[test]
    fn unknown_rating_has_no_age_floor() {
        assert_eq!(rating("MPAA", "Not Rated").age_floor(), None);
        assert_eq!(rating("XYZ", "42").age_floor(), None);
    }
}
