use std::path::Path;
use std::sync::LazyLock;

use domain::common::Quality;
use domain::library::ParsedMedia;
use domain::media::FORMAT_TOKENS;
use domain::metadata::ExternalId;
use jiff::{Timestamp, tz::TimeZone};
use regex::Regex;

const CONF_STRONG: f32 = 0.9;
const CONF_WEAK: f32 = 0.4;
const CONF_NONE: f32 = 0.0;

static SEASON_EPISODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bs([0-9]{1,2})[ ._-]{0,2}ep?[ ._-]{0,2}([0-9]{1,3})").unwrap()
});
static EPISODE_OF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([0-9]{1,3})of[0-9]{1,3}\b").unwrap());
static DIGIT_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]+").unwrap());
static CODEC_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[hx][ ._]?26[45]").unwrap());
static FRAME_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[0-9]{3,4}[x×][0-9]{3,4}").unwrap());
static CHECKSUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[0-9A-Fa-f]{8}\]").unwrap());
static SEASON_EPISODE_X: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([0-9]{1,2})x([0-9]{1,3})\b").unwrap());
static YEAR_PAREN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[(\[]((?:19|20)[0-9]{2})[)\]]").unwrap());
static YEAR_BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:19|20)[0-9]{2}\b").unwrap());
static JUNK: LazyLock<Regex> = LazyLock::new(|| {
    let mut tokens: Vec<&str> = FORMAT_TOKENS
        .iter()
        .filter(|token| token.truncates_a_title)
        .map(|token| token.text)
        .collect();
    tokens.sort_unstable_by_key(|token| std::cmp::Reverse(token.len()));
    let escaped: Vec<String> = tokens.into_iter().map(regex::escape).collect();
    Regex::new(&format!(r"(?i)\b({})\b", escaped.join("|"))).unwrap()
});
static RESOLUTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(480p|576p|720p|1080p|2160p|4k)\b").unwrap());
static PROVIDER_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[(tmdbid|imdbid)-([a-z0-9]{1,24})\]").unwrap());
static SEASON_DIR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bseasons?[ ._-]*([0-9]{1,3})\b").unwrap());
static ORDINAL_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:0[0-9][ ._-]|[1-9][0-9]\.)").unwrap());
static SEASON_ORDINAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{2}[ ._-]").unwrap());

const MAX_ANCESTORS: usize = 3;

pub fn parse_filename(path: &str) -> ParsedMedia {
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = basename.rsplit_once('.').map_or(basename, |(s, _)| s);
    let mut parsed = parse_stem(stem);
    let folders = ancestors(path);

    let season_dir = folders.iter().enumerate().find_map(|(index, dir)| {
        let captures = SEASON_DIR.captures(dir)?;
        let season = captures.get(1).unwrap().as_str().parse().ok()?;
        Some((index, captures.get(0).unwrap().start(), season))
    });

    if let Some((index, cut, folder_season)) = season_dir {
        if parsed.episode.is_none() {
            parsed.episode = folders[..index]
                .iter()
                .find_map(|dir| find_season_episode(dir))
                .map(|(season, episode, _)| {
                    parsed.season = Some(season);
                    episode
                })
                .or_else(|| find_episode_of(stem))
                .or_else(|| find_compact_episode(stem, folder_season));
        }
        if parsed.episode.is_some() {
            parsed.season = parsed.season.or(Some(folder_season));
            let series = series_title(&folders, index, cut);
            if !series.is_empty() {
                parsed.title = series;
            }
        }
    }
    if !parsed.is_episodic()
        && !under_a_series(&folders, season_dir)
        && let Some(folder) = folders.first().map(|dir| folder_media(dir))
        && folder.year.is_some()
        && !folder.title.is_empty()
    {
        parsed.title = folder.title;
        parsed.year = folder.year;
    }

    parsed
}

fn season_names(folders: &[&str], index: usize, cut: usize) -> (String, String) {
    let named = &folders[index][..cut];
    let above = folders.get(index + 1).map(|dir| parse_stem(dir).title).unwrap_or_default();
    let stripped = parse_stem(&SEASON_ORDINAL.replace(named, "")).title;
    let own = if names_a_season_of_the_folder_above(&stripped, &above) {
        stripped
    } else {
        parse_stem(named).title
    };
    (own, above)
}

fn names_a_season_of_the_folder_above(own: &str, above: &str) -> bool {
    own.is_empty() || (!above.is_empty() && squashed(above).ends_with(&squashed(own)))
}

fn under_a_series(folders: &[&str], season_dir: Option<(usize, usize, u16)>) -> bool {
    let Some((index, cut, _)) = season_dir else {
        return false;
    };
    let (own, above) = season_names(folders, index, cut);
    names_a_season_of_the_folder_above(&own, &above)
}

fn series_title(folders: &[&str], index: usize, cut: usize) -> String {
    let (own, above) = season_names(folders, index, cut);
    if names_a_season_of_the_folder_above(&own, &above) {
        return if above.is_empty() { own } else { above };
    }
    own
}

fn squashed(title: &str) -> String {
    title.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn find_episode_of(stem: &str) -> Option<u16> {
    EPISODE_OF.captures(stem)?.get(1)?.as_str().parse().ok()
}

fn find_compact_episode(stem: &str, season: u16) -> Option<u16> {
    let cleaned = [&*JUNK, &*CODEC_NUMBER, &*FRAME_SIZE, &*CHECKSUM]
        .iter()
        .fold(stem.to_owned(), |text, noise| noise.replace_all(&text, " ").into_owned());
    DIGIT_RUN.find_iter(&cleaned).find_map(|run| {
        let digits = run.as_str();
        let before_a_letter = cleaned[run.end()..].starts_with(|c: char| c.is_ascii_alphabetic());
        if !(3..=4).contains(&digits.len()) || before_a_letter || YEAR_BARE.is_match(digits) {
            return None;
        }
        let split = digits.len() - 2;
        let episode: u16 = digits[split..].parse().ok()?;
        (digits[..split].parse::<u16>().ok()? == season && episode > 0).then_some(episode)
    })
}

fn ancestors(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split(['/', '\\']).filter(|part| !part.is_empty()).collect();
    parts.pop();
    parts.reverse();
    parts.truncate(MAX_ANCESTORS);
    parts
}

fn folder_media(name: &str) -> ParsedMedia {
    parse_stem(ORDINAL_PREFIX.replace(name, "").as_ref())
}

fn parse_stem(stem: &str) -> ParsedMedia {
    let season_episode = find_season_episode(stem);
    let year = find_year(stem);
    let junk = JUNK.find(stem).map(|m| m.start());
    let external_id = find_provider_tag(stem, season_episode.is_some());

    let mut cut = stem.len();
    if let Some((_, _, start)) = season_episode {
        cut = cut.min(start);
    }
    if let Some((_, start)) = year {
        cut = cut.min(start);
    }
    if let Some(start) = junk {
        cut = cut.min(start);
    }
    if let Some((_, start)) = &external_id {
        cut = cut.min(*start);
    }

    ParsedMedia {
        title: clean_title(&stem[..cut]),
        year: year.map(|(y, _)| y),
        season: season_episode.map(|(s, _, _)| s),
        episode: season_episode.map(|(_, e, _)| e),
        quality: find_quality(stem),
        external_id: external_id.map(|(id, _)| id),
    }
}

pub fn confidence(parsed: &ParsedMedia) -> f32 {
    if parsed.title.is_empty() {
        CONF_NONE
    } else if parsed.external_id.is_some() || parsed.is_episodic() || parsed.year.is_some() {
        CONF_STRONG
    } else {
        CONF_WEAK
    }
}

fn find_provider_tag(stem: &str, episodic: bool) -> Option<(ExternalId, usize)> {
    let captures = PROVIDER_TAG.captures(stem)?;
    let value = captures.get(2).unwrap().as_str();
    let id = match captures.get(1).unwrap().as_str().to_ascii_lowercase().as_str() {
        "imdbid" => ExternalId { source: "imdb".to_owned(), value: value.to_owned() },
        _ => ExternalId {
            source: "tmdb".to_owned(),
            value: format!("{}/{value}", if episodic { "tv" } else { "movie" }),
        },
    };
    Some((id, captures.get(0).unwrap().start()))
}

fn find_season_episode(stem: &str) -> Option<(u16, u16, usize)> {
    let captures = SEASON_EPISODE.captures(stem).or_else(|| SEASON_EPISODE_X.captures(stem))?;
    let whole = captures.get(0).unwrap();
    let season = captures.get(1).unwrap().as_str().parse().unwrap();
    let episode = captures.get(2).unwrap().as_str().parse().unwrap();
    Some((season, episode, whole.start()))
}

static LATEST_RELEASE_YEAR: LazyLock<u16> =
    LazyLock::new(|| Timestamp::now().to_zoned(TimeZone::UTC).year() as u16 + 1);

fn releasable(year: u16) -> bool {
    year <= *LATEST_RELEASE_YEAR
}

fn find_year(stem: &str) -> Option<(u16, usize)> {
    if let Some(captures) = YEAR_PAREN.captures(stem) {
        let year: u16 = captures.get(1).unwrap().as_str().parse().unwrap();
        if releasable(year) {
            return Some((year, captures.get(0).unwrap().start()));
        }
    }
    YEAR_BARE
        .find_iter(stem)
        .filter_map(|matched| {
            let year: u16 = matched.as_str().parse().ok()?;
            releasable(year).then_some((year, matched.start()))
        })
        .last()
}

fn find_quality(stem: &str) -> Option<Quality> {
    let token = RESOLUTION.find(stem)?.as_str().to_ascii_lowercase();
    Some(if token == "2160p" || token == "4k" {
        Quality::Uhd
    } else if token == "1080p" {
        Quality::Fhd
    } else if token == "720p" {
        Quality::Hd
    } else {
        Quality::Sd
    })
}

pub fn quality_from_height(height: u32) -> Quality {
    if height >= 2000 {
        Quality::Uhd
    } else if height >= 1080 {
        Quality::Fhd
    } else if height >= 720 {
        Quality::Hd
    } else {
        Quality::Sd
    }
}

pub fn quality_token(quality: Quality) -> &'static str {
    match quality {
        Quality::Uhd => "2160p",
        Quality::Fhd => "1080p",
        Quality::Hd => "720p",
        Quality::Sd => "480p",
    }
}

fn strip_quality_token(stem: &str) -> String {
    RESOLUTION.replace_all(stem, "").split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn upscaled_output_path(source_path: &str, target: Quality, uuid: &str) -> String {
    let src = Path::new(source_path);
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let base = strip_quality_token(stem);
    let name = format!("{base} {} [Upscaled {uuid}].mp4", quality_token(target));
    src.parent().unwrap_or(Path::new("")).join(name).to_string_lossy().into_owned()
}

fn clean_title(raw: &str) -> String {
    let spaced: String = raw.chars().map(|c| if c == '.' || c == '_' { ' ' } else { c }).collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| matches!(c, '-' | ' ' | '(' | '['))
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str) -> ParsedMedia {
        parse_filename(name)
    }

    const PLACEMENTS: &[(&str, &str)] = &[
        ("/tv/Harbor Lights/Season 02/Harbor.Lights.S02E03.720p-TAG.mkv", "Harbor Lights"),
        (
            "/tv/Tallowbrook/Tallowbrook.Season.01/1x01 - The One Where The Lamp Goes Out.mkv",
            "Tallowbrook",
        ),
        ("/tv/Orbit.XR-7/XR-7.Season.04/XR-7.S04E01.mkv", "Orbit XR-7"),
        ("/Series/Tallowbrook.Season.01/1x01 - The One With The Fog.mkv", "Tallowbrook"),
        ("/tv/Ember Vale/Season 02/1x01.mkv", "Ember Vale"),
        (
            "/tv/Drift/Drift.Season.06/Drift.S06E17.HDTV.XviD-TAG/drift.0617.hdtv.xvid-tag.cd1.avi",
            "Drift",
        ),
        ("/tv/Mile Marker/Mile Marker Season 20 (2013)/The Quiet Detour.mkv", "The Quiet Detour"),
        (
            "/tv/Mile Marker/Mile Marker - Season 20/Mile Marker - The Quiet Detour (2013).mkv",
            "Mile Marker - The Quiet Detour",
        ),
        ("/tv/Silent Signal/Season 20 (2013)/Silent Signal 2013.mkv", "Silent Signal"),
        ("/tv/Silent Signal/Season 20/Silent.Signal.2013.HDTV.XviD-TAG/cd1.avi", "cd1"),
        ("/tv/Show/Show.Season.01/a stray recording.mkv", "a stray recording"),
        ("/m/Static Season 2 (2008)/tag-ss2-1080p.mkv", "Static Season 2"),
        ("/m/Sunrise/01.Sunrise.(1979)/rip-TAG.mkv", "Sunrise"),
        (
            "/m/Midnight.Archive/07.Midnight.Archive.And.The.Silent.Signal.Part.1.(2010)/tag-ma7part1-1080p.mkv",
            "Midnight Archive And The Silent Signal Part 1",
        ),
        ("/m/Paper.Skies.1999.1080p.mkv", "Paper Skies"),
        ("/m/14 Paper Skies (1957)/14 Paper Skies (1957).mkv", "14 Paper Skies"),
        ("/m/27 Neon Harbors (2002)/rip-TAG.mkv", "27 Neon Harbors"),
        ("/m/27_Neon_Harbors_(2002)/rip.mkv", "27 Neon Harbors"),
        ("/m/Neon Harbor/01.Neon.Harbor.(1982)/01.Neon.Harbor.(1982).mkv", "Neon Harbor"),
        ("/m/Neon Harbor/01 - Neon Harbor (1982)/rip.avi", "Neon Harbor"),
        ("/m/Neon Harbor/12.Neon.Harbor.Part.12.(2005)/rip.avi", "Neon Harbor Part 12"),
        ("/tv/30 Lanterns/Season 1/30.Lanterns.S01E01.mkv", "30 Lanterns"),
        ("/tv/30 Lanterns/Season 1/S01E01.mkv", "30 Lanterns"),
        ("/Series/30 Lanterns Season 1/S01E01.mkv", "30 Lanterns"),
        ("/tv/10.20.30/Season 1/10.20.30.S01E01.mkv", "10 20 30"),
        ("/tv/10.20.30/Season 1/S01E01.mkv", "10 20 30"),
        ("/Series/10.20.30.Season.1/S01E01.mkv", "10 20 30"),
        ("/tv/Neon Harbor/01 - Season 1/S01E01.mkv", "Neon Harbor"),
        ("/tv/Neon Harbor/10 - Season 10/S10E01.mkv", "Neon Harbor"),
        ("/tv/Neon Harbor/11 Season 11/Neon.Harbor.S11E01.mkv", "Neon Harbor"),
        ("/tv/Neon Harbor/02.Neon.Harbor.Season.2/S02E01.mkv", "Neon Harbor"),
    ];

    #[test]
    fn every_shape_the_folder_rules_place() {
        for (path, title) in PLACEMENTS {
            assert_eq!(parse(path).title, *title, "[{path}] is the shape that disagrees");
        }
    }

    #[test]
    fn a_dotted_two_digit_prefix_on_a_movie_folder_is_read_as_an_ordinal() {
        assert_eq!(parse("/m/30.Lanterns.(2002)/rip.avi").title, "Lanterns");
        assert_eq!(parse("/m/30 Lanterns (2002)/rip.avi").title, "30 Lanterns");
    }

    #[test]
    fn quality_from_height_maps_thresholds() {
        assert_eq!(quality_from_height(2160), Quality::Uhd);
        assert_eq!(quality_from_height(1080), Quality::Fhd);
        assert_eq!(quality_from_height(720), Quality::Hd);
        assert_eq!(quality_from_height(480), Quality::Sd);
    }

    #[test]
    fn quality_token_round_trips_through_the_parser() {
        for q in [Quality::Uhd, Quality::Fhd, Quality::Hd, Quality::Sd] {
            assert_eq!(find_quality(quality_token(q)), Some(q));
        }
    }

    #[test]
    fn upscaled_output_path_swaps_quality_and_marks_upscaled() {
        let out =
            upscaled_output_path("/m/Movie (2011)/Movie (2011) 480p.mkv", Quality::Fhd, "abcd1234");
        assert!(out.starts_with("/m/Movie (2011)/"));
        assert!(out.ends_with("Movie (2011) 1080p [Upscaled abcd1234].mp4"));
        let p = parse(&out);
        assert_eq!(p.title, "Movie");
        assert_eq!(p.year, Some(2011));
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn upscaled_output_path_is_unique_per_uuid() {
        let a = upscaled_output_path("/m/A/A 720p.mkv", Quality::Uhd, "uuid-a");
        let b = upscaled_output_path("/m/A/A 720p.mkv", Quality::Uhd, "uuid-b");
        assert_ne!(a, b);
    }

    #[test]
    fn upscaled_output_path_preserves_episode_identity() {
        let out =
            upscaled_output_path("/tv/Show/Season 01/Show S01E02 720p.mkv", Quality::Fhd, "zz");
        let p = parse(&out);
        assert_eq!(p.title, "Show");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(2));
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn parses_dotted_movie_with_year_and_resolution() {
        let p = parse("/m/Paper.Skies.1999.1080p.BluRay.x264-TAG.mkv");
        assert_eq!(p.title, "Paper Skies");
        assert_eq!(p.year, Some(1999));
        assert_eq!(p.season, None);
        assert_eq!(p.episode, None);
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn parses_parenthesized_year() {
        let p = parse("The Matrix (1999).mkv");
        assert_eq!(p.title, "The Matrix");
        assert_eq!(p.year, Some(1999));
        assert_eq!(p.quality, None);
    }

    #[test]
    fn parenthesized_release_year_wins_over_year_in_title() {
        let p = parse("Blade Runner 2049 (2017) 2160p.mkv");
        assert_eq!(p.title, "Blade Runner 2049");
        assert_eq!(p.year, Some(2017));
        assert_eq!(p.quality, Some(Quality::Uhd));
    }

    #[test]
    fn keeps_number_in_movie_title() {
        let p = parse("Sinister 2 (2015).mkv");
        assert_eq!(p.title, "Sinister 2");
        assert_eq!(p.year, Some(2015));
    }

    #[test]
    fn a_future_year_in_the_title_is_not_a_release_year() {
        let p = parse("Orbit Station 2049.mkv");
        assert_eq!(p.title, "Orbit Station 2049");
        assert_eq!(p.year, None);
    }

    #[test]
    fn a_future_year_is_refused_in_brackets_as_well_as_bare() {
        assert_eq!(parse("Some Film (2098).mkv").year, None);
        assert_eq!(parse("Some Film 2098.mkv").year, None);
    }

    #[test]
    fn this_year_and_next_are_release_years_in_brackets_and_bare() {
        let current = Timestamp::now().to_zoned(TimeZone::UTC).year() as u16;
        for year in [current, current + 1] {
            let bracketed = parse(&format!("/m/Some.Film.({year})/Some Film ({year}).mkv"));
            assert_eq!((bracketed.title.as_str(), bracketed.year), ("Some Film", Some(year)));
            let bare = parse(&format!("/m/Some.Film.({year})/Some Film {year} 1080p WEB-DL.mkv"));
            assert_eq!((bare.title.as_str(), bare.year), ("Some Film", Some(year)));
        }
        assert_eq!(parse(&format!("Some Film ({}).mkv", current + 2)).year, None);
    }

    #[test]
    fn a_bare_release_year_is_taken_from_the_end_not_the_title() {
        let p = parse("2001 A Lantern Voyage 1968.mkv");
        assert_eq!(p.title, "2001 A Lantern Voyage");
        assert_eq!(p.year, Some(1968));
    }

    #[test]
    fn a_bare_future_year_does_not_truncate_a_longer_title() {
        let p = parse("Orbit Station 2049 2017 1080p.mkv");
        assert_eq!(p.title, "Orbit Station 2049");
        assert_eq!(p.year, Some(2017));
    }

    #[test]
    fn parses_dotted_season_episode() {
        let p = parse("Show.Name.S01E02.720p.mkv");
        assert_eq!(p.title, "Show Name");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(2));
        assert_eq!(p.year, None);
        assert_eq!(p.quality, Some(Quality::Hd));
        assert!(p.is_episodic());
    }

    #[test]
    fn parses_x_style_season_episode() {
        let p = parse("Show - 1x02.mkv");
        assert_eq!(p.title, "Show");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(2));
    }

    #[test]
    fn multi_episode_keeps_first_episode() {
        let p = parse("Show.S01E02E03.mkv");
        assert_eq!(p.title, "Show");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(2));
    }

    #[test]
    fn bare_title_has_no_year_or_episode() {
        let p = parse("recording.mkv");
        assert_eq!(p.title, "recording");
        assert_eq!(p.year, None);
        assert!(!p.is_episodic());
    }

    #[test]
    fn year_only_name_has_empty_title() {
        let p = parse("2019.mkv");
        assert_eq!(p.title, "");
        assert_eq!(p.year, Some(2019));
    }

    #[test]
    fn dotfile_stem_is_empty() {
        let p = parse("/m/.mkv");
        assert_eq!(p.title, "");
    }

    #[test]
    fn resolution_token_maps_to_quality() {
        assert_eq!(parse("A (2001) 4k.mkv").quality, Some(Quality::Uhd));
        assert_eq!(parse("A (2001) 2160p.mkv").quality, Some(Quality::Uhd));
        assert_eq!(parse("A (2001) 1080p.mkv").quality, Some(Quality::Fhd));
        assert_eq!(parse("A (2001) 720p.mkv").quality, Some(Quality::Hd));
        assert_eq!(parse("A (2001) 480p.mkv").quality, Some(Quality::Sd));
        assert_eq!(parse("A (2001) 576p.mkv").quality, Some(Quality::Sd));
    }

    #[test]
    fn a_provider_tag_is_read_without_polluting_the_title() {
        let p = parse("/ext/The Matrix (1999)/The Matrix (1999) [tmdbid-603].mkv");
        assert_eq!(p.title, "The Matrix");
        assert_eq!(p.year, Some(1999));
        assert_eq!(
            p.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "movie/603".into() })
        );
    }

    #[test]
    fn a_tagged_episode_resolves_against_the_tv_endpoint() {
        let p = parse("Great Show - S01E02 [TMDBID-1399].mkv");
        assert_eq!(p.title, "Great Show");
        assert_eq!(
            p.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
    }

    #[test]
    fn an_imdb_tag_keeps_its_value_verbatim() {
        let p = parse("The Matrix [imdbid-tt0133093].mkv");
        assert_eq!(p.title, "The Matrix");
        assert_eq!(
            p.external_id,
            Some(ExternalId { source: "imdb".into(), value: "tt0133093".into() })
        );
    }

    #[test]
    fn an_untagged_name_parses_exactly_as_before() {
        let p = parse("Paper.Skies.1999.1080p.BluRay.x264-TAG.mkv");
        assert_eq!(p.title, "Paper Skies");
        assert_eq!(p.year, Some(1999));
        assert_eq!(p.external_id, None);
    }

    #[test]
    fn an_unknown_tag_is_left_alone() {
        let p = parse("The Matrix (1999) [tvdbid-603].mkv");
        assert_eq!(p.external_id, None);
    }

    #[test]
    fn a_season_folder_names_the_series_when_the_filename_cannot() {
        let p = parse(
            "/tv/Tallowbrook/Tallowbrook.Season.01/1x01 - The One Where The Lamp Goes Out.mkv",
        );
        assert_eq!(p.title, "Tallowbrook");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(1));
    }

    #[test]
    fn an_episode_marker_alone_still_finds_its_series() {
        let p = parse("/tv/Mile.Marker/Mile.Marker.Season.20/S20E06.mkv");
        assert_eq!(p.title, "Mile Marker");
        assert_eq!(p.season, Some(20));
        assert_eq!(p.episode, Some(6));
    }

    #[test]
    fn a_leading_episode_number_does_not_become_part_of_the_series() {
        let a = parse(
            "/tv/Harbor.Lights/Harbor.Lights.Season.01/01 - Harbor Lights S1E00 - The Pilot.mkv",
        );
        let b = parse(
            "/tv/Harbor.Lights/Harbor.Lights.Season.01/02 - Harbor Lights S1E01 - The Harbour.mkv",
        );
        assert_eq!(a.title, "Harbor Lights");
        assert_eq!(a.title, b.title);
        assert_eq!(b.episode, Some(1));
    }

    #[test]
    fn an_absolute_episode_number_does_not_become_part_of_the_series() {
        let a = parse(
            "/tv/Rooftop.Garden.Club/Rooftop.Garden.Club.Season.01/Rooftop.Garden.Club.001.-.1x01.-.Pilot.avi",
        );
        let b = parse(
            "/tv/Rooftop.Garden.Club/Rooftop.Garden.Club.Season.01/Rooftop.Garden.Club.002.-.1x02.-.Seedlings.avi",
        );
        assert_eq!(a.title, "Rooftop Garden Club");
        assert_eq!(a.title, b.title);
        assert_eq!((b.season, b.episode), (Some(1), Some(2)));
    }

    #[test]
    fn a_release_folder_supplies_the_episode_the_filename_lost() {
        let p = parse(
            "/tv/Drift/Drift.Season.06/Drift.S06E17.HDTV.XviD-TAG/drift.0617.hdtv.xvid-tag.cd1.avi",
        );
        assert_eq!(p.title, "Drift");
        assert_eq!((p.season, p.episode), (Some(6), Some(17)));
    }

    #[test]
    fn a_franchise_folder_outranks_the_season_folder_prefix() {
        let p = parse("/tv/Orbit.XR-7/XR-7.Season.04/XR-7.S04E01.mkv");
        assert_eq!(p.title, "Orbit XR-7");
    }

    #[test]
    fn a_library_root_never_becomes_the_series_title() {
        let p = parse("/Series/Tallowbrook.Season.01/1x01 - The One With The Fog.mkv");
        assert_eq!(p.title, "Tallowbrook");
    }

    #[test]
    fn a_bare_season_folder_takes_the_series_from_above_it() {
        let p = parse("/tv/Ember Vale/Season 02/1x01.mkv");
        assert_eq!(p.title, "Ember Vale");
    }

    #[test]
    fn a_spaced_or_ep_episode_marker_is_read() {
        let spaced =
            parse("/tv/Puddlejump/Puddlejump.Season.3/Puddlejump - S03 E02 - Rainy Day.mp4");
        assert_eq!((spaced.season, spaced.episode), (Some(3), Some(2)));
        let ep = parse("/tv/Ember.Vale/Ember.Vale.Season.02/Ember Vale s02ep01 720p.mkv");
        assert_eq!((ep.season, ep.episode), (Some(2), Some(1)));
    }

    #[test]
    fn a_documentary_part_number_is_an_episode() {
        let p = parse("/tv/Starwell/Starwell.Season.01/Starwell.1980.03of13.Distant.Lights.mkv");
        assert_eq!(p.title, "Starwell");
        assert_eq!((p.season, p.episode), (Some(1), Some(3)));
    }

    #[test]
    fn a_compact_number_is_an_episode_when_the_folder_agrees() {
        let three = parse("/tv/Lantern/Lantern.Season.01/tag-lantern103.avi");
        assert_eq!((three.season, three.episode), (Some(1), Some(3)));
        let four = parse("/tv/Pioneer.One/Pioneer.One.Season.02/pioneer.one.0201.dvdrip.avi");
        assert_eq!((four.season, four.episode), (Some(2), Some(1)));
    }

    #[test]
    fn a_codec_a_frame_size_a_checksum_or_episode_zero_is_never_a_compact_episode() {
        for path in [
            "/tv/Neon Harbor/Season 2/Neon.Harbor.2017.1080p.WEB-DL.DD5.1.H.264-TAG.mkv",
            "/tv/Neon Harbor/Season 2/Neon Harbor 2017 WEB DDP5 1 H 264-TAG.mkv",
            "/tv/Neon Harbor/Season 2/neon_harbor_2019_web-dl_x264-tag.mkv",
            "/tv/Neon Harbor/Season 10/Neon.Harbor.1920x1080.mkv",
            "/tv/Neon Harbor/Season 10/Neon_Harbor_1920x1080.mkv",
            "/tv/Neon Harbor/Season 7/Neon Harbor 1280x720.mkv",
            "/tv/Neon Harbor/Season 1/Neon Harbor - 100 Greatest Moments.mkv",
            "/tv/Neon Harbor/Season 1/Neon Harbor - Pilot [ABCD0103].mkv",
        ] {
            let parsed = parse(path);
            assert_eq!((parsed.season, parsed.episode), (None, None), "[{path}]");
        }
    }

    #[test]
    fn the_first_compact_number_that_fits_the_season_is_the_episode() {
        for (path, season, episode) in [
            ("/tv/Neon Harbor 911/Season 1/neon.harbor.911.103.mkv", 1, 3),
            ("/tv/Lantern 100/Season 1/lantern.100.103.mkv", 1, 3),
            ("/tv/Lantern/Season 1/lantern.101.102.mkv", 1, 1),
            ("/tv/Neon Harbor/Season 1/Neon Harbor - 101.mkv", 1, 1),
        ] {
            let parsed = parse(path);
            assert_eq!((parsed.season, parsed.episode), (Some(season), Some(episode)), "[{path}]");
        }
    }

    #[test]
    fn a_resolution_is_never_read_as_a_compact_episode() {
        let p = parse("/tv/Show/Show.Season.10/show 1080p.mkv");
        assert_eq!(p.episode, None);
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn a_compact_number_that_disagrees_with_the_folder_is_not_an_episode() {
        let p = parse("/tv/Show/Show.Season.01/Show.1980.something.mkv");
        assert_eq!(p.episode, None);
    }

    #[test]
    fn a_year_is_never_the_episode_of_the_season_folder_it_sits_in() {
        let paren = parse(
            "/tv/Mile Marker/Mile Marker - Season 20/Mile Marker - The Quiet Detour (2013).mkv",
        );
        assert_eq!(paren.episode, None);
        assert_eq!(paren.year, Some(2013));

        let bare = parse("/tv/Mile Marker/Season 19/Mile Marker 1992 Special.mkv");
        assert_eq!(bare.episode, None);
        assert_eq!(bare.year, Some(1992));
    }

    #[test]
    fn a_season_folder_never_supplies_a_movie_title() {
        let parsed = parse("/tv/Silent Signal/Season 20 (2013)/Silent Signal 2013.mkv");

        assert_eq!(parsed.title, "Silent Signal");
        assert_eq!(parsed.year, Some(2013));
    }

    #[test]
    fn a_year_shaped_run_is_refused_as_the_episode_whether_it_has_passed_or_not() {
        let past = parse("/tv/Silent Signal/Season 20/Silent Signal 2013.mkv");
        assert_eq!(past.episode, None);

        let ahead = parse("/tv/Silent Signal/Season 20/Silent Signal 2128.mkv");
        assert_eq!(ahead.episode, None);
        assert_eq!(ahead.season, None);
    }

    #[test]
    fn a_movie_folder_named_like_a_season_still_supplies_the_title() {
        let p = parse("/m/Static Season 2 (2008)/tag-ss2-1080p.mkv");
        assert_eq!(p.title, "Static Season 2");
        assert_eq!(p.year, Some(2008));
        assert_eq!(p.episode, None);
    }

    #[test]
    fn a_movie_folder_supplies_the_title_the_filename_lost() {
        let p = parse(
            "/m/Midnight.Archive/07.Midnight.Archive.And.The.Silent.Signal.Part.1.(2010)/tag-ma7part1-1080p.mkv",
        );
        assert_eq!(p.title, "Midnight Archive And The Silent Signal Part 1");
        assert_eq!(p.year, Some(2010));
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn two_files_in_one_movie_folder_are_one_title() {
        let uhd = parse(
            "/m/Pioneer.One/04.Pioneer.One.Episode.4.A.New.Dawn.(1977)/Pioneer One Episode IV - A New Dawn (1977) 2160p.mkv",
        );
        let hd = parse(
            "/m/Pioneer.One/04.Pioneer.One.Episode.4.A.New.Dawn.(1977)/Pioneer.One.Episode.4.A.New.Dawn.1977.720p.mkv",
        );
        assert_eq!(uhd.title, hd.title);
        assert_eq!(uhd.year, hd.year);
        assert_ne!(uhd.quality, hd.quality);
    }

    #[test]
    fn an_ordinal_prefix_is_not_part_of_the_movie_title() {
        let p = parse("/m/Neon.Harbor/01.Neon.Harbor.(1982)/nhrbr720p-tag.avi");
        assert_eq!(p.title, "Neon Harbor");
        assert_eq!(p.year, Some(1982));
    }

    #[test]
    fn a_numeric_title_keeps_its_leading_digits() {
        assert_eq!(parse("/m/Misc/101.Lanterns.(1961)/rip.avi").title, "101 Lanterns");
        assert_eq!(parse("/m/Misc/500.Nights.of.Rain.(2009)/rip.avi").title, "500 Nights of Rain");
    }

    #[test]
    fn a_grouping_folder_without_a_year_leaves_the_filename_alone() {
        let p = parse("/m/Misc.Movies/Paper.Skies.1999.1080p.BluRay.x264-TAG.mkv");
        assert_eq!(p.title, "Paper Skies");
        assert_eq!(p.year, Some(1999));
    }

    #[test]
    fn a_folder_extension_is_never_stripped() {
        let p = parse("/m/Pioneer.One/Pioneer.One.(1977)/rip.avi");
        assert_eq!(p.year, Some(1977));
    }

    #[test]
    fn a_stray_file_under_a_season_folder_stays_unpaired() {
        let p = parse("/tv/Show/Show.Season.01/a stray recording.mkv");
        assert_eq!(p.season, None);
        assert_eq!(p.episode, None);
    }

    #[test]
    fn confidence_tiers() {
        let episodic = ParsedMedia {
            title: "Show".into(),
            year: None,
            season: Some(1),
            episode: Some(2),
            quality: None,
            external_id: None,
        };
        let movie_year = ParsedMedia {
            title: "Movie".into(),
            year: Some(2001),
            season: None,
            episode: None,
            quality: None,
            external_id: None,
        };
        let movie_bare = ParsedMedia {
            title: "Movie".into(),
            year: None,
            season: None,
            episode: None,
            quality: None,
            external_id: None,
        };
        let tagged = ParsedMedia {
            external_id: Some(ExternalId { source: "tmdb".into(), value: "movie/603".into() }),
            ..movie_bare.clone()
        };
        let empty = ParsedMedia {
            title: String::new(),
            year: Some(2001),
            season: None,
            episode: None,
            quality: None,
            external_id: None,
        };
        assert_eq!(confidence(&episodic), CONF_STRONG);
        assert_eq!(confidence(&movie_year), CONF_STRONG);
        assert_eq!(confidence(&movie_bare), CONF_WEAK);
        assert_eq!(confidence(&tagged), CONF_STRONG);
        assert_eq!(confidence(&empty), CONF_NONE);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(input in "\\PC*") {
            let _ = parse_filename(&input);
        }

        #[test]
        fn season_and_episode_are_paired(input in "\\PC*") {
            let parsed = parse_filename(&input);
            prop_assert_eq!(parsed.season.is_some(), parsed.episode.is_some());
        }

        #[test]
        fn year_within_supported_range(input in "\\PC*") {
            if let Some(year) = parse_filename(&input).year {
                prop_assert!((1900..=2099).contains(&year));
            }
        }

        #[test]
        fn title_has_no_edge_dashes_or_spaces(input in "\\PC*") {
            let title = parse_filename(&input).title;
            prop_assert_eq!(
                title.trim_matches(|c: char| c == '-' || c == ' '),
                title.as_str()
            );
        }

        #[test]
        fn a_directory_carrying_neither_year_nor_season_changes_nothing(
            name in "[^/\\\\]{0,40}",
            dir in "[a-z]{1,12}",
        ) {
            let path = format!("/{dir}/{name}");
            prop_assert_eq!(parse_filename(&path), parse_filename(&name));
        }

        #[test]
        fn a_season_folder_never_leaves_a_season_without_an_episode(
            name in "[^/\\\\]{0,40}",
            season in 1u16..40,
        ) {
            let path = format!("/show/Show.Season.{season:02}/{name}");
            let parsed = parse_filename(&path);
            prop_assert_eq!(parsed.season.is_some(), parsed.episode.is_some());
        }

    }
}
