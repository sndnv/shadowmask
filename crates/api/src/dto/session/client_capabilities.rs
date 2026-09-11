use serde::Deserialize;

use domain::media::HdrFormat;
use domain::profile::{AudioCodecCap, ClientDecoding, Container, VideoCodecCap};
use domain::session::ClientCapabilities;

#[derive(Debug, Deserialize)]
pub struct ClientCapabilitiesDto {
    pub platform: String,
    pub profile_version: u32,
    pub max_bitrate: Option<u64>,
    pub decoding: Option<ClientDecodingDto>,
}

#[derive(Debug, Deserialize)]
pub struct ClientDecodingDto {
    pub containers: Option<Vec<String>>,
    pub video: Option<Vec<VideoCodecCapDto>>,
    pub audio: Option<Vec<AudioCodecCapDto>>,
    pub hdr: Option<Vec<String>>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate: Option<u64>,
    pub max_frame_rate: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct VideoCodecCapDto {
    pub codec: String,
    pub max_level: Option<String>,
    pub max_bit_depth: u8,
    #[serde(default = "smooth_unless_stated")]
    pub smooth: bool,
}

#[derive(Debug, Deserialize)]
pub struct AudioCodecCapDto {
    pub codec: String,
    pub max_channels: u8,
}

fn smooth_unless_stated() -> bool {
    true
}

impl From<ClientCapabilitiesDto> for ClientCapabilities {
    fn from(c: ClientCapabilitiesDto) -> Self {
        ClientCapabilities {
            platform: c.platform,
            profile_version: c.profile_version,
            max_bitrate: c.max_bitrate,
            decoding: c.decoding.map(ClientDecoding::from),
        }
    }
}

impl From<ClientDecodingDto> for ClientDecoding {
    fn from(d: ClientDecodingDto) -> Self {
        ClientDecoding {
            containers: d
                .containers
                .map(|list| list.iter().filter_map(|c| Container::parse(c)).collect()),
            video: d.video.map(|list| {
                list.into_iter()
                    .map(|v| VideoCodecCap {
                        codec: v.codec,
                        max_level: v.max_level,
                        max_bit_depth: v.max_bit_depth,
                        smooth: v.smooth,
                    })
                    .collect()
            }),
            audio: d.audio.map(|list| {
                list.into_iter()
                    .map(|a| AudioCodecCap { codec: a.codec, max_channels: a.max_channels })
                    .collect()
            }),
            hdr: d.hdr.map(|list| list.iter().filter_map(|h| HdrFormat::parse(h)).collect()),
            max_width: d.max_width,
            max_height: d.max_height,
            max_bitrate: d.max_bitrate,
            max_frame_rate: d.max_frame_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> ClientCapabilities {
        serde_json::from_str::<ClientCapabilitiesDto>(json).unwrap().into()
    }

    #[test]
    fn a_client_that_only_names_a_profile_still_parses() {
        // Every client shipped so far sends these two fields and nothing else.
        let caps = parse(r#"{"platform":"chrome","profile_version":1}"#);
        assert_eq!(caps.platform, "chrome");
        assert!(caps.decoding.is_none());
        assert_eq!(caps.max_bitrate, None);
    }

    #[test]
    fn a_measured_report_carries_through() {
        let caps = parse(
            r#"{"platform":"android","profile_version":1,"max_bitrate":8000000,"decoding":{
                "containers":["mp4","webm"],
                "video":[{"codec":"hevc","max_bit_depth":10},{"codec":"av1","max_bit_depth":10,"smooth":false}],
                "audio":[{"codec":"eac3","max_channels":8}],
                "hdr":["hdr10","hlg"],
                "max_width":3840,"max_height":2160,"max_frame_rate":30}}"#,
        );
        let decoding = caps.decoding.expect("a report was sent");
        assert_eq!(decoding.containers, Some(vec![Container::Mp4, Container::Mkv]));
        let video = decoding.video.expect("video was measured");
        assert!(video[0].smooth);
        assert!(!video[1].smooth);
        assert_eq!(decoding.hdr, Some(vec![HdrFormat::Hdr10, HdrFormat::Hlg]));
        assert_eq!(decoding.max_frame_rate, Some(30));
        assert_eq!(decoding.audio.map(|list| list[0].max_channels), Some(8));
        assert_eq!(caps.max_bitrate, Some(8_000_000));
    }

    #[test]
    fn a_name_this_server_does_not_know_is_dropped_rather_than_refused() {
        // A newer client naming a format this build has never heard of should
        // lose that one entry, not fail to start playing.
        let caps = parse(
            r#"{"platform":"android","profile_version":1,"decoding":{
                "containers":["mp4","ogg"],"hdr":["hdr10","hdr12"]}}"#,
        );
        let decoding = caps.decoding.expect("a report was sent");
        assert_eq!(decoding.containers, Some(vec![Container::Mp4]));
        assert_eq!(decoding.hdr, Some(vec![HdrFormat::Hdr10]));
        assert_eq!(decoding.video, None);
    }
}
