use crate::media::{AudioTrack, EmbeddedSubtitleTrack, SubtitleFormat, VideoTrack};
use crate::negotiation::{NegotiationInput, NegotiationOutcome};
use crate::playback::SubtitleTrackRef;
use crate::profile::CapabilityProfile;
use crate::session::{DeliveryMode, SelectedTracks, SubtitleDelivery, SubtitleSelection};

pub fn negotiate(input: &NegotiationInput, profile: &CapabilityProfile) -> NegotiationOutcome {
    let video = input.video.first();
    let audio = selected_audio(input);
    let subtitle_delivery = input
        .requested_subtitle
        .as_ref()
        .map(|sel| subtitle_delivery_for(sel, &input.subtitles));

    let video_ok = video.is_none_or(|v| video_supported(v, profile, input.max_bitrate));
    let audio_ok = audio.is_none_or(|a| audio_supported(a, profile));
    let subtitle_burn = subtitle_delivery == Some(SubtitleDelivery::Burned);
    let container_ok = profile.containers.contains(&input.container);

    let mode = if !video_ok || !audio_ok || subtitle_burn {
        DeliveryMode::Transcode
    } else if !container_ok {
        DeliveryMode::Remux
    } else {
        DeliveryMode::Direct
    };

    NegotiationOutcome {
        mode,
        selected: SelectedTracks {
            audio_track: audio.map(|a| a.index),
            subtitle_track: input.requested_subtitle.as_ref().map(|s| s.track.clone()),
            subtitle_delivery,
        },
    }
}

fn selected_audio(input: &NegotiationInput) -> Option<&AudioTrack> {
    match input.requested_audio {
        Some(idx) => input.audio.iter().find(|a| a.index == idx),
        None => input.audio.first(),
    }
}

fn video_supported(v: &VideoTrack, profile: &CapabilityProfile, user_cap: Option<u64>) -> bool {
    let Some(cap) = profile.video.iter().find(|c| c.codec == v.codec) else {
        return false;
    };
    let bitrate_ok = v
        .bitrate
        .is_none_or(|b| b <= effective_cap(profile.max_bitrate, user_cap));
    cap.max_bit_depth >= v.bit_depth
        && v.width <= profile.max_width
        && v.height <= profile.max_height
        && v.hdr.is_none_or(|h| profile.hdr.contains(&h))
        && bitrate_ok
}

fn audio_supported(a: &AudioTrack, profile: &CapabilityProfile) -> bool {
    profile
        .audio
        .iter()
        .any(|c| c.codec == a.codec && a.channels <= c.max_channels)
}

fn effective_cap(profile_cap: u64, user_cap: Option<u64>) -> u64 {
    match user_cap {
        Some(user) => profile_cap.min(user),
        None => profile_cap,
    }
}

fn subtitle_delivery_for(
    sel: &SubtitleSelection,
    subtitles: &[EmbeddedSubtitleTrack],
) -> SubtitleDelivery {
    match &sel.track {
        SubtitleTrackRef::Embedded(idx) => {
            let image = subtitles
                .iter()
                .find(|s| s.index == *idx)
                .is_some_and(|s| matches!(s.format, SubtitleFormat::Pgs | SubtitleFormat::VobSub));
            if image {
                SubtitleDelivery::Burned
            } else {
                SubtitleDelivery::HlsVtt
            }
        }
        SubtitleTrackRef::File(_) => SubtitleDelivery::HlsVtt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{HdrFormat, SubtitleFileId};
    use crate::profile::{AudioCodecCap, Container, VideoCodecCap};

    fn video_track(
        codec: &str,
        width: u32,
        height: u32,
        bit_depth: u8,
        hdr: Option<HdrFormat>,
        bitrate: Option<u64>,
    ) -> VideoTrack {
        VideoTrack {
            index: 0,
            codec: codec.to_owned(),
            width,
            height,
            bit_depth,
            hdr,
            frame_rate: 24.0,
            bitrate,
        }
    }

    fn audio_track(codec: &str, channels: u8, index: u32) -> AudioTrack {
        AudioTrack {
            index,
            codec: codec.to_owned(),
            channels,
            language: None,
            bitrate: None,
        }
    }

    fn subtitle(index: u32, format: SubtitleFormat) -> EmbeddedSubtitleTrack {
        EmbeddedSubtitleTrack {
            index,
            language: None,
            format,
            forced: false,
            default: false,
        }
    }

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4, Container::Hls],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
            }],
            audio: vec![AudioCodecCap {
                codec: "aac".to_owned(),
                max_channels: 2,
            }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
        }
    }

    fn input() -> NegotiationInput {
        NegotiationInput {
            container: Container::Mp4,
            video: vec![video_track("h264", 1920, 1080, 8, None, Some(5_000_000))],
            audio: vec![audio_track("aac", 2, 1)],
            subtitles: vec![],
            requested_audio: None,
            requested_subtitle: None,
            max_bitrate: None,
        }
    }

    fn embedded(idx: u32) -> SubtitleSelection {
        SubtitleSelection {
            track: SubtitleTrackRef::Embedded(idx),
            offset_ms: None,
        }
    }

    #[test]
    fn direct_when_everything_matches() {
        let out = negotiate(&input(), &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.audio_track, Some(1));
        assert_eq!(out.selected.subtitle_delivery, None);
    }

    #[test]
    fn remux_when_only_container_unsupported() {
        let i = NegotiationInput {
            container: Container::Mkv,
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Remux);
    }

    #[test]
    fn transcode_when_video_codec_unsupported() {
        let i = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn transcode_when_bit_depth_too_high() {
        let i = NegotiationInput {
            video: vec![video_track("h264", 1920, 1080, 10, None, Some(5_000_000))],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn transcode_when_resolution_too_high() {
        let i = NegotiationInput {
            video: vec![video_track("h264", 3840, 2160, 8, None, Some(5_000_000))],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn transcode_when_hdr_unsupported() {
        let i = NegotiationInput {
            video: vec![video_track(
                "h264",
                1920,
                1080,
                8,
                Some(HdrFormat::Hdr10),
                Some(5_000_000),
            )],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn direct_when_hdr_supported() {
        let p = CapabilityProfile {
            hdr: vec![HdrFormat::Hdr10],
            ..profile()
        };
        let i = NegotiationInput {
            video: vec![video_track(
                "h264",
                1920,
                1080,
                8,
                Some(HdrFormat::Hdr10),
                Some(5_000_000),
            )],
            ..input()
        };
        assert_eq!(negotiate(&i, &p).mode, DeliveryMode::Direct);
    }

    #[test]
    fn transcode_when_bitrate_over_profile_cap() {
        let i = NegotiationInput {
            video: vec![video_track("h264", 1920, 1080, 8, None, Some(20_000_000))],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn transcode_when_bitrate_over_user_cap() {
        let i = NegotiationInput {
            max_bitrate: Some(3_000_000),
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn direct_when_bitrate_unknown() {
        let i = NegotiationInput {
            video: vec![video_track("h264", 1920, 1080, 8, None, None)],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Direct);
    }

    #[test]
    fn transcode_when_audio_codec_unsupported() {
        let i = NegotiationInput {
            audio: vec![audio_track("eac3", 2, 1)],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn transcode_when_audio_channels_exceed() {
        let i = NegotiationInput {
            audio: vec![audio_track("aac", 6, 1)],
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn text_subtitle_uses_hls_vtt_without_transcode() {
        let i = NegotiationInput {
            subtitles: vec![subtitle(2, SubtitleFormat::Srt)],
            requested_subtitle: Some(embedded(2)),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(
            out.selected.subtitle_track,
            Some(SubtitleTrackRef::Embedded(2))
        );
        assert_eq!(
            out.selected.subtitle_delivery,
            Some(SubtitleDelivery::HlsVtt)
        );
    }

    #[test]
    fn image_subtitle_forces_burn_and_transcode() {
        let i = NegotiationInput {
            subtitles: vec![subtitle(3, SubtitleFormat::Pgs)],
            requested_subtitle: Some(embedded(3)),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(
            out.selected.subtitle_delivery,
            Some(SubtitleDelivery::Burned)
        );
    }

    #[test]
    fn external_subtitle_file_uses_hls_vtt() {
        let i = NegotiationInput {
            requested_subtitle: Some(SubtitleSelection {
                track: SubtitleTrackRef::File(SubtitleFileId("ext1".to_owned())),
                offset_ms: None,
            }),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(
            out.selected.subtitle_delivery,
            Some(SubtitleDelivery::HlsVtt)
        );
    }

    #[test]
    fn requested_audio_index_selects_that_track() {
        let i = NegotiationInput {
            audio: vec![audio_track("aac", 2, 1), audio_track("ac3", 2, 2)],
            requested_audio: Some(2),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.selected.audio_track, Some(2));
        assert_eq!(out.mode, DeliveryMode::Transcode);
    }

    #[test]
    fn requested_audio_index_absent_selects_none() {
        let i = NegotiationInput {
            requested_audio: Some(99),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.selected.audio_track, None);
        assert_eq!(out.mode, DeliveryMode::Direct);
    }

    #[test]
    fn no_video_is_supported() {
        let i = NegotiationInput {
            video: vec![],
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.audio_track, Some(1));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::profile::{AudioCodecCap, Container, VideoCodecCap};
    use proptest::prelude::*;

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4, Container::Hls],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
            }],
            audio: vec![AudioCodecCap {
                codec: "aac".to_owned(),
                max_channels: 2,
            }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
        }
    }

    prop_compose! {
        fn inputs()(
            container in prop_oneof![Just(Container::Mp4), Just(Container::Hls), Just(Container::Mkv)],
            vcodec in prop_oneof![Just("h264"), Just("vp9")],
            width in prop_oneof![Just(1280u32), Just(1920u32), Just(3840u32)],
            bit_depth in prop_oneof![Just(8u8), Just(10u8)],
            bitrate in prop::option::of(1_000_000u64..=30_000_000u64),
            acodec in prop_oneof![Just("aac"), Just("eac3")],
            channels in prop_oneof![Just(2u8), Just(6u8)],
            requested_audio in prop::option::of(1u32..=2),
            subtitle in prop::option::of(prop_oneof![Just(SubtitleFormat::Srt), Just(SubtitleFormat::Pgs)]),
        ) -> NegotiationInput {
            let height = if width >= 3840 { 2160 } else { 1080 };
            let subtitles = subtitle
                .map(|format| {
                    vec![EmbeddedSubtitleTrack {
                        index: 0,
                        language: None,
                        format,
                        forced: false,
                        default: false,
                    }]
                })
                .unwrap_or_default();
            let requested_subtitle = subtitle.map(|_| SubtitleSelection {
                track: SubtitleTrackRef::Embedded(0),
                offset_ms: None,
            });
            NegotiationInput {
                container,
                video: vec![VideoTrack {
                    index: 0,
                    codec: vcodec.to_owned(),
                    width,
                    height,
                    bit_depth,
                    hdr: None,
                    frame_rate: 24.0,
                    bitrate,
                }],
                audio: vec![AudioTrack {
                    index: 1,
                    codec: acodec.to_owned(),
                    channels,
                    language: None,
                    bitrate: None,
                }],
                subtitles,
                requested_audio,
                requested_subtitle,
                max_bitrate: None,
            }
        }
    }

    proptest! {
        #[test]
        fn always_produces_a_delivery_mode(input in inputs()) {
            let out = negotiate(&input, &profile());
            prop_assert!(matches!(
                out.mode,
                DeliveryMode::Direct | DeliveryMode::Remux | DeliveryMode::Transcode
            ));
        }

        #[test]
        fn burned_subtitle_forces_transcode(input in inputs()) {
            let out = negotiate(&input, &profile());
            if out.selected.subtitle_delivery == Some(SubtitleDelivery::Burned) {
                prop_assert_eq!(out.mode, DeliveryMode::Transcode);
            }
        }

        #[test]
        fn direct_implies_container_supported_and_not_burned(input in inputs()) {
            let profile = profile();
            let out = negotiate(&input, &profile);
            if out.mode == DeliveryMode::Direct {
                prop_assert!(profile.containers.contains(&input.container));
                prop_assert_ne!(
                    out.selected.subtitle_delivery,
                    Some(SubtitleDelivery::Burned)
                );
            }
        }

        #[test]
        fn remux_implies_unsupported_container(input in inputs()) {
            let profile = profile();
            let out = negotiate(&input, &profile);
            if out.mode == DeliveryMode::Remux {
                prop_assert!(!profile.containers.contains(&input.container));
            }
        }

        #[test]
        fn audio_selection_is_consistent(input in inputs()) {
            let out = negotiate(&input, &profile());
            prop_assert_eq!(
                out.selected.audio_track,
                selected_audio(&input).map(|a| a.index)
            );
        }
    }
}
