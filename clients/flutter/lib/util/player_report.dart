import 'package:shadowmask/model/session/client_decoding.dart';

const Map<String, int> kPlayerVideo = <String, int>{
  'h264': 10,
  'hevc': 12,
  'vp9': 12,
  'av1': 12,
  'mpeg4': 8,
  'msmpeg4v3': 8,
};

const Set<String> kSmoothInSoftware = <String>{'mpeg4', 'msmpeg4v3'};

const Map<String, int> kPlayerAudio = <String, int>{
  'aac': 8,
  'ac3': 6,
  'eac3': 8,
  'dts': 8,
  'truehd': 8,
  'flac': 8,
  'opus': 8,
  'vorbis': 8,
  'mp3': 2,
  'mp2': 2,
};

const List<String> kPlayerOwnContainers = <String>['mkv'];

const Map<String, String> kPlayerDemuxers = <String, String>{
  'mp4': 'mov',
  'ts': 'mpegts',
  'avi': 'avi',
  'mov': 'mov',
  'm4v': 'mov',
  'mpg': 'mpeg',
  'm2ts': 'mpegts',
  'wmv': 'asf',
  'flv': 'flv',
};

Set<String> demuxerNames(String raw) => <String>{
  for (final String name in raw.split(','))
    if (name.trim().isNotEmpty) name.trim(),
};

ClientDecoding? playerReport({
  required Set<String> decoders,
  required Set<String> demuxers,
  bool phone = false,
  ClientDecoding? hardware,
}) {
  if (decoders.isEmpty) {
    return hardware == null || hardware.isEmpty ? null : hardware;
  }
  final ClientDecoding report = ClientDecoding(
    containers: _containers(demuxers),
    video: phone && (hardware?.video.isEmpty ?? true)
        ? const <VideoCodecCap>[]
        : <VideoCodecCap>[
            for (final MapEntry<String, int> codec in kPlayerVideo.entries)
              if (decoders.contains(codec.key))
                _named(hardware, codec.key) ??
                    VideoCodecCap(
                      codec: codec.key,
                      maxBitDepth: codec.value,
                      smooth: !phone || kSmoothInSoftware.contains(codec.key),
                    ),
          ],
    audio: <AudioCodecCap>[
      for (final MapEntry<String, int> codec in kPlayerAudio.entries)
        if (decoders.contains(codec.key))
          AudioCodecCap(codec: codec.key, maxChannels: codec.value),
    ],
    hdr: hardware?.hdr,
    maxWidth: hardware?.maxWidth,
    maxHeight: hardware?.maxHeight,
    maxFrameRate: hardware?.maxFrameRate,
  );
  return report.isEmpty ? null : report;
}

List<String>? _containers(Set<String> demuxers) {
  if (demuxers.isEmpty) {
    return null;
  }
  return <String>[
    ...kPlayerOwnContainers,
    for (final MapEntry<String, String> container in kPlayerDemuxers.entries)
      if (demuxers.contains(container.value)) container.key,
  ];
}

VideoCodecCap? _named(ClientDecoding? hardware, String codec) {
  for (final VideoCodecCap cap in hardware?.video ?? const <VideoCodecCap>[]) {
    if (cap.codec == codec) {
      return cap;
    }
  }
  return null;
}
