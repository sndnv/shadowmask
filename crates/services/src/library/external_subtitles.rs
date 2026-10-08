use domain::common::LanguageCode;
use domain::library::DiscoveredSubtitle;
use domain::media::SubtitleFormat;

const FLAG_TAGS: &[&str] = &["sdh", "cc", "forced"];
const HEARING_IMPAIRED: &str = "hi";

pub fn discover_subtitles(video_path: &str, siblings: &[String]) -> Vec<DiscoveredSubtitle> {
    let stem = file_stem(video_path);
    siblings.iter().filter_map(|sibling| discover_one(stem, sibling)).collect()
}

pub fn sidecar_owner<'a>(sidecar: &str, videos: &[&'a str]) -> Option<&'a str> {
    videos
        .iter()
        .rev()
        .copied()
        .filter(|video| sidecar_suffix(file_stem(video), sidecar).is_some())
        .max_by_key(|video| file_stem(video).len())
}

fn sidecar_suffix<'a>(video_stem: &str, sibling: &'a str) -> Option<&'a str> {
    let (base, _) = basename(sibling).rsplit_once('.')?;
    let suffix = base.strip_prefix(video_stem)?;
    (suffix.is_empty() || suffix.starts_with(['.', ' ', '_', '-', '('])).then_some(suffix)
}

fn discover_one(video_stem: &str, sibling: &str) -> Option<DiscoveredSubtitle> {
    let (name, ext) = basename(sibling).rsplit_once('.')?;
    let format = SubtitleFormat::from_extension(ext)?;
    let language = match sidecar_suffix(video_stem, sibling) {
        Some(suffix) => language_of(suffix),
        None => language_of_whole_name(name),
    };
    Some(DiscoveredSubtitle { path: sibling.to_owned(), language, format })
}

fn chunks_of(text: &str) -> Vec<&str> {
    text.split(['.', ' ', '(', ')', '[', ']'])
        .map(|chunk| chunk.trim_matches(['_', '-']))
        .filter(|chunk| !chunk.is_empty())
        .collect()
}

fn language_of_whole_name(name: &str) -> Option<LanguageCode> {
    let tokens: Vec<&str> = chunks_of(name)
        .into_iter()
        .flat_map(|chunk| match LanguageCode::known(chunk) {
            Some(_) => vec![chunk],
            None => chunk.split(['_', '-']).filter(|word| !word.is_empty()).collect(),
        })
        .filter(|token| !is_flag(token))
        .collect();
    let trailing: Vec<&str> = tokens
        .iter()
        .rev()
        .skip_while(|token| is_number(token))
        .take_while(|token| !is_number(token))
        .copied()
        .collect();
    let code = match trailing.as_slice() {
        [last, before, ..] if last.eq_ignore_ascii_case(HEARING_IMPAIRED) => {
            LanguageCode::known(before).or_else(|| LanguageCode::known(last))
        }
        [last, ..] => LanguageCode::known(last),
        [] => None,
    };
    code.or_else(|| {
        (1..=trailing.len()).find_map(|count| {
            let words: Vec<&str> = trailing[..count].iter().rev().copied().collect();
            LanguageCode::from_name(&words.join(" "))
        })
    })
}

fn is_number(token: &str) -> bool {
    token.bytes().all(|b| b.is_ascii_digit())
}

fn language_of(text: &str) -> Option<LanguageCode> {
    let chunks = chunks_of(text);
    let words: Vec<&str> = chunks
        .iter()
        .flat_map(|chunk| chunk.split(['_', '-']))
        .filter(|word| !word.is_empty() && !is_flag(word) && !is_number(word))
        .collect();
    chunks
        .iter()
        .find_map(|chunk| LanguageCode::from_regional_tag(chunk))
        .or_else(|| chunks.iter().find_map(|chunk| language_code(chunk)))
        .or_else(|| match words.as_slice() {
            [word] => language_code(word),
            _ => None,
        })
        .or_else(|| LanguageCode::from_name(&words.join(" ")))
}

fn language_code(word: &str) -> Option<LanguageCode> {
    let is_code = (2..=3).contains(&word.len())
        && word.chars().all(|c| c.is_ascii_alphabetic())
        && !is_flag(word);
    is_code.then(|| LanguageCode::canonical(word))
}

fn is_flag(word: &str) -> bool {
    FLAG_TAGS.iter().any(|flag| flag.eq_ignore_ascii_case(word))
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
    fn a_three_letter_sidecar_tag_is_stored_as_two_letters() {
        let subs = discover_subtitles(
            "/m/Movie.mkv",
            &paths(&["/m/Movie.eng.srt", "/m/Movie.fre.srt", "/m/Movie.fil.srt"]),
        );
        let languages: Vec<_> =
            subs.iter().map(|s| s.language.as_ref().map(|l| l.0.as_str())).collect();
        assert_eq!(languages, [Some("en"), Some("fr"), Some("fil")]);
    }

    #[test]
    fn a_regional_sidecar_tag_keeps_its_region() {
        let subs = discover_subtitles(
            "/m/Movie.mkv",
            &paths(&[
                "/m/Movie.pt-BR.srt",
                "/m/Movie.pt_BR.srt",
                "/m/Movie.zh-tw.forced.srt",
                "/m/Movie.en-US.sdh.srt",
            ]),
        );
        let languages: Vec<_> =
            subs.iter().map(|s| s.language.as_ref().map(|l| l.0.as_str())).collect();
        assert_eq!(languages, [Some("pt-BR"), Some("pt-BR"), Some("zh-TW"), Some("en-US")]);
    }

    #[test]
    fn a_hyphenated_word_is_not_a_language() {
        let subs = discover_subtitles("/m/Movie.mkv", &paths(&["/m/Movie.re-cut.srt"]));
        assert_eq!(subs[0].language, None);
    }

    #[test]
    fn discovers_sidecar_without_language() {
        let subs = discover_subtitles("/m/Movie.mkv", &paths(&["/m/Movie.srt"]));
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].language, None);
        assert_eq!(subs[0].format, SubtitleFormat::Srt);
    }

    #[test]
    fn ignores_files_that_are_not_subtitles() {
        let subs = discover_subtitles("/m/Movie.mkv", &paths(&["/m/Movie.mkv", "/m/Movie.nfo"]));
        assert!(subs.is_empty());
    }

    fn languages(video: &str, names: &[&str]) -> Vec<Option<String>> {
        discover_subtitles(video, &paths(names))
            .into_iter()
            .map(|subtitle| subtitle.language.map(|code| code.0))
            .collect()
    }

    fn expected(codes: &[Option<&str>]) -> Vec<Option<String>> {
        codes.iter().map(|code| code.map(str::to_owned)).collect()
    }

    #[test]
    fn a_sibling_handed_over_without_the_video_name_is_read_by_its_whole_name() {
        assert_eq!(
            languages("/m/Movie.mkv", &["/m/Other.en.srt", "/m/Movieish.fr.srt"]),
            expected(&[Some("en"), Some("fr")])
        );
    }

    #[test]
    fn the_video_name_may_end_in_a_space_underscore_hyphen_or_bracket() {
        assert_eq!(
            languages(
                "/m/Movie.mkv",
                &[
                    "/m/Movie ENG.srt",
                    "/m/Movie_fr.srt",
                    "/m/Movie-pt-BR.srt",
                    "/m/Movie(es).srt",
                    "/m/Movie.English.srt",
                ]
            ),
            expected(&[Some("en"), Some("fr"), Some("pt-BR"), Some("es"), Some("en")])
        );
    }

    #[test]
    fn a_file_named_only_by_its_language_reads_that_language() {
        assert_eq!(
            languages(
                "/m/Film/Film.mkv",
                &[
                    "/m/Film/Subs/English.srt",
                    "/m/Film/Subs/2_English.srt",
                    "/m/Film/Subs/English(SDH).srt",
                    "/m/Film/Subs/English_FORCED.srt",
                    "/m/Film/Subs/Danish.srt",
                    "/m/Film/Subs/eng.srt",
                    "/m/Film/Subs/3_ger.srt",
                    "/m/Film/Subs/pt_BR.srt",
                ]
            ),
            expected(&[
                Some("en"),
                Some("en"),
                Some("en"),
                Some("en"),
                Some("da"),
                Some("en"),
                Some("de"),
                Some("pt-BR"),
            ])
        );
    }

    #[test]
    fn a_release_name_reads_the_language_at_its_end_not_a_title_word() {
        assert_eq!(
            languages(
                "/m/Film (2022)/Film (2022).mkv",
                &[
                    "/m/Film (2022)/The.Batman.2022.1080p.WEB.en.srt",
                    "/m/Film (2022)/It.Chapter.Two.2019.ENG.srt",
                    "/m/Film (2022)/Her.2013.en.srt",
                    "/m/Film (2022)/Her.en.srt",
                    "/m/Film (2022)/Up.2009.English.srt",
                    "/m/Film (2022)/Up.2009.English.forced.srt",
                    "/m/Film (2022)/Movie.2019.pt_BR.srt",
                ]
            ),
            expected(&[
                Some("en"),
                Some("en"),
                Some("en"),
                Some("en"),
                Some("en"),
                Some("en"),
                Some("pt-BR"),
            ])
        );
    }

    #[test]
    fn hi_after_a_language_marks_hearing_impaired_and_alone_is_hindi() {
        assert_eq!(
            languages(
                "/m/Film/Film.mkv",
                &["/m/Film/Release.2019.en.hi.srt", "/m/Film/Release.2019.hi.srt"]
            ),
            expected(&[Some("en"), Some("hi")])
        );
    }

    #[test]
    fn a_release_name_without_a_known_language_reads_unknown() {
        assert_eq!(
            languages(
                "/m/Film/Film.mkv",
                &[
                    "/m/Film/Movie.HD-TV.srt",
                    "/m/Film/Release.2022.1080p.WEB.srt",
                    "/m/Film/Movie.DTS.srt",
                    "/m/Film/Movie.Up.srt",
                    "/m/Film/2019.srt",
                    "/m/Film/Release.2019.fil.srt",
                ]
            ),
            expected(&[None, None, None, None, None, None])
        );
    }

    #[test]
    fn a_name_isolang_does_not_know_reads_unknown() {
        assert_eq!(
            languages(
                "/m/Film/Film.mkv",
                &["/m/Film/Subs/Portuguese - Brazilian.srt", "/m/Film/Subs/Greek.srt"]
            ),
            expected(&[None, None])
        );
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
