import 'dart:js_interop';

import 'package:web/web.dart' as web;

import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/browser_report.dart';

const Verdict _refused = (
  supported: false,
  smooth: false,
  powerEfficient: false,
);

Future<ClientDecoding?> measureDecoding() async {
  final List<Answer<VideoQuestion>> video = <Answer<VideoQuestion>>[
    for (final VideoQuestion q in kVideoQuestions)
      (
        question: q,
        file: await _ask('file', video: _video(q)),
        stream: await _ask('media-source', video: _video(q)),
      ),
  ];
  final List<AudioAnswer> audio = await () async {
    final List<AudioAnswer> list = [];
    for (final AudioQuestion q in kAudioQuestions) {
      for (final int channels in kAudioChannels) {
        list.add((
          question: q,
          channels: channels,
          file: await _ask('file', audio: _audio(q, channels)),
          stream: await _ask('media-source', audio: _audio(q, channels)),
        ));
      }
    }
    return list;
  }();
  final List<Answer<HdrQuestion>> hdr = <Answer<HdrQuestion>>[
    for (final HdrQuestion q in kHdrQuestions)
      (
        question: q,
        file: await _ask('file', video: _hdr(q)),
        stream: await _ask('media-source', video: _hdr(q)),
      ),
  ];
  return browserReport(video: video, audio: audio, hdr: hdr);
}

web.VideoConfiguration _video(VideoQuestion q) => web.VideoConfiguration(
  contentType: q.contentType,
  width: kProbeSizes[q.height]!.width,
  height: q.height,
  bitrate: kProbeSizes[q.height]!.bitrate,
  framerate: kProbeFrameRate,
);

web.AudioConfiguration _audio(AudioQuestion q, int channels) =>
    web.AudioConfiguration(
      contentType: q.contentType,
      channels: '$channels',
      bitrate: 256000,
      samplerate: 48000,
    );

web.VideoConfiguration _hdr(HdrQuestion q) {
  final ({int width, int bitrate}) size = kProbeSizes[kUltraHd]!;
  final String? metadata = q.metadata;
  return metadata == null
      ? web.VideoConfiguration(
          contentType: q.contentType,
          width: size.width,
          height: kUltraHd,
          bitrate: size.bitrate,
          framerate: kProbeFrameRate,
          colorGamut: kHdrColorGamut,
          transferFunction: q.transfer,
        )
      : web.VideoConfiguration(
          contentType: q.contentType,
          width: size.width,
          height: kUltraHd,
          bitrate: size.bitrate,
          framerate: kProbeFrameRate,
          hdrMetadataType: metadata,
          colorGamut: kHdrColorGamut,
          transferFunction: q.transfer,
        );
}

Future<Verdict> _ask(
  String type, {
  web.VideoConfiguration? video,
  web.AudioConfiguration? audio,
}) async {
  try {
    final web.MediaDecodingConfiguration config = video != null
        ? web.MediaDecodingConfiguration(type: type, video: video)
        : web.MediaDecodingConfiguration(type: type, audio: audio!);
    final web.MediaCapabilitiesDecodingInfo info = await web
        .window
        .navigator
        .mediaCapabilities
        .decodingInfo(config)
        .toDart;
    return (
      supported: info.supported,
      smooth: info.smooth,
      powerEfficient: info.powerEfficient,
    );
  } catch (_) {
    return _refused;
  }
}
