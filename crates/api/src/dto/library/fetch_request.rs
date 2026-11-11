use serde::Deserialize;

use domain::library::FetchInput;
use domain::media::FetchOptions;

#[derive(Debug, Deserialize)]
pub struct FetchRequest {
    pub source_url: String,
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
    #[serde(default)]
    pub episode_title: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub skip_note: Option<String>,
    #[serde(default)]
    pub prefer_h264: bool,
    #[serde(default)]
    pub site_subtitles: bool,
    #[serde(default)]
    pub chapters: bool,
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
        Ok(())
    }

    pub fn into_input(self) -> FetchInput {
        FetchInput {
            source_url: self.source_url,
            title: self.title,
            year: self.year,
            external_id: self.external_id,
            season: self.season,
            episode: self.episode,
            episode_title: self.episode_title,
            options: FetchOptions {
                format: self.format,
                skip_note: self.skip_note,
                prefer_h264: self.prefer_h264,
                site_subtitles: self.site_subtitles,
                chapters: self.chapters,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> FetchRequest {
        FetchRequest {
            source_url: "https://example.com/watch?v=abc".into(),
            library_id: "ext".into(),
            title: "The Matrix".into(),
            year: None,
            external_id: None,
            season: None,
            episode: None,
            episode_title: None,
            format: None,
            skip_note: None,
            prefer_h264: false,
            site_subtitles: false,
            chapters: false,
        }
    }

    #[test]
    fn movie_validates_and_maps() {
        let req = base();
        assert!(req.validate().is_ok());
        let input = req.into_input();
        assert_eq!(input.title, "The Matrix");
        assert_eq!(input.source_url, "https://example.com/watch?v=abc");
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
    fn a_kind_sent_by_an_older_client_is_ignored() {
        let req: FetchRequest = serde_json::from_value(serde_json::json!({
            "source_url": "https://example.com/watch?v=abc",
            "kind": "tv",
            "library_id": "ext",
            "title": "Great Show"
        }))
        .unwrap();
        assert!(req.validate().is_ok());
        let input = req.into_input();
        assert_eq!(input.season, None);
        assert_eq!(input.options, FetchOptions::default());
    }

    #[test]
    fn download_options_pass_through() {
        let req: FetchRequest = serde_json::from_value(serde_json::json!({
            "source_url": "https://example.com/watch?v=abc",
            "library_id": "ext",
            "title": "The Matrix",
            "format": "bestvideo+bestaudio",
            "skip_note": "Premium",
            "prefer_h264": true,
            "site_subtitles": true,
            "chapters": true
        }))
        .unwrap();
        assert_eq!(
            req.into_input().options,
            FetchOptions {
                format: Some("bestvideo+bestaudio".into()),
                skip_note: Some("Premium".into()),
                prefer_h264: true,
                site_subtitles: true,
                chapters: true,
            }
        );
    }

    #[test]
    fn season_episode_and_title_pass_through() {
        let mut req = base();
        req.season = Some(1);
        req.episode = Some(2);
        req.episode_title = Some("Pilot".into());
        let input = req.into_input();
        assert_eq!((input.season, input.episode), (Some(1), Some(2)));
        assert_eq!(input.episode_title.as_deref(), Some("Pilot"));
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
