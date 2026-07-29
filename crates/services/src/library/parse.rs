use std::path::Path;
use std::sync::LazyLock;

use domain::common::Quality;
use domain::library::ParsedMedia;
use regex::Regex;

const CONF_STRONG: f32 = 0.9;
const CONF_WEAK: f32 = 0.4;
const CONF_NONE: f32 = 0.0;

static SEASON_EPISODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bs([0-9]{1,2})e([0-9]{1,3})").unwrap());
static SEASON_EPISODE_X: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([0-9]{1,2})x([0-9]{1,3})\b").unwrap());
static YEAR_PAREN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[(\[]((?:19|20)[0-9]{2})[)\]]").unwrap());
static YEAR_BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:19|20)[0-9]{2}\b").unwrap());
static JUNK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(480p|576p|720p|1080p|2160p|4k|bluray|blu-ray|bdrip|brrip|webrip|web-dl|webdl|hdtv|dvdrip|x264|x265|h264|h265|hevc|aac|ac3|dts|xvid|remux)\b",
    )
    .unwrap()
});
static RESOLUTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(480p|576p|720p|1080p|2160p|4k)\b").unwrap());

pub fn parse_filename(path: &str) -> ParsedMedia {
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = basename.rsplit_once('.').map_or(basename, |(s, _)| s);

    let season_episode = find_season_episode(stem);
    let year = find_year(stem);
    let junk = JUNK.find(stem).map(|m| m.start());

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

    ParsedMedia {
        title: clean_title(&stem[..cut]),
        year: year.map(|(y, _)| y),
        season: season_episode.map(|(s, _, _)| s),
        episode: season_episode.map(|(_, e, _)| e),
        quality: find_quality(stem),
    }
}

pub fn confidence(parsed: &ParsedMedia) -> f32 {
    if parsed.title.is_empty() {
        CONF_NONE
    } else if parsed.is_episodic() || parsed.year.is_some() {
        CONF_STRONG
    } else {
        CONF_WEAK
    }
}

fn find_season_episode(stem: &str) -> Option<(u16, u16, usize)> {
    let captures = SEASON_EPISODE
        .captures(stem)
        .or_else(|| SEASON_EPISODE_X.captures(stem))?;
    let whole = captures.get(0).unwrap();
    let season = captures.get(1).unwrap().as_str().parse().unwrap();
    let episode = captures.get(2).unwrap().as_str().parse().unwrap();
    Some((season, episode, whole.start()))
}

fn find_year(stem: &str) -> Option<(u16, usize)> {
    if let Some(captures) = YEAR_PAREN.captures(stem) {
        let year = captures.get(1).unwrap().as_str().parse().unwrap();
        return Some((year, captures.get(0).unwrap().start()));
    }
    let matched = YEAR_BARE.find(stem)?;
    Some((matched.as_str().parse().unwrap(), matched.start()))
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
    RESOLUTION
        .replace_all(stem, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn upscaled_output_path(source_path: &str, target: Quality, uuid: &str) -> String {
    let src = Path::new(source_path);
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let base = strip_quality_token(stem);
    let name = format!("{base} {} [Upscaled {uuid}].mp4", quality_token(target));
    src.parent()
        .unwrap_or(Path::new(""))
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn clean_title(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|c| if c == '.' || c == '_' { ' ' } else { c })
        .collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '-' || c == ' ')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str) -> ParsedMedia {
        parse_filename(name)
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
        let out = upscaled_output_path(
            "/m/Movie (2011)/Movie (2011) 480p.mkv",
            Quality::Fhd,
            "abcd1234",
        );
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
        let out = upscaled_output_path(
            "/tv/Show/Season 01/Show S01E02 720p.mkv",
            Quality::Fhd,
            "zz",
        );
        let p = parse(&out);
        assert_eq!(p.title, "Show");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episode, Some(2));
        assert_eq!(p.quality, Some(Quality::Fhd));
    }

    #[test]
    fn parses_dotted_movie_with_year_and_resolution() {
        let p = parse("/m/The.Matrix.1999.1080p.BluRay.x264-GRP.mkv");
        assert_eq!(p.title, "The Matrix");
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
    fn confidence_tiers() {
        let episodic = ParsedMedia {
            title: "Show".into(),
            year: None,
            season: Some(1),
            episode: Some(2),
            quality: None,
        };
        let movie_year = ParsedMedia {
            title: "Movie".into(),
            year: Some(2001),
            season: None,
            episode: None,
            quality: None,
        };
        let movie_bare = ParsedMedia {
            title: "Movie".into(),
            year: None,
            season: None,
            episode: None,
            quality: None,
        };
        let empty = ParsedMedia {
            title: String::new(),
            year: Some(2001),
            season: None,
            episode: None,
            quality: None,
        };
        assert_eq!(confidence(&episodic), CONF_STRONG);
        assert_eq!(confidence(&movie_year), CONF_STRONG);
        assert_eq!(confidence(&movie_bare), CONF_WEAK);
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
        fn directory_prefix_is_ignored(name in "[^/\\\\]{0,40}", dir in "[a-z0-9/]{0,20}") {
            let path = format!("/{dir}/{name}");
            prop_assert_eq!(parse_filename(&path), parse_filename(&name));
        }

    }
}
