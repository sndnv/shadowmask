use serde::Deserialize;

use domain::library::FetchInput;

use crate::dto::library::LibraryKindDto;

#[derive(Debug, Deserialize)]
pub struct FetchRequest {
    pub source_url: String,
    pub kind: LibraryKindDto,
    pub library_id: String,
    pub title: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub season: Option<u16>,
    #[serde(default)]
    pub episode: Option<u16>,
}

impl FetchRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.source_url.trim().is_empty() {
            return Err("source_url must not be empty");
        }
        if self.library_id.trim().is_empty() {
            return Err("library_id must not be empty");
        }
        if self.title.trim().is_empty() {
            return Err("title must not be empty");
        }
        if matches!(self.kind, LibraryKindDto::Tv)
            && (self.season.is_none() || self.episode.is_none())
        {
            return Err("season and episode are required for tv");
        }
        Ok(())
    }

    pub fn into_input(self) -> FetchInput {
        FetchInput {
            source_url: self.source_url,
            kind: self.kind.into(),
            title: self.title,
            year: self.year,
            external_id: self.external_id,
            season: self.season,
            episode: self.episode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> FetchRequest {
        FetchRequest {
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKindDto::Movie,
            library_id: "ext".into(),
            title: "The Matrix".into(),
            year: None,
            external_id: None,
            season: None,
            episode: None,
        }
    }

    #[test]
    fn movie_validates_and_maps() {
        let req = base();
        assert!(req.validate().is_ok());
        let input = req.into_input();
        assert_eq!(input.title, "The Matrix");
        assert!(matches!(input.kind, domain::library::LibraryKind::Movie));
    }

    #[test]
    fn year_passes_through() {
        let mut req = base();
        req.year = Some(1999);
        assert_eq!(req.into_input().year, Some(1999));
    }

    #[test]
    fn external_id_passes_through() {
        let mut req = base();
        req.external_id = Some("tt0133093".into());
        let input = req.into_input();
        assert_eq!(input.external_id.as_deref(), Some("tt0133093"));
    }

    #[test]
    fn tv_requires_season_and_episode() {
        let mut req = base();
        req.kind = LibraryKindDto::Tv;
        assert!(req.validate().is_err());
        req.season = Some(1);
        req.episode = Some(2);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn empty_fields_are_rejected() {
        let mut url = base();
        url.source_url = "  ".into();
        assert!(url.validate().is_err());
        let mut title = base();
        title.title = String::new();
        assert!(title.validate().is_err());
        let mut library = base();
        library.library_id = String::new();
        assert!(library.validate().is_err());
    }
}
