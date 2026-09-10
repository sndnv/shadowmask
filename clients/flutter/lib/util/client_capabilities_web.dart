import 'dart:js_interop';

import 'package:web/web.dart' as web;

import 'package:shadowmask/model/session/client_decoding.dart';

typedef _Probe = ({String codec, String contentType, int maxBitDepth});

const List<_Probe> _probes = <_Probe>[
  (
    codec: 'h264',
    contentType: 'video/mp4; codecs="avc1.640028"',
    maxBitDepth: 8,
  ),
  (
    codec: 'hevc',
    contentType: 'video/mp4; codecs="hvc1.2.4.L120.B0"',
    maxBitDepth: 10,
  ),
  (
    codec: 'vp9',
    contentType: 'video/webm; codecs="vp09.02.10.10"',
    maxBitDepth: 10,
  ),
  (
    codec: 'av1',
    contentType: 'video/mp4; codecs="av01.0.08M.10"',
    maxBitDepth: 10,
  ),
];

Future<ClientDecoding?> measureDecoding() async {
  final List<VideoCodecCap> video = <VideoCodecCap>[];
  for (final _Probe probe in _probes) {
    final VideoCodecCap? cap = await _measure(probe);
    if (cap != null) {
      video.add(cap);
    }
  }
  return video.isEmpty ? null : ClientDecoding(video: video);
}

Future<VideoCodecCap?> _measure(_Probe probe) async {
  try {
    final web.MediaCapabilitiesDecodingInfo info = await web
        .window
        .navigator
        .mediaCapabilities
        .decodingInfo(
          web.MediaDecodingConfiguration(
            type: 'file',
            video: web.VideoConfiguration(
              contentType: probe.contentType,
              width: 1920,
              height: 1080,
              bitrate: 8000000,
              framerate: 30,
            ),
          ),
        )
        .toDart;
    if (!info.supported) {
      return null;
    }
    return VideoCodecCap(
      codec: probe.codec,
      maxBitDepth: probe.maxBitDepth,
      smooth: info.smooth,
    );
  } catch (_) {
    return null;
  }
}
