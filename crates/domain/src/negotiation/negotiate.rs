use crate::media::{AudioTrack, EmbeddedSubtitleTrack, VideoTrack};
use crate::negotiation::{NegotiationInput, NegotiationOutcome, NegotiationReason};
use crate::playback::SubtitleTrackRef;
use crate::profile::CapabilityProfile;
use crate::session::{
    AudioDelivery, DeliveryMode, DeliveryPreference, SelectedTracks, SubtitleDelivery,
    SubtitleSelection, TrackHandling, segments_carry,
};

pub const CONVERTED_AUDIO_CODEC: &str = "aac";

pub fn negotiate(input: &NegotiationInput, profile: &CapabilityProfile) -> NegotiationOutcome {
    let video = input.video.first();
    let audio = selected_audio(input);

    let max_height = effective_max_height(profile.max_height, input.target_height);
    let mut reasons =
        video.map(|v| video_reasons(v, profile, max_height, input.max_bitrate)).unwrap_or_default();
    let reencode_video = !reasons.is_empty();
    let sound = audio.map(|a| audio_reasons(a, profile)).unwrap_or_default();
    let downmix = input.downmix_stereo && audio.is_some_and(|a| a.channels > 2);
    let convert_audio = !sound.is_empty() || downmix;
    reasons.extend(sound);
    if downmix {
        reasons.push(NegotiationReason::Downmix);
    }
    let container_ok = profile.containers.contains(&input.container);
    if !container_ok {
        reasons.push(NegotiationReason::Container);
    }
    let audio_mapped = !input.tracks.audio && audio.is_some_and(|a| !is_first_audio(input, a));
    if audio_mapped {
        reasons.push(NegotiationReason::AudioTrack);
    }
    let base_repackage = !container_ok || audio_mapped;
    let plays_directly =
        mode_for(input.delivery, reencode_video, false, base_repackage, convert_audio)
            == DeliveryMode::Direct;
    let subtitle_delivery = input.requested_subtitle.as_ref().map(|sel| {
        subtitle_delivery_for(sel, &input.subtitles, input.force_burn, input.tracks, plays_directly)
    });

    let burned = subtitle_delivery == Some(SubtitleDelivery::Burned);
    if burned {
        reasons.push(NegotiationReason::SubtitleBurn);
    }
    let subtitle_hls = subtitle_delivery == Some(SubtitleDelivery::HlsVtt);
    if subtitle_hls {
        reasons.push(NegotiationReason::SubtitleSoft);
    }
    let repackage = base_repackage || subtitle_hls;
    let video_not_copyable = video.is_some_and(|v| !segments_carry(Some(&v.codec), None));

    let wanted = mode_for(input.delivery, reencode_video, burned, repackage, convert_audio);
    if wanted == DeliveryMode::Remux && video_not_copyable {
        reasons.push(NegotiationReason::VideoNotCopyable);
    }
    let mode = copyable(wanted, video_not_copyable);
    let auto = mode_for(DeliveryPreference::Auto, reencode_video, burned, repackage, convert_audio);
    if input.delivery == DeliveryPreference::AlwaysConvert
        && copyable(auto, video_not_copyable) != DeliveryMode::Transcode
    {
        reasons.push(NegotiationReason::Forced);
    }
    let mode = if mode == DeliveryMode::Remux && !input.prepared {
        reasons.push(NegotiationReason::Preparing);
        DeliveryMode::Transcode
    } else {
        mode
    };
    let converting = convert_audio && input.delivery != DeliveryPreference::NeverConvert;
    let audio_not_copyable = mode == DeliveryMode::Remux
        && !converting
        && audio.is_some_and(|a| !segments_carry(video.map(|v| v.codec.as_str()), Some(&a.codec)));
    if audio_not_copyable {
        reasons.push(NegotiationReason::AudioNotCopyable);
    }

    NegotiationOutcome {
        mode,
        selected: SelectedTracks {
            audio_track: audio.map(|a| a.index),
            audio_delivery: audio.map(|_| match mode {
                DeliveryMode::Direct => AudioDelivery::Embedded,
                _ => AudioDelivery::Stream,
            }),
            subtitle_track: input.requested_subtitle.as_ref().map(|s| s.track.clone()),
            subtitle_delivery,
            subtitle_url: None,
        },
        reasons,
        convert_audio: mode == DeliveryMode::Remux && (converting || audio_not_copyable),
        audio_channels: audio.and_then(|a| converted_channels(a, downmix, profile)),
    }
}

fn copyable(mode: DeliveryMode, video_not_copyable: bool) -> DeliveryMode {
    match mode {
        DeliveryMode::Remux if video_not_copyable => DeliveryMode::Transcode,
        mode => mode,
    }
}

pub fn effective_max_height(profile_max: u32, target: Option<u32>) -> u32 {
    match target {
        Some(target) => profile_max.min(target),
        None => profile_max,
    }
}

fn selected_audio(input: &NegotiationInput) -> Option<&AudioTrack> {
    match input.requested_audio {
        Some(idx) => input.audio.iter().find(|a| a.index == idx),
        None => input.audio.first(),
    }
}

fn is_first_audio(input: &NegotiationInput, audio: &AudioTrack) -> bool {
    input.audio.first().is_some_and(|first| first.index == audio.index)
}

fn mode_for(
    delivery: DeliveryPreference,
    reencode_video: bool,
    burned: bool,
    repackage: bool,
    convert_audio: bool,
) -> DeliveryMode {
    let wanted = if reencode_video || burned {
        DeliveryMode::Transcode
    } else if repackage || convert_audio {
        DeliveryMode::Remux
    } else {
        DeliveryMode::Direct
    };
    match delivery {
        DeliveryPreference::Auto => wanted,
        DeliveryPreference::AlwaysConvert => DeliveryMode::Transcode,
        DeliveryPreference::NeverConvert if !burned => {
            if repackage {
                DeliveryMode::Remux
            } else {
                DeliveryMode::Direct
            }
        }
        DeliveryPreference::NeverConvert => wanted,
    }
}

fn video_reasons(
    v: &VideoTrack,
    profile: &CapabilityProfile,
    max_height: u32,
    user_cap: Option<u64>,
) -> Vec<NegotiationReason> {
    let Some(cap) = profile.video.iter().find(|c| c.codec == v.codec) else {
        return vec![NegotiationReason::VideoCodec];
    };
    let mut reasons = Vec::new();
    if !cap.smooth {
        reasons.push(NegotiationReason::VideoSoftwareOnly);
    }
    if cap.max_bit_depth < v.bit_depth {
        reasons.push(NegotiationReason::VideoBitDepth);
    }
    if v.width > profile.max_width || v.height > max_height {
        reasons.push(NegotiationReason::VideoResolution);
    }
    if profile.max_frame_rate.is_some_and(|max| v.frame_rate.round() as u32 > max) {
        reasons.push(NegotiationReason::VideoFrameRate);
    }
    if v.hdr.is_some_and(|h| !profile.hdr.contains(&h)) {
        reasons.push(NegotiationReason::VideoHdr);
    }
    if v.bitrate.is_some_and(|b| b > effective_cap(profile.max_bitrate, user_cap)) {
        reasons.push(NegotiationReason::VideoBitrate);
    }
    reasons
}

fn audio_reasons(a: &AudioTrack, profile: &CapabilityProfile) -> Vec<NegotiationReason> {
    let channels =
        profile.audio.iter().filter(|c| c.codec == a.codec).map(|c| c.max_channels).max();
    match channels {
        None => vec![NegotiationReason::AudioCodec],
        Some(max) if a.channels > max => vec![NegotiationReason::AudioChannels],
        Some(_) => Vec::new(),
    }
}

fn converted_channels(a: &AudioTrack, downmix: bool, profile: &CapabilityProfile) -> Option<u8> {
    if downmix {
        return Some(2);
    }
    let limit = profile
        .audio
        .iter()
        .filter(|c| c.codec == CONVERTED_AUDIO_CODEC)
        .map(|c| c.max_channels)
        .max()?;
    (a.channels > limit).then_some(limit)
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
    force_burn: bool,
    tracks: TrackHandling,
    plays_directly: bool,
) -> SubtitleDelivery {
    if force_burn {
        return SubtitleDelivery::Burned;
    }
    let image = match &sel.track {
        SubtitleTrackRef::Embedded(idx) => {
            subtitles.iter().find(|s| s.index == *idx).is_some_and(|s| s.format.is_image())
        }
        SubtitleTrackRef::File(_) => false,
    };
    let embedded = matches!(sel.track, SubtitleTrackRef::Embedded(_));
    match (plays_directly, image) {
        (true, true) if tracks.image => SubtitleDelivery::Embedded,
        (_, true) => SubtitleDelivery::Burned,
        (true, false) if embedded && tracks.embedded => SubtitleDelivery::Embedded,
        (true, false) if tracks.external => SubtitleDelivery::External,
        (false, false) if tracks.external && !tracks.rendition => SubtitleDelivery::External,
        _ => SubtitleDelivery::HlsVtt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{HdrFormat, SubtitleFileId, SubtitleFormat};
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
            title: None,
        }
    }

    fn subtitle(index: u32, format: SubtitleFormat) -> EmbeddedSubtitleTrack {
        EmbeddedSubtitleTrack {
            index,
            language: None,
            format,
            forced: false,
            default: false,
            title: None,
        }
    }

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4, Container::Hls],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
                smooth: true,
            }],
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 2 }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
            max_frame_rate: None,
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
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
            delivery: DeliveryPreference::Auto,
            prepared: true,
            tracks: TrackHandling::default(),
        }
    }

    fn embedded(idx: u32) -> SubtitleSelection {
        SubtitleSelection { track: SubtitleTrackRef::Embedded(idx), offset_ms: None }
    }

    // The per-session escape hatch: the viewer already knows this version
    // misbehaves, so their choice beats what the capabilities imply.
    #[test]
    fn always_convert_transcodes_what_would_have_played_directly() {
        let i = NegotiationInput { delivery: DeliveryPreference::AlwaysConvert, ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(out.reasons.contains(&NegotiationReason::Forced));
    }

    #[test]
    fn always_convert_says_nothing_extra_when_it_changed_nothing() {
        let i = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            delivery: DeliveryPreference::AlwaysConvert,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(!out.reasons.contains(&NegotiationReason::Forced));
        assert!(out.reasons.contains(&NegotiationReason::VideoCodec));
    }

    #[test]
    fn never_convert_drops_a_transcode_to_direct_and_keeps_the_reasons() {
        let i = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert!(
            out.reasons.contains(&NegotiationReason::VideoCodec),
            "the reasons still record what would have happened"
        );
    }

    #[test]
    fn never_convert_stops_at_remux_when_the_container_cannot_be_played() {
        let i = NegotiationInput {
            container: Container::Mkv,
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Remux);
    }

    // Burning needs a re-encode, so honouring the preference would drop the
    // subtitles without saying so.
    #[test]
    fn never_convert_gives_way_to_burned_in_subtitles() {
        let i = NegotiationInput {
            subtitles: vec![subtitle(3, SubtitleFormat::Pgs)],
            requested_subtitle: Some(embedded(3)),
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
    }

    #[test]
    fn a_remux_of_a_version_not_prepared_yet_is_transcoded_instead() {
        let i = NegotiationInput { container: Container::Mkv, prepared: false, ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.reasons, vec![NegotiationReason::Container, NegotiationReason::Preparing]);
    }

    #[test]
    fn never_convert_still_transcodes_a_remux_that_has_no_keyframes() {
        let i = NegotiationInput {
            container: Container::Mkv,
            delivery: DeliveryPreference::NeverConvert,
            prepared: false,
            ..input()
        };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn a_version_not_prepared_yet_still_plays_directly_or_transcodes_as_it_would() {
        let direct = NegotiationInput { prepared: false, ..input() };
        let out = negotiate(&direct, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert!(out.reasons.is_empty());

        let transcode = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            prepared: false,
            ..input()
        };
        let out = negotiate(&transcode, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.reasons, vec![NegotiationReason::VideoCodec]);
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
        let i = NegotiationInput { container: Container::Mkv, ..input() };
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
        let p = CapabilityProfile { hdr: vec![HdrFormat::Hdr10], ..profile() };
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
        let i = NegotiationInput { max_bitrate: Some(3_000_000), ..input() };
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
    fn an_audio_codec_the_player_lacks_converts_only_the_sound() {
        let i = NegotiationInput { audio: vec![audio_track("eac3", 2, 1)], ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::AudioCodec]);
        assert_eq!(out.audio_channels, None);
    }

    #[test]
    fn too_many_channels_convert_the_sound_down_to_what_the_player_takes() {
        let i = NegotiationInput { audio: vec![audio_track("aac", 6, 1)], ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::AudioChannels]);
        assert_eq!(out.audio_channels, Some(2));
    }

    #[test]
    fn a_converted_sound_keeps_its_channels_when_the_player_takes_them_all() {
        let p = CapabilityProfile {
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 8 }],
            ..profile()
        };
        let i = NegotiationInput { audio: vec![audio_track("truehd", 8, 1)], ..input() };
        let out = negotiate(&i, &p);
        assert!(out.convert_audio);
        assert_eq!(out.audio_channels, None);

        let no_aac = CapabilityProfile {
            audio: vec![AudioCodecCap { codec: "opus".to_owned(), max_channels: 2 }],
            ..profile()
        };
        assert_eq!(negotiate(&i, &no_aac).audio_channels, None);
    }

    fn taking(video: &[&str], audio: &[&str]) -> CapabilityProfile {
        let base = profile();
        CapabilityProfile {
            video: base
                .video
                .iter()
                .cloned()
                .chain(video.iter().map(|codec| VideoCodecCap {
                    codec: (*codec).to_owned(),
                    max_level: None,
                    max_bit_depth: 8,
                    smooth: true,
                }))
                .collect(),
            audio: base
                .audio
                .iter()
                .cloned()
                .chain(
                    audio
                        .iter()
                        .map(|codec| AudioCodecCap { codec: (*codec).to_owned(), max_channels: 8 }),
                )
                .collect(),
            ..base
        }
    }

    #[test]
    fn a_video_no_segment_carries_is_transcoded_rather_than_remuxed_even_under_never() {
        let never = NegotiationInput {
            container: Container::Avi,
            video: vec![video_track("msmpeg4v3", 640, 480, 8, None, None)],
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        let out = negotiate(&never, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(out.reasons.contains(&NegotiationReason::VideoNotCopyable));
        assert!(!out.convert_audio);

        let decodes = taking(&["msmpeg4v3"], &[]);
        let auto = NegotiationInput { delivery: DeliveryPreference::Auto, ..never.clone() };
        let out = negotiate(&auto, &decodes);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(
            out.reasons,
            vec![NegotiationReason::Container, NegotiationReason::VideoNotCopyable]
        );

        let always = NegotiationInput { delivery: DeliveryPreference::AlwaysConvert, ..never };
        let out = negotiate(&always, &decodes);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(!out.reasons.contains(&NegotiationReason::Forced));
    }

    #[test]
    fn mpeg4_part_2_is_transcoded_where_it_would_have_been_remuxed() {
        let decodes = taking(&["mpeg4"], &["ac3"]);
        let xvid = NegotiationInput {
            video: vec![video_track("mpeg4", 720, 400, 8, None, None)],
            audio: vec![audio_track("mp3", 2, 1)],
            ..input()
        };
        let out = negotiate(&xvid, &decodes);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(
            out.reasons,
            vec![NegotiationReason::AudioCodec, NegotiationReason::VideoNotCopyable]
        );
        assert!(!out.convert_audio);

        let downmix = NegotiationInput {
            audio: vec![audio_track("ac3", 6, 1)],
            downmix_stereo: true,
            ..xvid.clone()
        };
        let out = negotiate(&downmix, &decodes);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(
            out.reasons,
            vec![NegotiationReason::Downmix, NegotiationReason::VideoNotCopyable]
        );

        let never = NegotiationInput {
            container: Container::Avi,
            delivery: DeliveryPreference::NeverConvert,
            ..xvid.clone()
        };
        let out = negotiate(&never, &decodes);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(
            out.reasons,
            vec![
                NegotiationReason::AudioCodec,
                NegotiationReason::Container,
                NegotiationReason::VideoNotCopyable
            ]
        );

        let plays = NegotiationInput { audio: vec![audio_track("ac3", 2, 1)], ..xvid };
        let out = negotiate(&plays, &decodes);
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert!(out.reasons.is_empty());
    }

    #[test]
    fn a_sound_no_segment_carries_beside_its_video_is_converted_even_under_never() {
        let flac = NegotiationInput {
            container: Container::Mkv,
            audio: vec![audio_track("flac", 2, 1)],
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        let out = negotiate(&flac, &profile());
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio);
        assert!(out.reasons.contains(&NegotiationReason::AudioNotCopyable));

        let takes = taking(&["vp9"], &["flac", "truehd"]);
        let decoded = NegotiationInput { delivery: DeliveryPreference::Auto, ..flac };
        let out = negotiate(&decoded, &takes);
        assert!(out.convert_audio);
        assert_eq!(
            out.reasons,
            vec![NegotiationReason::Container, NegotiationReason::AudioNotCopyable]
        );

        let truehd = NegotiationInput {
            container: Container::Mkv,
            audio: vec![audio_track("truehd", 8, 1)],
            ..input()
        };
        let out = negotiate(&truehd, &takes);
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(!out.convert_audio, "mpegts carries truehd beside h264");
        assert_eq!(out.reasons, vec![NegotiationReason::Container]);

        let beside_vp9 = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            ..truehd
        };
        let out = negotiate(&beside_vp9, &takes);
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio, "no one container carries vp9 with truehd");
        assert_eq!(
            out.reasons,
            vec![NegotiationReason::Container, NegotiationReason::AudioNotCopyable]
        );
    }

    #[test]
    fn a_video_change_transcodes_both_and_still_limits_the_channels() {
        let i = NegotiationInput {
            video: vec![video_track("vp9", 1920, 1080, 8, None, Some(5_000_000))],
            audio: vec![audio_track("eac3", 6, 1)],
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(!out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::VideoCodec, NegotiationReason::AudioCodec]);
        assert_eq!(out.audio_channels, Some(2));
    }

    #[test]
    fn never_convert_leaves_the_sound_as_it_is() {
        let i = NegotiationInput {
            audio: vec![audio_track("eac3", 2, 1)],
            delivery: DeliveryPreference::NeverConvert,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert!(!out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::AudioCodec]);

        let repackaged = NegotiationInput { container: Container::Mkv, ..i };
        let out = negotiate(&repackaged, &profile());
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(!out.convert_audio);
    }

    #[test]
    fn always_convert_transcodes_a_sound_only_change() {
        let i = NegotiationInput {
            audio: vec![audio_track("eac3", 2, 1)],
            delivery: DeliveryPreference::AlwaysConvert,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(!out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::AudioCodec, NegotiationReason::Forced]);
    }

    #[test]
    fn a_sound_only_change_on_a_version_not_prepared_yet_is_transcoded() {
        let i =
            NegotiationInput { audio: vec![audio_track("eac3", 2, 1)], prepared: false, ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert!(!out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::AudioCodec, NegotiationReason::Preparing]);
    }

    #[test]
    fn text_subtitle_uses_hls_vtt_without_transcode() {
        let i = NegotiationInput {
            subtitles: vec![subtitle(2, SubtitleFormat::Srt)],
            requested_subtitle: Some(embedded(2)),
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert_eq!(out.selected.subtitle_track, Some(SubtitleTrackRef::Embedded(2)));
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));
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
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
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
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));
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
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio);
    }

    #[test]
    fn requested_audio_index_absent_selects_none() {
        let i = NegotiationInput { requested_audio: Some(99), ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.selected.audio_track, None);
        assert_eq!(out.mode, DeliveryMode::Direct);
    }

    #[test]
    fn no_video_is_supported() {
        let i = NegotiationInput { video: vec![], ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.audio_track, Some(1));
    }

    #[test]
    fn target_rung_below_source_forces_transcode() {
        let i = NegotiationInput { target_height: Some(720), ..input() };
        assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Transcode);
    }

    #[test]
    fn original_or_higher_rung_stays_direct() {
        for target in [None, Some(1080), Some(2160)] {
            let i = NegotiationInput { target_height: target, ..input() };
            assert_eq!(negotiate(&i, &profile()).mode, DeliveryMode::Direct);
        }
    }

    #[test]
    fn effective_max_height_takes_smaller_of_profile_and_target() {
        assert_eq!(effective_max_height(1080, None), 1080);
        assert_eq!(effective_max_height(1080, Some(720)), 720);
        assert_eq!(effective_max_height(720, Some(1080)), 720);
    }

    #[test]
    fn force_burn_burns_text_subtitle_and_transcodes() {
        let i = NegotiationInput {
            subtitles: vec![subtitle(2, SubtitleFormat::Srt)],
            requested_subtitle: Some(embedded(2)),
            force_burn: true,
            ..input()
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
    }

    #[test]
    fn downmix_converts_only_multichannel_sound_to_stereo() {
        let surround_profile = CapabilityProfile {
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 8 }],
            ..profile()
        };
        let surround = NegotiationInput {
            audio: vec![audio_track("aac", 6, 1)],
            downmix_stereo: true,
            ..input()
        };
        let out = negotiate(&surround, &surround_profile);
        assert_eq!(out.mode, DeliveryMode::Remux);
        assert!(out.convert_audio);
        assert_eq!(out.reasons, vec![NegotiationReason::Downmix]);
        assert_eq!(out.audio_channels, Some(2));

        let already_stereo = NegotiationInput { downmix_stereo: true, ..input() };
        let out = negotiate(&already_stereo, &surround_profile);
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert!(!out.convert_audio);
        assert_eq!(out.audio_channels, None);
    }

    #[test]
    fn a_codec_the_client_can_only_software_decode_is_transcoded() {
        // A phone that software decodes 4K HEVC at single digit fps still
        // reports it as supported, so the client says so and the server, not
        // the client, decides that means transcode.
        let p = CapabilityProfile {
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
                smooth: false,
            }],
            ..profile()
        };
        let out = negotiate(&input(), &p);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.reasons, vec![NegotiationReason::VideoSoftwareOnly]);
    }

    #[test]
    fn frame_rate_over_the_ceiling_transcodes_and_broadcast_rates_do_not() {
        // A hardware decoder that reaches 4K30 but not 4K60 is the case a
        // resolution ceiling alone cannot express. 59.94 is 60, not 59.
        let p = CapabilityProfile { max_frame_rate: Some(30), ..profile() };
        let sixty = NegotiationInput {
            video: vec![VideoTrack {
                frame_rate: 59.94,
                ..video_track("h264", 1920, 1080, 8, None, Some(5_000_000))
            }],
            ..input()
        };
        let out = negotiate(&sixty, &p);
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.reasons, vec![NegotiationReason::VideoFrameRate]);

        let thirty = NegotiationInput {
            video: vec![VideoTrack {
                frame_rate: 29.97,
                ..video_track("h264", 1920, 1080, 8, None, Some(5_000_000))
            }],
            ..input()
        };
        assert_eq!(negotiate(&thirty, &p).mode, DeliveryMode::Direct);
    }

    #[test]
    fn an_unconstrained_frame_rate_never_transcodes_on_rate() {
        let fast = NegotiationInput {
            video: vec![VideoTrack {
                frame_rate: 120.0,
                ..video_track("h264", 1920, 1080, 8, None, Some(5_000_000))
            }],
            ..input()
        };
        assert_eq!(negotiate(&fast, &profile()).mode, DeliveryMode::Direct);
    }

    #[test]
    fn every_mismatch_names_itself() {
        // Nothing recorded why a session became transcode, which is the
        // question every 4K support report turned on.
        let cases: Vec<(NegotiationInput, NegotiationReason)> = vec![
            (
                NegotiationInput {
                    video: vec![video_track("vp9", 1920, 1080, 8, None, None)],
                    ..input()
                },
                NegotiationReason::VideoCodec,
            ),
            (
                NegotiationInput {
                    video: vec![video_track("h264", 1920, 1080, 10, None, None)],
                    ..input()
                },
                NegotiationReason::VideoBitDepth,
            ),
            (
                NegotiationInput {
                    video: vec![video_track("h264", 3840, 2160, 8, None, None)],
                    ..input()
                },
                NegotiationReason::VideoResolution,
            ),
            (
                NegotiationInput {
                    video: vec![video_track("h264", 1920, 1080, 8, Some(HdrFormat::Hdr10), None)],
                    ..input()
                },
                NegotiationReason::VideoHdr,
            ),
            (
                NegotiationInput {
                    video: vec![video_track("h264", 1920, 1080, 8, None, Some(20_000_000))],
                    ..input()
                },
                NegotiationReason::VideoBitrate,
            ),
            (
                NegotiationInput { audio: vec![audio_track("eac3", 2, 1)], ..input() },
                NegotiationReason::AudioCodec,
            ),
            (
                NegotiationInput { audio: vec![audio_track("aac", 6, 1)], ..input() },
                NegotiationReason::AudioChannels,
            ),
            (
                NegotiationInput { container: Container::Mkv, ..input() },
                NegotiationReason::Container,
            ),
            (
                NegotiationInput {
                    subtitles: vec![subtitle(2, SubtitleFormat::Srt)],
                    requested_subtitle: Some(embedded(2)),
                    ..input()
                },
                NegotiationReason::SubtitleSoft,
            ),
            (
                NegotiationInput {
                    subtitles: vec![subtitle(3, SubtitleFormat::Pgs)],
                    requested_subtitle: Some(embedded(3)),
                    ..input()
                },
                NegotiationReason::SubtitleBurn,
            ),
            (
                NegotiationInput {
                    audio: vec![audio_track("aac", 6, 1)],
                    downmix_stereo: true,
                    ..input()
                },
                NegotiationReason::Downmix,
            ),
            (
                NegotiationInput {
                    audio: vec![audio_track("aac", 2, 1), audio_track("aac", 2, 2)],
                    requested_audio: Some(2),
                    ..input()
                },
                NegotiationReason::AudioTrack,
            ),
        ];

        let base = profile();
        let surround = CapabilityProfile {
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 8 }],
            ..profile()
        };
        for (case, expected) in cases {
            let p = if case.downmix_stereo { &surround } else { &base };
            let out = negotiate(&case, p);
            assert!(out.reasons.contains(&expected), "expected {expected} in {:?}", out.reasons);
        }
    }

    #[test]
    fn a_direct_session_has_nothing_to_report() {
        assert!(negotiate(&input(), &profile()).reasons.is_empty());
    }

    fn player() -> TrackHandling {
        TrackHandling { embedded: true, image: true, external: true, rendition: false, audio: true }
    }

    fn browser() -> TrackHandling {
        TrackHandling { external: true, rendition: true, ..TrackHandling::default() }
    }

    fn file_track() -> SubtitleSelection {
        SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("ext1".to_owned())),
            offset_ms: None,
        }
    }

    fn with_subtitle(tracks: TrackHandling, format: SubtitleFormat) -> NegotiationInput {
        NegotiationInput {
            subtitles: vec![subtitle(2, format)],
            requested_subtitle: Some(embedded(2)),
            tracks,
            ..input()
        }
    }

    fn streamed(i: NegotiationInput) -> NegotiationInput {
        NegotiationInput { video: vec![video_track("vp9", 1920, 1080, 8, None, None)], ..i }
    }

    #[test]
    fn a_player_that_selects_tracks_keeps_an_embedded_text_subtitle_in_direct_play() {
        let out = negotiate(&with_subtitle(player(), SubtitleFormat::Srt), &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Embedded));
        assert!(out.reasons.is_empty());
    }

    #[test]
    fn a_browser_gets_an_embedded_text_subtitle_as_a_file_in_direct_play() {
        let out = negotiate(&with_subtitle(browser(), SubtitleFormat::Srt), &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::External));
    }

    #[test]
    fn a_subtitle_file_stays_a_file_in_direct_play_for_a_client_that_loads_one() {
        for tracks in [player(), browser()] {
            let i = NegotiationInput { requested_subtitle: Some(file_track()), tracks, ..input() };
            let out = negotiate(&i, &profile());
            assert_eq!(out.mode, DeliveryMode::Direct);
            assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::External));
        }
    }

    #[test]
    fn an_image_subtitle_stays_embedded_only_for_a_client_that_draws_images() {
        for format in [SubtitleFormat::Pgs, SubtitleFormat::VobSub, SubtitleFormat::Dvb] {
            let drawn = negotiate(&with_subtitle(player(), format), &profile());
            assert_eq!(drawn.mode, DeliveryMode::Direct, "{format:?}");
            assert_eq!(drawn.selected.subtitle_delivery, Some(SubtitleDelivery::Embedded));
            let burned = negotiate(&with_subtitle(browser(), format), &profile());
            assert_eq!(burned.mode, DeliveryMode::Transcode, "{format:?}");
            assert_eq!(burned.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
        }
    }

    #[test]
    fn in_a_stream_a_player_gets_a_file_and_a_browser_the_rendition() {
        let player_out =
            negotiate(&streamed(with_subtitle(player(), SubtitleFormat::Srt)), &profile());
        assert_eq!(player_out.mode, DeliveryMode::Transcode);
        assert_eq!(player_out.selected.subtitle_delivery, Some(SubtitleDelivery::External));
        assert!(!player_out.reasons.contains(&NegotiationReason::SubtitleSoft));
        let browser_out =
            negotiate(&streamed(with_subtitle(browser(), SubtitleFormat::Srt)), &profile());
        assert_eq!(browser_out.selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));
    }

    #[test]
    fn in_a_stream_an_image_subtitle_is_burned_even_for_a_client_that_draws_images() {
        let out = negotiate(&streamed(with_subtitle(player(), SubtitleFormat::Pgs)), &profile());
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
    }

    #[test]
    fn forcing_a_conversion_streams_and_a_player_then_gets_a_file() {
        let i = NegotiationInput {
            delivery: DeliveryPreference::AlwaysConvert,
            ..with_subtitle(player(), SubtitleFormat::Srt)
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::External));
        assert!(out.reasons.contains(&NegotiationReason::Forced));
    }

    #[test]
    fn never_convert_keeps_direct_play_and_an_embedded_subtitle() {
        let i = NegotiationInput {
            delivery: DeliveryPreference::NeverConvert,
            ..streamed(with_subtitle(player(), SubtitleFormat::Srt))
        };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Embedded));
    }

    #[test]
    fn burning_on_request_wins_over_what_the_client_handles() {
        let i =
            NegotiationInput { force_burn: true, ..with_subtitle(player(), SubtitleFormat::Srt) };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Transcode);
        assert_eq!(out.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
    }

    #[test]
    fn a_later_audio_track_plays_directly_only_for_a_client_that_switches_tracks() {
        let two = |tracks| NegotiationInput {
            audio: vec![audio_track("aac", 2, 1), audio_track("aac", 2, 2)],
            requested_audio: Some(2),
            tracks,
            ..input()
        };
        let switched = negotiate(&two(player()), &profile());
        assert_eq!(switched.mode, DeliveryMode::Direct);
        assert_eq!(switched.selected.audio_track, Some(2));
        assert_eq!(switched.selected.audio_delivery, Some(AudioDelivery::Embedded));
        assert!(switched.reasons.is_empty());

        let mapped = negotiate(&two(browser()), &profile());
        assert_eq!(mapped.mode, DeliveryMode::Remux);
        assert!(!mapped.convert_audio);
        assert_eq!(mapped.reasons, vec![NegotiationReason::AudioTrack]);
        assert_eq!(mapped.selected.audio_delivery, Some(AudioDelivery::Stream));
    }

    #[test]
    fn the_first_audio_track_plays_directly_for_every_client() {
        let i = NegotiationInput { requested_audio: Some(1), tracks: browser(), ..input() };
        let out = negotiate(&i, &profile());
        assert_eq!(out.mode, DeliveryMode::Direct);
        assert_eq!(out.selected.audio_delivery, Some(AudioDelivery::Embedded));
    }

    #[test]
    fn without_sound_there_is_no_audio_delivery() {
        let out = negotiate(&NegotiationInput { audio: vec![], ..input() }, &profile());
        assert_eq!(out.selected.audio_delivery, None);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::media::SubtitleFormat;
    use crate::profile::{AudioCodecCap, Container, VideoCodecCap};
    use proptest::prelude::*;

    fn profile() -> CapabilityProfile {
        let video = |codec: &str| VideoCodecCap {
            codec: codec.to_owned(),
            max_level: None,
            max_bit_depth: 8,
            smooth: true,
        };
        let audio =
            |codec: &str, max_channels| AudioCodecCap { codec: codec.to_owned(), max_channels };
        CapabilityProfile {
            containers: vec![Container::Mp4, Container::Hls],
            video: vec![video("h264"), video("mpeg4"), video("msmpeg4v3")],
            audio: vec![audio("aac", 2), audio("flac", 8), audio("truehd", 8)],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
            max_frame_rate: Some(60),
        }
    }

    prop_compose! {
        fn inputs()(
            container in prop_oneof![Just(Container::Mp4), Just(Container::Hls), Just(Container::Mkv)],
            vcodec in prop_oneof![Just("h264"), Just("vp9"), Just("mpeg4"), Just("msmpeg4v3")],
            width in prop_oneof![Just(1280u32), Just(1920u32), Just(3840u32)],
            bit_depth in prop_oneof![Just(8u8), Just(10u8)],
            bitrate in prop::option::of(1_000_000u64..=30_000_000u64),
            acodec in prop_oneof![Just("aac"), Just("eac3"), Just("flac"), Just("truehd")],
            channels in prop_oneof![Just(2u8), Just(6u8)],
            requested_audio in prop::option::of(1u32..=2),
            subtitle in prop::option::of(prop_oneof![Just(SubtitleFormat::Srt), Just(SubtitleFormat::Pgs)]),
            target_height in prop::option::of(prop_oneof![Just(480u32), Just(720u32), Just(1080u32)]),
            force_burn in any::<bool>(),
            downmix_stereo in any::<bool>(),
            delivery in prop_oneof![
                Just(DeliveryPreference::Auto),
                Just(DeliveryPreference::NeverConvert),
                Just(DeliveryPreference::AlwaysConvert),
            ],
            prepared in any::<bool>(),
            second_audio in any::<bool>(),
            declared in prop::array::uniform5(any::<bool>()),
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
                        title: None,
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
                audio: [1, 2]
                    .into_iter()
                    .take(if second_audio { 2 } else { 1 })
                    .map(|index| AudioTrack {
                        index,
                        codec: acodec.to_owned(),
                        channels,
                        language: None,
                        bitrate: None,
                        title: None,
                    })
                    .collect(),
                subtitles,
                requested_audio,
                requested_subtitle,
                max_bitrate: None,
                target_height,
                force_burn,
                downmix_stereo,
                delivery,
                prepared,
                tracks: TrackHandling {
                    embedded: declared[0],
                    image: declared[1],
                    external: declared[2],
                    rendition: declared[3],
                    audio: declared[4],
                },
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
                prop_assert_ne!(
                    out.selected.subtitle_delivery,
                    Some(SubtitleDelivery::HlsVtt)
                );
            }
        }

        #[test]
        fn remux_implies_unsupported_container_soft_subtitle_or_sound_to_convert(input in inputs()) {
            let profile = profile();
            let out = negotiate(&input, &profile);
            if out.mode == DeliveryMode::Remux {
                prop_assert!(
                    !profile.containers.contains(&input.container)
                        || out.selected.subtitle_delivery == Some(SubtitleDelivery::HlsVtt)
                        || out.convert_audio
                        || out.reasons.contains(&NegotiationReason::AudioTrack)
                );
            }
        }

        #[test]
        fn a_client_is_only_answered_with_a_delivery_it_declared(input in inputs()) {
            let out = negotiate(&input, &profile());
            let image = input.subtitles.first().is_some_and(|s| s.format.is_image());
            let draws = if image { input.tracks.image } else { input.tracks.embedded };
            match out.selected.subtitle_delivery {
                Some(SubtitleDelivery::Embedded) => prop_assert!(draws),
                Some(SubtitleDelivery::External) => prop_assert!(input.tracks.external && !image),
                _ => {}
            }
        }

        #[test]
        fn a_subtitle_the_client_handles_in_the_file_leaves_direct_play_alone(input in inputs()) {
            let plain = negotiate(
                &NegotiationInput { requested_subtitle: None, subtitles: vec![], ..input.clone() },
                &profile(),
            );
            let out = negotiate(&input, &profile());
            if matches!(
                out.selected.subtitle_delivery,
                Some(SubtitleDelivery::Embedded | SubtitleDelivery::External)
            ) {
                prop_assert_eq!(out.mode, plain.mode);
            }
        }

        #[test]
        fn a_later_audio_track_plays_directly_only_for_a_client_that_switches_tracks(input in inputs()) {
            let out = negotiate(&input, &profile());
            let later = out.selected.audio_track.is_some_and(|index| index != 1);
            if out.mode == DeliveryMode::Direct && later {
                prop_assert!(input.tracks.audio);
            }
            prop_assert_eq!(
                out.selected.audio_delivery,
                out.selected.audio_track.map(|_| if out.mode == DeliveryMode::Direct {
                    AudioDelivery::Embedded
                } else {
                    AudioDelivery::Stream
                })
            );
        }

        #[test]
        fn a_sound_is_converted_alone_only_in_a_remux_for_a_sound_reason(input in inputs()) {
            let out = negotiate(&input, &profile());
            if out.convert_audio {
                let not_copyable = out.reasons.contains(&NegotiationReason::AudioNotCopyable);
                prop_assert_eq!(out.mode, DeliveryMode::Remux);
                prop_assert!(input.delivery != DeliveryPreference::NeverConvert || not_copyable);
                prop_assert!(out.reasons.iter().any(|reason| matches!(
                    reason,
                    NegotiationReason::AudioCodec
                        | NegotiationReason::AudioChannels
                        | NegotiationReason::Downmix
                        | NegotiationReason::AudioNotCopyable
                )));
                let copies_regardless = input.delivery == DeliveryPreference::NeverConvert;
                prop_assert!(
                    copies_regardless
                        || !out.reasons.iter().any(|reason| reason.as_str().starts_with("video_"))
                );
            }
        }

        #[test]
        fn a_remux_only_copies_what_its_segments_carry(input in inputs()) {
            let out = negotiate(&input, &profile());
            if out.mode == DeliveryMode::Remux {
                let video = input.video.first().map(|v| v.codec.as_str());
                let audio = out
                    .selected
                    .audio_track
                    .and_then(|index| input.audio.iter().find(|a| a.index == index))
                    .map(|a| if out.convert_audio { CONVERTED_AUDIO_CODEC } else { a.codec.as_str() });
                prop_assert!(segments_carry(video, audio), "[{:?}] with [{:?}]", video, audio);
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

        // Only holds when the viewer has not overridden the outcome: a
        // preference deliberately keeps the reasons while changing the mode.
        #[test]
        fn a_session_left_alone_is_direct_exactly_when_there_is_nothing_to_report(input in inputs()) {
            let input = NegotiationInput { delivery: DeliveryPreference::Auto, ..input };
            let out = negotiate(&input, &profile());
            prop_assert_eq!(out.mode == DeliveryMode::Direct, out.reasons.is_empty());
        }

        #[test]
        fn a_preference_gets_what_it_asked_for(input in inputs()) {
            let out = negotiate(&input, &profile());
            match input.delivery {
                DeliveryPreference::Auto => {}
                DeliveryPreference::AlwaysConvert => {
                    prop_assert_eq!(out.mode, DeliveryMode::Transcode);
                }
                DeliveryPreference::NeverConvert => prop_assert!(
                    out.mode != DeliveryMode::Transcode
                        || out.selected.subtitle_delivery == Some(SubtitleDelivery::Burned)
                        || out.reasons.contains(&NegotiationReason::Preparing)
                        || out.reasons.contains(&NegotiationReason::VideoNotCopyable)
                ),
            }
        }

        #[test]
        fn a_version_without_keyframes_transcodes_only_what_would_have_been_remuxed(input in inputs()) {
            let ready = negotiate(&NegotiationInput { prepared: true, ..input.clone() }, &profile());
            let out = negotiate(&input, &profile());
            let held_back = !input.prepared && ready.mode == DeliveryMode::Remux;
            prop_assert_eq!(out.reasons.contains(&NegotiationReason::Preparing), held_back);
            let expected = if held_back { DeliveryMode::Transcode } else { ready.mode };
            prop_assert_eq!(out.mode, expected);
        }
    }
}
