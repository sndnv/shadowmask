use crate::common::LanguageCode;
use crate::media::{AudioTrack, EmbeddedSubtitleTrack};

pub fn preferred_audio_track(tracks: &[AudioTrack], preferred: &[LanguageCode]) -> Option<u32> {
    preferred.iter().find_map(|want| {
        tracks.iter().find(|track| spoken(track.language.as_ref(), want)).map(|track| track.index)
    })
}

pub fn preferred_subtitle_track(
    tracks: &[EmbeddedSubtitleTrack],
    preferred: &[LanguageCode],
) -> Option<u32> {
    preferred.iter().find_map(|want| {
        tracks
            .iter()
            .find(|track| !track.forced && spoken(track.language.as_ref(), want))
            .or_else(|| tracks.iter().find(|track| spoken(track.language.as_ref(), want)))
            .map(|track| track.index)
    })
}

fn spoken(language: Option<&LanguageCode>, want: &LanguageCode) -> bool {
    language.is_some_and(|code| code.0.eq_ignore_ascii_case(&want.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::SubtitleFormat;

    fn audio(index: u32, language: Option<&str>) -> AudioTrack {
        AudioTrack {
            index,
            codec: "aac".into(),
            channels: 2,
            language: language.map(|l| LanguageCode(l.into())),
            bitrate: None,
        }
    }

    fn subtitle(index: u32, language: Option<&str>, forced: bool) -> EmbeddedSubtitleTrack {
        EmbeddedSubtitleTrack {
            index,
            language: language.map(|l| LanguageCode(l.into())),
            format: SubtitleFormat::Srt,
            forced,
            default: false,
        }
    }

    fn want(codes: &[&str]) -> Vec<LanguageCode> {
        codes.iter().map(|c| LanguageCode((*c).into())).collect()
    }

    #[test]
    fn the_first_listed_language_that_exists_wins() {
        let tracks = vec![audio(0, Some("eng")), audio(1, Some("jpn"))];

        assert_eq!(preferred_audio_track(&tracks, &want(&["jpn", "eng"])), Some(1));
        assert_eq!(preferred_audio_track(&tracks, &want(&["fra", "eng"])), Some(0));
    }

    #[test]
    fn language_matching_ignores_case() {
        let tracks = vec![audio(3, Some("ENG"))];

        assert_eq!(preferred_audio_track(&tracks, &want(&["eng"])), Some(3));
    }

    #[test]
    fn no_preference_and_no_match_both_select_nothing() {
        let tracks = vec![audio(0, Some("eng")), audio(1, None)];

        assert_eq!(preferred_audio_track(&tracks, &[]), None);
        assert_eq!(preferred_audio_track(&tracks, &want(&["fra"])), None);
        assert_eq!(preferred_audio_track(&[], &want(&["eng"])), None);
    }

    #[test]
    fn a_full_subtitle_track_is_preferred_over_a_forced_one() {
        let tracks = vec![subtitle(0, Some("eng"), true), subtitle(1, Some("eng"), false)];

        assert_eq!(preferred_subtitle_track(&tracks, &want(&["eng"])), Some(1));
    }

    #[test]
    fn a_forced_track_is_taken_when_it_is_the_only_one() {
        let tracks = vec![subtitle(4, Some("eng"), true)];

        assert_eq!(preferred_subtitle_track(&tracks, &want(&["eng"])), Some(4));
        assert_eq!(preferred_subtitle_track(&tracks, &want(&["fra"])), None);
    }
}
