use domain::common::LanguageCode;
use domain::library::DiscoveredSubtitle;
use domain::media::SubtitleFormat;

const FLAG_TAGS: &[&str] = &["sdh", "cc", "forced"];

pub fn discover_subtitles(video_path: &str, siblings: &[String]) -> Vec<DiscoveredSubtitle> {
    let stem = file_stem(video_path);
    siblings
        .iter()
        .filter_map(|sibling| discover_one(stem, sibling))
        .collect()
}

fn discover_one(video_stem: &str, sibling: &str) -> Option<DiscoveredSubtitle> {
    let name = basename(sibling);
    let (base, ext) = name.rsplit_once('.')?;
    let format = SubtitleFormat::from_extension(ext)?;
    let suffix = base.strip_prefix(video_stem)?;
    if !suffix.is_empty() && !suffix.starts_with('.') {
        return None;
    }
    let language = suffix
        .split('.')
        .find_map(parse_language_tag)
        .map(LanguageCode);
    Some(DiscoveredSubtitle {
        path: sibling.to_owned(),
        language,
        format,
    })
}

fn parse_language_tag(token: &str) -> Option<String> {
    let lower = token.to_ascii_lowercase();
    let is_code = (2..=3).contains(&lower.len()) && lower.chars().all(|c| c.is_ascii_alphabetic());
    if is_code && !FLAG_TAGS.contains(&lower.as_str()) {
        Some(lower)
    } else {
        None
    }
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn file_stem(path: &str) -> &str {
    let base = basename(path);
    base.rsplit_once('.').map_or(base, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn discovers_sidecar_with_language() {
        let subs = discover_subtitles("/m/Movie.mkv", &paths(&["/m/Movie.en.srt"]));
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].path, "/m/Movie.en.srt");
        assert_eq!(subs[0].language, Some(LanguageCode("en".to_owned())));
        assert_eq!(subs[0].format, SubtitleFormat::Srt);
    }

    #[test]
    fn discovers_sidecar_without_language() {
        let subs = discover_subtitles("/m/Movie.mkv", &paths(&["/m/Movie.srt"]));
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].language, None);
        assert_eq!(subs[0].format, SubtitleFormat::Srt);
    }

    #[test]
    fn ignores_other_titles_and_non_subtitle_files() {
        let subs = discover_subtitles(
            "/m/Movie.mkv",
            &paths(&[
                "/m/Movie.mkv",
                "/m/Movie.nfo",
                "/m/Other.en.srt",
                "/m/Movieish.en.srt",
            ]),
        );
        assert!(subs.is_empty());
    }

    #[test]
    fn parses_language_ignoring_flags_and_mixed_formats() {
        let subs = discover_subtitles(
            "/m/Movie.mkv",
            &paths(&[
                "/m/Movie.en.srt",
                "/m/Movie.es.forced.srt",
                "/m/Movie.sdh.srt",
                "/m/Movie.vtt",
            ]),
        );
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[0].language, Some(LanguageCode("en".to_owned())));
        assert_eq!(subs[1].language, Some(LanguageCode("es".to_owned())));
        assert_eq!(subs[2].language, None);
        assert_eq!(subs[3].language, None);
        assert_eq!(subs[3].format, SubtitleFormat::Vtt);
    }
}
