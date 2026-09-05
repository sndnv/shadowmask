use crate::common::LanguageCode;
use crate::media::{AudioTrack, EmbeddedSubtitleTrack, SubtitleFile};
use crate::negotiation::{preferred_audio_track, preferred_subtitle_track};
use crate::playback::{SubtitleOverride, SubtitleTrackRef};
use crate::session::{AudioRequest, SubtitleRequest, SubtitleSelection};

pub struct AvailableSubtitles<'a> {
    pub embedded: &'a [EmbeddedSubtitleTrack],
    pub files: &'a [SubtitleFile],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAudio {
    pub selected: Option<u32>,
    pub remembered: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSubtitle {
    pub selected: Option<SubtitleSelection>,
    pub remembered: Option<SubtitleOverride>,
}

pub fn resolve_audio(
    request: &AudioRequest,
    stored: Option<u32>,
    tracks: &[AudioTrack],
    preferred: &[LanguageCode],
) -> ResolvedAudio {
    if let Some(index) = requested_audio(request, tracks) {
        return ResolvedAudio {
            selected: Some(index),
            remembered: Some(index),
        };
    }
    if let Some(index) = stored.filter(|index| has_audio(tracks, *index)) {
        return ResolvedAudio {
            selected: Some(index),
            remembered: Some(index),
        };
    }
    ResolvedAudio {
        selected: preferred_audio_track(tracks, preferred),
        remembered: None,
    }
}

pub fn resolve_subtitle(
    request: &SubtitleRequest,
    stored: Option<&SubtitleOverride>,
    available: &AvailableSubtitles<'_>,
    preferred: &[LanguageCode],
) -> ResolvedSubtitle {
    if let Some(resolved) = requested_subtitle(request, available) {
        return resolved;
    }
    match stored {
        Some(SubtitleOverride::Off) => off(),
        Some(SubtitleOverride::Track(track)) if present(track, available) => chosen(track.clone()),
        _ => ResolvedSubtitle {
            selected: by_language(preferred, available).map(selection),
            remembered: None,
        },
    }
}

fn requested_audio(request: &AudioRequest, tracks: &[AudioTrack]) -> Option<u32> {
    match request {
        AudioRequest::Unspecified => None,
        AudioRequest::Track(index) => has_audio(tracks, *index).then_some(*index),
        AudioRequest::Language(language) => {
            preferred_audio_track(tracks, std::slice::from_ref(language))
        }
    }
}

fn requested_subtitle(
    request: &SubtitleRequest,
    available: &AvailableSubtitles<'_>,
) -> Option<ResolvedSubtitle> {
    match request {
        SubtitleRequest::Unspecified => None,
        SubtitleRequest::Off => Some(off()),
        SubtitleRequest::Track(selection) => {
            present(&selection.track, available).then(|| ResolvedSubtitle {
                remembered: Some(SubtitleOverride::Track(selection.track.clone())),
                selected: Some(selection.clone()),
            })
        }
        SubtitleRequest::Language(language) => {
            by_language(std::slice::from_ref(language), available).map(chosen)
        }
    }
}

fn by_language(
    preferred: &[LanguageCode],
    available: &AvailableSubtitles<'_>,
) -> Option<SubtitleTrackRef> {
    if let Some(index) = preferred_subtitle_track(available.embedded, preferred) {
        return Some(SubtitleTrackRef::Embedded(index));
    }
    preferred.iter().find_map(|want| {
        available
            .files
            .iter()
            .find(|file| {
                file.language
                    .as_ref()
                    .is_some_and(|code| code.0.eq_ignore_ascii_case(&want.0))
            })
            .map(|file| SubtitleTrackRef::File(file.id.clone()))
    })
}

fn present(track: &SubtitleTrackRef, available: &AvailableSubtitles<'_>) -> bool {
    match track {
        SubtitleTrackRef::Embedded(index) => {
            available.embedded.iter().any(|track| track.index == *index)
        }
        SubtitleTrackRef::File(id) => available.files.iter().any(|file| file.id == *id),
    }
}

fn has_audio(tracks: &[AudioTrack], index: u32) -> bool {
    tracks.iter().any(|track| track.index == index)
}

fn selection(track: SubtitleTrackRef) -> SubtitleSelection {
    SubtitleSelection {
        track,
        offset_ms: None,
    }
}

fn chosen(track: SubtitleTrackRef) -> ResolvedSubtitle {
    ResolvedSubtitle {
        remembered: Some(SubtitleOverride::Track(track.clone())),
        selected: Some(selection(track)),
    }
}

fn off() -> ResolvedSubtitle {
    ResolvedSubtitle {
        selected: None,
        remembered: Some(SubtitleOverride::Off),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::VersionId;
    use crate::media::{SubtitleFileId, SubtitleFormat, SubtitleSource};

    fn want(codes: &[&str]) -> Vec<LanguageCode> {
        codes.iter().map(|c| LanguageCode((*c).into())).collect()
    }

    fn language(code: &str) -> LanguageCode {
        LanguageCode(code.into())
    }

    fn audio(index: u32, spoken: Option<&str>) -> AudioTrack {
        AudioTrack {
            index,
            codec: "aac".into(),
            channels: 2,
            language: spoken.map(language),
            bitrate: None,
        }
    }

    fn subtitle(index: u32, spoken: Option<&str>) -> EmbeddedSubtitleTrack {
        EmbeddedSubtitleTrack {
            index,
            language: spoken.map(language),
            format: SubtitleFormat::Srt,
            forced: false,
            default: false,
        }
    }

    fn file(id: &str, spoken: Option<&str>) -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId(id.into()),
            version: VersionId("v1".into()),
            language: spoken.map(language),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: format!("/subs/{id}.srt"),
            translated_from: None,
            label: None,
            pinned: false,
        }
    }

    fn embedded(tracks: &[EmbeddedSubtitleTrack]) -> AvailableSubtitles<'_> {
        AvailableSubtitles {
            embedded: tracks,
            files: &[],
        }
    }

    #[test]
    fn an_explicit_audio_track_wins_and_is_remembered() {
        let tracks = vec![audio(0, Some("eng")), audio(1, Some("jpn"))];
        let resolved = resolve_audio(&AudioRequest::Track(1), Some(0), &tracks, &want(&["eng"]));

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, Some(1));
    }

    #[test]
    fn a_carried_language_beats_the_stored_override() {
        let tracks = vec![audio(0, Some("eng")), audio(1, Some("fra"))];
        let resolved = resolve_audio(
            &AudioRequest::Language(language("fra")),
            Some(0),
            &tracks,
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, Some(1));
    }

    #[test]
    fn the_stored_override_beats_the_account_preference() {
        let tracks = vec![audio(0, Some("eng")), audio(1, Some("jpn"))];
        let resolved = resolve_audio(
            &AudioRequest::Unspecified,
            Some(1),
            &tracks,
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, Some(1));
    }

    // The account preference must never harden into an override, or it would stop
    // tracking the account setting from the next episode onward.
    #[test]
    fn the_account_preference_is_used_but_not_remembered() {
        let tracks = vec![audio(0, Some("jpn")), audio(1, Some("eng"))];
        let resolved = resolve_audio(&AudioRequest::Unspecified, None, &tracks, &want(&["eng"]));

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn a_requested_audio_track_that_is_absent_falls_through() {
        let tracks = vec![audio(0, Some("jpn")), audio(1, Some("eng"))];
        let resolved = resolve_audio(&AudioRequest::Track(9), None, &tracks, &want(&["eng"]));

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn a_carried_language_that_is_absent_falls_through_to_the_account() {
        let tracks = vec![audio(0, Some("jpn")), audio(1, Some("eng"))];
        let resolved = resolve_audio(
            &AudioRequest::Language(language("fra")),
            None,
            &tracks,
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, Some(1));
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn a_stored_audio_override_pointing_at_a_missing_track_falls_through() {
        let tracks = vec![audio(0, Some("eng"))];
        let resolved = resolve_audio(
            &AudioRequest::Unspecified,
            Some(7),
            &tracks,
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, Some(0));
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn subtitles_switched_off_are_remembered_as_off() {
        let tracks = vec![subtitle(2, Some("eng"))];
        let resolved = resolve_subtitle(
            &SubtitleRequest::Off,
            None,
            &embedded(&tracks),
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, None);
        assert_eq!(resolved.remembered, Some(SubtitleOverride::Off));
    }

    // Off must beat the account preference, or turning subtitles off would not stick.
    #[test]
    fn a_stored_off_override_suppresses_the_account_preference() {
        let tracks = vec![subtitle(2, Some("eng"))];
        let resolved = resolve_subtitle(
            &SubtitleRequest::Unspecified,
            Some(&SubtitleOverride::Off),
            &embedded(&tracks),
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, None);
        assert_eq!(resolved.remembered, Some(SubtitleOverride::Off));
    }

    #[test]
    fn a_carried_subtitle_language_matches_an_embedded_track() {
        let tracks = vec![subtitle(2, Some("eng")), subtitle(3, Some("fra"))];
        let resolved = resolve_subtitle(
            &SubtitleRequest::Language(language("fra")),
            None,
            &embedded(&tracks),
            &want(&["eng"]),
        );

        assert_eq!(
            resolved.remembered,
            Some(SubtitleOverride::Track(SubtitleTrackRef::Embedded(3)))
        );
    }

    // An episode may carry a language only as a sidecar file, so both lists are searched.
    #[test]
    fn a_carried_subtitle_language_falls_back_to_a_sidecar_file() {
        let tracks = vec![subtitle(2, Some("eng"))];
        let files = vec![file("sf1", Some("fra"))];
        let resolved = resolve_subtitle(
            &SubtitleRequest::Language(language("fra")),
            None,
            &AvailableSubtitles {
                embedded: &tracks,
                files: &files,
            },
            &want(&["eng"]),
        );

        assert_eq!(
            resolved.remembered,
            Some(SubtitleOverride::Track(SubtitleTrackRef::File(
                SubtitleFileId("sf1".into())
            )))
        );
    }

    #[test]
    fn a_carried_subtitle_language_the_episode_lacks_falls_through() {
        let tracks = vec![subtitle(2, Some("eng"))];
        let resolved = resolve_subtitle(
            &SubtitleRequest::Language(language("fra")),
            None,
            &embedded(&tracks),
            &want(&["eng"]),
        );

        assert_eq!(
            resolved.selected.map(|selection| selection.track),
            Some(SubtitleTrackRef::Embedded(2))
        );
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn a_stored_subtitle_file_that_is_gone_falls_through() {
        let tracks = vec![subtitle(2, Some("eng"))];
        let stored = SubtitleOverride::Track(SubtitleTrackRef::File(SubtitleFileId("gone".into())));
        let resolved = resolve_subtitle(
            &SubtitleRequest::Unspecified,
            Some(&stored),
            &embedded(&tracks),
            &want(&["eng"]),
        );

        assert_eq!(
            resolved.selected.map(|selection| selection.track),
            Some(SubtitleTrackRef::Embedded(2))
        );
        assert_eq!(resolved.remembered, None);
    }

    #[test]
    fn nothing_matches_and_nothing_is_selected() {
        let resolved = resolve_subtitle(
            &SubtitleRequest::Unspecified,
            None,
            &embedded(&[]),
            &want(&["eng"]),
        );

        assert_eq!(resolved.selected, None);
        assert_eq!(resolved.remembered, None);
    }
}
