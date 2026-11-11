use domain::library::{LibraryId, LibraryKind, ParsedMedia};
use domain::media::FetchOptions;
use domain::metadata::ExternalId;
use serde::{Deserialize, Serialize};

use super::parse::{PROVIDER_TAG, RESOLUTION, YEAR_BARE, YEAR_PAREN};
use crate::job::encode_payload;
use crate::library::parse_filename;

const MAX_STEM_BYTES: usize = 200;
const MAX_FORMAT_CHARS: usize = 200;
const MAX_SKIP_NOTE_CHARS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchJobPayload {
    pub library: LibraryId,
    pub source_url: String,
    pub title: String,
    pub year: Option<u16>,
    pub external_id: Option<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
    pub episode_title: Option<String>,
    pub options: FetchOptions,
}

impl FetchJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }

    pub fn resolved_external_id(&self, kind: LibraryKind) -> Option<ExternalId> {
        let value = self.external_id.as_deref()?.trim();
        if value.is_empty() {
            return None;
        }
        if value.starts_with("tt") {
            return Some(ExternalId { source: "imdb".to_owned(), value: value.to_owned() });
        }
        let endpoint = match kind {
            LibraryKind::Tv => "tv",
            LibraryKind::Movie => "movie",
        };
        let value =
            if value.contains('/') { value.to_owned() } else { format!("{endpoint}/{value}") };
        Some(ExternalId { source: "tmdb".to_owned(), value })
    }

    pub fn filename_stem(&self, kind: LibraryKind) -> String {
        let title = safe_segment(&self.title);
        let tag = self
            .resolved_external_id(kind)
            .as_ref()
            .and_then(filename_tag)
            .map(|tag| format!(" [{tag}]"))
            .unwrap_or_default();
        match kind {
            LibraryKind::Tv => {
                let s = self.season.unwrap_or(1);
                let e = self.episode.unwrap_or(1);
                let numbered = format!("{title} - S{s:02}E{e:02}");
                let room = MAX_STEM_BYTES.saturating_sub(numbered.len() + tag.len() + 3);
                match self.episode_title.as_deref().and_then(|name| episode_segment(name, room)) {
                    Some(name) => format!("{numbered} - {name}{tag}"),
                    None => format!("{numbered}{tag}"),
                }
            }
            LibraryKind::Movie => match self.year {
                Some(year) => format!("{title} ({year}){tag}"),
                None => format!("{title}{tag}"),
            },
        }
    }

    pub fn parsed(&self, kind: LibraryKind, path: &str) -> ParsedMedia {
        let episodic = matches!(kind, LibraryKind::Tv);
        ParsedMedia {
            title: self.title.trim().to_owned(),
            year: self.year,
            season: episodic.then(|| self.season.unwrap_or(1)),
            episode: episodic.then(|| self.episode.unwrap_or(1)),
            quality: parse_filename(path).quality,
            external_id: self.resolved_external_id(kind),
            episode_title: self
                .episode_title
                .as_deref()
                .map(str::trim)
                .filter(|name| episodic && !name.is_empty())
                .map(str::to_owned),
        }
    }

    pub fn destination_dir(&self, kind: LibraryKind, root: &str, fallback_name: &str) -> String {
        let title = match safe_segment(&self.title) {
            name if name.is_empty() => safe_segment(fallback_name),
            name => name,
        };
        match kind {
            LibraryKind::Tv => {
                let s = self.season.unwrap_or(1);
                format!("{root}/{title}/Season {s:02}")
            }
            LibraryKind::Movie => match self.year {
                Some(year) => format!("{root}/{title} ({year})"),
                None => format!("{root}/{title}"),
            },
        }
    }
}

pub fn filename_tag(id: &ExternalId) -> Option<String> {
    let value = id.value.rsplit('/').next().unwrap_or(&id.value);
    if value.is_empty() || value.len() > 24 || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    match id.source.as_str() {
        "tmdb" => Some(format!("tmdbid-{value}")),
        "imdb" => Some(format!("imdbid-{value}")),
        _ => None,
    }
}

pub(super) fn names_a_file(title: &str) -> bool {
    !safe_segment(title).is_empty()
}

fn safe_segment(name: &str) -> String {
    let mut current = sanitized(&name.replace('$', " "));
    loop {
        let next = sanitized(&current);
        if next == current {
            return current;
        }
        current = next;
    }
}

fn sanitized(name: &str) -> String {
    sanitize_filename::sanitize_with_options(
        name,
        sanitize_filename::Options { windows: true, truncate: true, replacement: " " },
    )
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

fn episode_segment(name: &str, room: usize) -> Option<String> {
    let words = without_release_words(&safe_segment(name));
    let cut = without_release_words(&words[..words.floor_char_boundary(room)]);
    let cut = cut.trim_end_matches(['.', ' ']);
    (!cut.is_empty()).then(|| cut.to_owned())
}

fn without_release_words(text: &str) -> String {
    [&*PROVIDER_TAG, &*YEAR_PAREN, &*YEAR_BARE, &*RESOLUTION]
        .into_iter()
        .fold(text.to_owned(), |text, pattern| pattern.replace_all(&text, " ").into_owned())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn checked_options(options: FetchOptions) -> Result<FetchOptions, String> {
    let format = filled(options.format);
    let skip_note = filled(options.skip_note);
    if let Some(format) = &format {
        if format.chars().count() > MAX_FORMAT_CHARS {
            return Err(format!("the format is longer than {MAX_FORMAT_CHARS} characters"));
        }
        if format.chars().any(char::is_control) {
            return Err("the format holds a control character".to_owned());
        }
        if format.starts_with('-') {
            return Err("the format cannot start with a -".to_owned());
        }
    }
    if let Some(note) = &skip_note {
        if note.chars().count() > MAX_SKIP_NOTE_CHARS {
            return Err(format!(
                "the note to skip is longer than {MAX_SKIP_NOTE_CHARS} characters"
            ));
        }
        if !note.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')) {
            return Err(
                "the note to skip may hold only letters, digits, spaces, - and _".to_owned()
            );
        }
    }
    if format.is_some() && skip_note.is_some() {
        return Err(
            "a format replaces the default choice, so it cannot also skip noted streams".to_owned()
        );
    }
    Ok(FetchOptions { format, skip_note, ..options })
}

fn filled(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned()).filter(|text| !text.is_empty())
}

#[derive(Serialize, Deserialize)]
struct Wire {
    library: String,
    source_url: String,
    title: String,
    #[serde(default)]
    year: Option<u16>,
    external_id: Option<String>,
    season: Option<u16>,
    episode: Option<u16>,
    #[serde(default)]
    episode_title: Option<String>,
    #[serde(default)]
    options: WireOptions,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct WireOptions {
    format: Option<String>,
    skip_note: Option<String>,
    prefer_h264: bool,
    site_subtitles: bool,
    chapters: bool,
}

impl From<&FetchJobPayload> for Wire {
    fn from(payload: &FetchJobPayload) -> Self {
        let options = &payload.options;
        Wire {
            library: payload.library.0.clone(),
            source_url: payload.source_url.clone(),
            title: payload.title.clone(),
            year: payload.year,
            external_id: payload.external_id.clone(),
            season: payload.season,
            episode: payload.episode,
            episode_title: payload.episode_title.clone(),
            options: WireOptions {
                format: options.format.clone(),
                skip_note: options.skip_note.clone(),
                prefer_h264: options.prefer_h264,
                site_subtitles: options.site_subtitles,
                chapters: options.chapters,
            },
        }
    }
}

impl From<Wire> for FetchJobPayload {
    fn from(wire: Wire) -> Self {
        FetchJobPayload {
            library: LibraryId(wire.library),
            source_url: wire.source_url,
            title: wire.title,
            year: wire.year,
            external_id: wire.external_id,
            season: wire.season,
            episode: wire.episode,
            episode_title: wire.episode_title,
            options: FetchOptions {
                format: wire.options.format,
                skip_note: wire.options.skip_note,
                prefer_h264: wire.options.prefer_h264,
                site_subtitles: wire.options.site_subtitles,
                chapters: wire.options.chapters,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::confidence;
    use domain::text::normalize_title;

    fn round_trip(payload: FetchJobPayload) {
        let encoded = payload.encode();
        assert_eq!(FetchJobPayload::decode(&encoded).unwrap(), payload);
    }

    fn movie(title: &str, year: Option<u16>, external_id: Option<&str>) -> FetchJobPayload {
        FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            title: title.into(),
            year,
            external_id: external_id.map(str::to_owned),
            season: None,
            episode: None,
            episode_title: None,
            options: FetchOptions::default(),
        }
    }

    fn named(payload: FetchJobPayload, episode_title: &str) -> FetchJobPayload {
        FetchJobPayload { episode_title: Some(episode_title.into()), ..payload }
    }

    fn series(
        title: &str,
        season: Option<u16>,
        episode: Option<u16>,
        external_id: Option<&str>,
    ) -> FetchJobPayload {
        FetchJobPayload {
            title: title.into(),
            season,
            episode,
            external_id: external_id.map(str::to_owned),
            ..movie(title, None, None)
        }
    }

    fn every_option() -> FetchOptions {
        FetchOptions {
            format: Some("bestvideo+bestaudio".into()),
            skip_note: Some("Premium".into()),
            prefer_h264: true,
            site_subtitles: true,
            chapters: true,
        }
    }

    #[test]
    fn round_trips_movie_and_tv() {
        round_trip(movie("The Matrix", Some(1999), Some("tt0133093")));
        round_trip(series("Great Show", Some(1), Some(2), None));
        round_trip(named(series("Great Show", Some(1), Some(2), None), "Pilot"));
        round_trip(FetchJobPayload { options: every_option(), ..movie("The Matrix", None, None) });
    }

    #[test]
    fn a_payload_written_before_the_year_existed_still_decodes() {
        let legacy = r#"{"library":"lib1","source_url":"https://example.com/a","kind":"movie","title":"The Matrix","external_id":null,"season":null,"episode":null}"#;
        assert_eq!(FetchJobPayload::decode(legacy).unwrap().year, None);
    }

    #[test]
    fn a_payload_queued_before_the_options_existed_decodes_with_every_option_off() {
        let queued = r#"{"library":"lib1","source_url":"https://example.com/a","title":"The Matrix","year":null,"external_id":null,"season":null,"episode":null,"episode_title":null}"#;
        assert_eq!(FetchJobPayload::decode(queued).unwrap().options, FetchOptions::default());
        let partial = r#"{"library":"lib1","source_url":"https://example.com/a","title":"The Matrix","external_id":null,"season":null,"episode":null,"options":{"chapters":true}}"#;
        assert_eq!(
            FetchJobPayload::decode(partial).unwrap().options,
            FetchOptions { chapters: true, ..FetchOptions::default() }
        );
    }

    fn format(text: &str) -> FetchOptions {
        FetchOptions { format: Some(text.into()), ..FetchOptions::default() }
    }

    fn skip_note(text: &str) -> FetchOptions {
        FetchOptions { skip_note: Some(text.into()), ..FetchOptions::default() }
    }

    #[test]
    fn options_with_nothing_typed_pass_unchanged_and_blank_text_is_unset() {
        let ticked = FetchOptions { format: Some("  ".into()), ..every_option() };
        assert_eq!(
            checked_options(FetchOptions { skip_note: Some(" \t ".into()), ..ticked }),
            Ok(FetchOptions { format: None, skip_note: None, ..every_option() })
        );
        assert_eq!(checked_options(FetchOptions::default()), Ok(FetchOptions::default()));
    }

    #[test]
    fn a_format_is_trimmed_and_kept_within_its_limits() {
        assert_eq!(checked_options(format("  bv*+ba/b ")), Ok(format("bv*+ba/b")));
        assert_eq!(checked_options(format(&"b".repeat(200))), Ok(format(&"b".repeat(200))));
        for (refused, reason) in [
            ("b".repeat(201), "longer than 200 characters"),
            ("bv\n+ba".to_owned(), "control character"),
            ("bv\u{7f}".to_owned(), "control character"),
            ("-o /etc/x".to_owned(), "cannot start with a -"),
            (" --exec rm".to_owned(), "cannot start with a -"),
        ] {
            let error = checked_options(format(&refused)).unwrap_err();
            assert!(error.contains(reason), "[{refused}] {error}");
        }
    }

    #[test]
    fn a_note_to_skip_takes_letters_of_any_script_digits_spaces_dashes_and_underscores() {
        for allowed in ["Premium", "enhanced bitrate", "1080p_HDR-10", "プレミアム", "Премиум"]
        {
            assert_eq!(checked_options(skip_note(allowed)), Ok(skip_note(allowed)), "[{allowed}]");
        }
        assert_eq!(checked_options(skip_note(&"é".repeat(40))), Ok(skip_note(&"é".repeat(40))));
        for refused in
            ["pre\"mium", "pre\\mium", "pre.mium", "(?i)x", "a|b", "x]", "a*", "a\u{a0}b"]
        {
            let error = checked_options(skip_note(refused)).unwrap_err();
            assert!(error.contains("may hold only"), "[{refused}] {error}");
        }
        let error = checked_options(skip_note(&"é".repeat(41))).unwrap_err();
        assert!(error.contains("longer than 40 characters"), "{error}");
    }

    #[test]
    fn a_format_and_a_note_to_skip_together_are_refused() {
        let both = FetchOptions { skip_note: Some("Premium".into()), ..format("bv+ba") };
        assert_eq!(
            checked_options(both),
            Err("a format replaces the default choice, so it cannot also skip noted streams".into())
        );
    }

    #[test]
    fn a_payload_queued_with_a_kind_still_decodes() {
        let queued = r#"{"library":"lib1","source_url":"https://example.com/a","kind":"tv","title":"Great Show","year":null,"external_id":null,"season":1,"episode":2}"#;
        let payload = FetchJobPayload::decode(queued).unwrap();
        assert_eq!(payload.title, "Great Show");
        assert_eq!((payload.season, payload.episode), (Some(1), Some(2)));
        assert_eq!(payload.episode_title, None);
    }

    #[test]
    fn an_episode_carries_its_typed_title_and_a_movie_none() {
        let episode = named(series("Great Show", Some(1), Some(2), None), "  Pilot ");
        assert_eq!(
            episode.parsed(LibraryKind::Tv, "/ext/x.mkv").episode_title.as_deref(),
            Some("Pilot")
        );
        assert_eq!(episode.parsed(LibraryKind::Movie, "/ext/x.mkv").episode_title, None);
    }

    fn reads_back(stem: &str) -> ParsedMedia {
        parse_filename(&format!("/ext/Great Show/Season 01/{stem}.mkv"))
    }

    #[test]
    fn an_episode_title_is_named_and_read_back() {
        let stem = named(series("Great Show", Some(1), Some(2), None), "Pilot")
            .filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02 - Pilot");
        let parsed = reads_back(&stem);
        assert_eq!(parsed.title, "Great Show");
        assert_eq!((parsed.season, parsed.episode), (Some(1), Some(2)));
    }

    #[test]
    fn a_tagged_episode_title_is_named_and_read_back() {
        let stem = named(series("Great Show", Some(1), Some(2), Some("1399")), "Pilot")
            .filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02 - Pilot [tmdbid-1399]");
        let parsed = reads_back(&stem);
        assert_eq!(parsed.title, "Great Show");
        assert_eq!((parsed.season, parsed.episode), (Some(1), Some(2)));
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
    }

    #[test]
    fn an_episode_title_that_reads_like_a_release_is_still_read_back() {
        for typed in ["Recap of S02E05", "Best of 2019", "Recap (2019)", "A Night in Rome"] {
            let stem = named(series("Great Show", Some(1), Some(2), None), typed)
                .filename_stem(LibraryKind::Tv);
            let parsed = reads_back(&stem);
            assert_eq!(parsed.title, "Great Show", "{stem}");
            assert_eq!((parsed.season, parsed.episode), (Some(1), Some(2)), "{stem}");
            assert_eq!(parsed.year, None, "{stem}");
        }
    }

    #[test]
    fn release_words_leave_the_episode_title_in_the_file_name_only() {
        let payload = named(series("Great Show", Some(1), Some(2), None), "Japan in 4K 1080p Tour");
        let stem = payload.filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02 - Japan in Tour");
        assert_eq!(reads_back(&stem).quality, None);
        assert_eq!(
            payload.parsed(LibraryKind::Tv, "/ext/x.mkv").episode_title.as_deref(),
            Some("Japan in 4K 1080p Tour")
        );
        let stem_of = |typed: &str| {
            named(series("Great Show", Some(1), Some(2), None), typed)
                .filename_stem(LibraryKind::Tv)
        };
        assert_eq!(stem_of("4K"), "Great Show - S01E02");
        assert_eq!(stem_of("Best of 2019"), "Great Show - S01E02 - Best of");
        assert_eq!(stem_of("Recap (2019)"), "Great Show - S01E02 - Recap");
        assert_eq!(stem_of("Tour in 4:K"), "Great Show - S01E02 - Tour in 4 K");
        assert_eq!(reads_back(&stem_of("Tour in 4:K")).quality, None);
        assert_eq!(stem_of("Tour in 4K:"), "Great Show - S01E02 - Tour in");
    }

    #[test]
    fn a_tag_typed_into_the_episode_title_cannot_beat_the_real_one() {
        let stem =
            named(series("Great Show", Some(1), Some(2), Some("1399")), "Fake [tmdbid-603] Cut")
                .filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02 - Fake Cut [tmdbid-1399]");
        assert_eq!(
            reads_back(&stem).external_id,
            Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
    }

    #[test]
    fn a_long_episode_title_is_cut_on_a_character_boundary() {
        let stem = named(series("Great Show", Some(1), Some(2), Some("1399")), &"日".repeat(300))
            .filename_stem(LibraryKind::Tv);
        assert!(stem.len() <= 200, "{}", stem.len());
        assert!(stem.starts_with("Great Show - S01E02 - 日日"));
        assert!(stem.ends_with("日 [tmdbid-1399]"));
    }

    #[test]
    fn a_cut_that_ends_on_a_dot_or_a_spaced_dot_leaves_neither() {
        for (typed, kept) in
            [(format!("{}.bbb", "a".repeat(177)), 177), (format!("{} .b", "a".repeat(176)), 176)]
        {
            let stem = named(series("Great Show", Some(1), Some(2), None), &typed)
                .filename_stem(LibraryKind::Tv);
            assert_eq!(stem, format!("Great Show - S01E02 - {}", "a".repeat(kept)));
        }
    }

    #[test]
    fn a_cut_cannot_rebuild_a_year_or_a_quality_word() {
        for typed in [format!("{} 20195", "a".repeat(173)), format!("{} 4KHDR", "a".repeat(175))] {
            let stem = named(series("Great Show", Some(1), Some(2), None), &typed)
                .filename_stem(LibraryKind::Tv);
            let parsed = reads_back(&stem);
            assert_eq!((parsed.year, parsed.quality), (None, None), "{stem}");
        }
    }

    #[test]
    fn a_series_title_that_fills_the_name_leaves_the_episode_title_out() {
        let long = "a".repeat(230);
        let stem =
            named(series(&long, Some(1), Some(2), None), "Pilot").filename_stem(LibraryKind::Tv);
        assert_eq!(stem, format!("{long} - S01E02"));
    }

    #[test]
    fn a_dollar_never_reaches_a_fetched_name() {
        let episode = named(series("Pay $HOME Back", Some(1), Some(2), None), "${USER} Deals");
        assert_eq!(episode.filename_stem(LibraryKind::Tv), "Pay HOME Back - S01E02 - {USER} Deals");
        assert_eq!(
            episode.destination_dir(LibraryKind::Tv, "/ext", "job-1"),
            "/ext/Pay HOME Back/Season 01"
        );
        assert_eq!(
            movie("$HOME Movie", None, None).filename_stem(LibraryKind::Movie),
            "HOME Movie"
        );
        assert_eq!(
            episode.parsed(LibraryKind::Tv, "/ext/x.mkv").title,
            "Pay $HOME Back",
            "the catalog keeps what was typed"
        );
        let id_with_a_dollar =
            named(series("Great Show", Some(1), Some(2), Some("1399$HOME")), "Pilot");
        assert_eq!(id_with_a_dollar.filename_stem(LibraryKind::Tv), "Great Show - S01E02 - Pilot");
    }

    #[test]
    fn a_dollar_in_a_series_title_keeps_its_series_key() {
        let payload = named(series("Ca$h Cab", Some(1), Some(2), None), "Pilot");
        let stem = payload.filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Ca h Cab - S01E02 - Pilot");
        let dir = payload.destination_dir(LibraryKind::Tv, "/ext", "job-1");
        let parsed = parse_filename(&format!("{dir}/{stem}.mkv"));
        assert_eq!(normalize_title(&parsed.title), normalize_title("Ca$h Cab"));
        assert_eq!(
            named(series("Cash $ Money", Some(1), Some(2), None), "Pilot")
                .filename_stem(LibraryKind::Tv),
            "Cash Money - S01E02 - Pilot"
        );
    }

    #[test]
    fn a_character_a_name_cannot_hold_keeps_its_series_key() {
        for typed in ["AC/DC Live", "Who?What"] {
            let payload = named(series(typed, Some(1), Some(2), None), "Pilot");
            let dir = payload.destination_dir(LibraryKind::Tv, "/ext", "job-1");
            let stem = payload.filename_stem(LibraryKind::Tv);
            let parsed = parse_filename(&format!("{dir}/{stem}.mkv"));
            assert_eq!(normalize_title(&parsed.title), normalize_title(typed), "{dir}/{stem}");
        }
        assert_eq!(
            named(series("AC/DC Live", Some(1), Some(2), None), "Pilot")
                .filename_stem(LibraryKind::Tv),
            "AC DC Live - S01E02 - Pilot"
        );
    }

    #[test]
    fn a_tag_value_the_parser_could_not_read_back_is_left_out() {
        let tag =
            |value: &str| filename_tag(&ExternalId { source: "tmdb".into(), value: value.into() });
        assert_eq!(tag("tv/1399").as_deref(), Some("tmdbid-1399"));
        assert_eq!(tag("1399$HOME"), None);
        assert_eq!(tag("tv/"), None);
        assert_eq!(tag(&"9".repeat(24)), Some(format!("tmdbid-{}", "9".repeat(24))));
        assert_eq!(tag(&"9".repeat(25)), None);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(FetchJobPayload::decode("not json").is_err());
    }

    #[test]
    fn a_typed_title_is_kept_whole_however_it_reads_as_a_release_name() {
        let payload = movie("OPEN STUDIO 4K DEMO REEL 2160P 60FPS", None, None);
        let parsed = payload.parsed(LibraryKind::Movie, "/ext/whatever.mkv");
        assert_eq!(parsed.title, "OPEN STUDIO 4K DEMO REEL 2160P 60FPS");
        assert_eq!(parse_filename("/ext/OPEN STUDIO 4K DEMO.mkv").title, "OPEN STUDIO");
    }

    #[test]
    fn a_title_that_ends_in_a_bare_year_keeps_it() {
        let parsed =
            movie("Blade Runner 2049", None, None).parsed(LibraryKind::Movie, "/ext/x.mkv");
        assert_eq!(parsed.title, "Blade Runner 2049");
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn a_movie_carries_its_year_and_no_episode_numbering() {
        let parsed = movie("The Matrix", Some(1999), Some("tt0133093"))
            .parsed(LibraryKind::Movie, "/ext/x.mkv");
        assert_eq!(parsed.year, Some(1999));
        assert_eq!(parsed.season, None);
        assert_eq!(parsed.episode, None);
        assert_eq!(parsed.external_id.unwrap().source, "imdb");
    }

    #[test]
    fn an_episode_defaults_to_the_same_numbering_the_filename_uses() {
        // filename_stem falls back to S01E01, so parsed has to agree or the
        // catalog entry names a different episode than the file on disk.
        let payload = series("Great Show", None, None, None);
        let parsed = payload.parsed(LibraryKind::Tv, "/ext/x.mkv");
        assert_eq!((parsed.season, parsed.episode), (Some(1), Some(1)));
        assert!(payload.filename_stem(LibraryKind::Tv).contains("S01E01"));

        let numbered =
            series("Great Show", Some(2), Some(5), None).parsed(LibraryKind::Tv, "/ext/x.mkv");
        assert_eq!((numbered.season, numbered.episode), (Some(2), Some(5)));
    }

    #[test]
    fn quality_still_comes_from_the_file_since_the_payload_cannot_know_it() {
        let payload = movie("Anything", None, None);
        assert_eq!(payload.parsed(LibraryKind::Movie, "/ext/x.mkv").quality, None);
        assert_eq!(
            payload.parsed(LibraryKind::Movie, "/ext/x 2160p.mkv").quality,
            parse_filename("/ext/x 2160p.mkv").quality
        );
    }

    #[test]
    fn resolved_external_id_detects_imdb_tmdb_and_blank() {
        let mut payload = movie("The Matrix", None, Some("tt0133093"));
        assert_eq!(
            payload.resolved_external_id(LibraryKind::Movie),
            Some(domain::metadata::ExternalId { source: "imdb".into(), value: "tt0133093".into() })
        );
        payload.external_id = Some("603".into());
        assert_eq!(
            payload.resolved_external_id(LibraryKind::Movie),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "movie/603".into() })
        );
        payload.external_id = Some("1399".into());
        assert_eq!(
            payload.resolved_external_id(LibraryKind::Tv),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
        payload.external_id = Some("tv/1399".into());
        assert_eq!(
            payload.resolved_external_id(LibraryKind::Tv),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
        payload.external_id = Some("  ".into());
        assert_eq!(payload.resolved_external_id(LibraryKind::Tv), None);
        payload.external_id = None;
        assert_eq!(payload.resolved_external_id(LibraryKind::Tv), None);
    }

    #[test]
    fn movie_stem_round_trips_through_parser() {
        let stem = movie("The Matrix", Some(1999), None).filename_stem(LibraryKind::Movie);
        assert_eq!(stem, "The Matrix (1999)");
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, Some(1999));
        assert!(!parsed.is_episodic());
    }

    #[test]
    fn movie_without_year_stem_round_trips_through_parser() {
        let stem = movie("Some Movie", None, None).filename_stem(LibraryKind::Movie);
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "Some Movie");
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn tv_stem_round_trips_through_parser() {
        let stem = series("Great Show", Some(1), Some(2), None).filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02");
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "Great Show");
        assert_eq!(parsed.season, Some(1));
        assert_eq!(parsed.episode, Some(2));
    }

    #[test]
    fn the_same_payload_is_named_and_placed_by_the_kind_it_is_given() {
        let payload = series("Great Show", Some(1), Some(2), None);
        assert_eq!(payload.filename_stem(LibraryKind::Movie), "Great Show");
        assert_eq!(payload.destination_dir(LibraryKind::Movie, "/ext", "job-1"), "/ext/Great Show");
        let parsed = payload.parsed(LibraryKind::Movie, "/ext/x.mkv");
        assert_eq!((parsed.season, parsed.episode), (None, None));
    }

    #[test]
    fn a_tagged_movie_survives_a_wipe_without_its_year() {
        let stem = movie("The Matrix", None, Some("603")).filename_stem(LibraryKind::Movie);
        assert_eq!(stem, "The Matrix [tmdbid-603]");
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, None);
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "movie/603".into() })
        );
        assert_eq!(confidence(&parsed), 0.9);
    }

    #[test]
    fn an_imdb_id_is_tagged_and_read_back_as_imdb() {
        let stem =
            movie("The Matrix", Some(1999), Some("tt0133093")).filename_stem(LibraryKind::Movie);
        assert_eq!(stem, "The Matrix (1999) [imdbid-tt0133093]");
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, Some(1999));
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "imdb".into(), value: "tt0133093".into() })
        );
    }

    #[test]
    fn a_tagged_episode_reads_back_as_a_tv_id() {
        let stem =
            series("Great Show", Some(1), Some(2), Some("1399")).filename_stem(LibraryKind::Tv);
        assert_eq!(stem, "Great Show - S01E02 [tmdbid-1399]");
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "Great Show");
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
    }

    #[test]
    fn a_provider_we_do_not_tag_for_leaves_the_name_alone() {
        assert_eq!(filename_tag(&ExternalId { source: "tvdb".into(), value: "603".into() }), None);
    }

    #[test]
    fn a_movie_lands_in_a_title_directory() {
        let dir =
            |payload: FetchJobPayload| payload.destination_dir(LibraryKind::Movie, "/ext", "job-1");
        assert_eq!(dir(movie("The Matrix", Some(1999), None)), "/ext/The Matrix (1999)");
        assert_eq!(dir(movie("Some Movie", None, None)), "/ext/Some Movie");
    }

    #[test]
    fn an_episode_lands_under_a_padded_season_directory() {
        let dir =
            |payload: FetchJobPayload| payload.destination_dir(LibraryKind::Tv, "/ext", "job-1");
        assert_eq!(dir(series("Great Show", Some(1), Some(2), None)), "/ext/Great Show/Season 01");
        assert_eq!(dir(series("Great Show", None, None, None)), "/ext/Great Show/Season 01");
        assert_eq!(dir(series("Great Show", Some(12), Some(3), None)), "/ext/Great Show/Season 12");
    }

    #[test]
    fn a_title_cannot_escape_the_library_root() {
        let dir =
            |payload: FetchJobPayload| payload.destination_dir(LibraryKind::Movie, "/ext", "job-1");
        assert_eq!(dir(movie("../../etc", None, None)), "/ext/.. .. etc");
        assert_eq!(dir(movie("Face/Off", Some(1997), None)), "/ext/Face Off (1997)");
        assert_eq!(
            movie("Alien: Resurrection", None, None).filename_stem(LibraryKind::Movie),
            "Alien Resurrection"
        );
    }

    #[test]
    fn a_title_that_sanitises_to_nothing_falls_back_to_the_job_id() {
        assert_eq!(
            movie("///", None, None).destination_dir(LibraryKind::Movie, "/ext", "job-1"),
            "/ext/job-1"
        );
        assert_eq!(
            series("///", Some(2), Some(1), None).destination_dir(LibraryKind::Tv, "/ext", "job-1"),
            "/ext/job-1/Season 02"
        );
    }

    #[test]
    fn a_title_that_collapses_to_dots_or_a_reserved_name_falls_back_to_the_job_id() {
        let titles = [
            "..\u{a0}",
            ".\u{a0}.",
            "..\u{a0}..",
            "\u{a0}..\u{3000}",
            "CON:",
            "nul\u{a0}",
            "CON .\u{a0}",
            "lpt9 .\u{3000}",
        ];
        for typed in titles {
            let movie_dir =
                movie(typed, None, None).destination_dir(LibraryKind::Movie, "/ext", "job-1");
            let tv_dir = series(typed, Some(1), Some(2), None).destination_dir(
                LibraryKind::Tv,
                "/ext",
                "job-1",
            );
            assert_eq!(movie_dir, "/ext/job-1", "{typed:?}");
            assert_eq!(tv_dir, "/ext/job-1/Season 01", "{typed:?}");
            assert!(!names_a_file(typed), "{typed:?}");
        }
        assert!(!names_a_file("///"));
        assert!(names_a_file("Great Show"));
        assert!(names_a_file("AC/DC"));
    }
}
