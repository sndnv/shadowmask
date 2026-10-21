import 'package:shadowmask/model/session/client_decoding.dart';

typedef VideoQuestion = ({
  String codec,
  int bitDepth,
  int height,
  String contentType,
});

typedef AudioQuestion = ({String codec, String contentType});

typedef HdrQuestion = ({
  String format,
  String contentType,
  String? metadata,
  String transfer,
});

typedef Verdict = ({bool supported, bool smooth, bool powerEfficient});

typedef Answer<Q> = ({Q question, Verdict file, Verdict stream});

typedef AudioAnswer = ({
  AudioQuestion question,
  int channels,
  Verdict file,
  Verdict stream,
});

const int kFullHd = 1080;
const int kUltraHd = 2160;

const Map<int, ({int width, int bitrate})> kProbeSizes =
    <int, ({int width, int bitrate})>{
      kFullHd: (width: 1920, bitrate: 8000000),
      kUltraHd: (width: 3840, bitrate: 25000000),
    };

const int kProbeFrameRate = 30;

const List<VideoQuestion> kVideoQuestions = <VideoQuestion>[
  (
    codec: 'h264',
    bitDepth: 10,
    height: kFullHd,
    contentType: 'video/mp4; codecs="avc1.6e0028"',
  ),
  (
    codec: 'h264',
    bitDepth: 8,
    height: kFullHd,
    contentType: 'video/mp4; codecs="avc1.640028"',
  ),
  (
    codec: 'h264',
    bitDepth: 8,
    height: kUltraHd,
    contentType: 'video/mp4; codecs="avc1.640033"',
  ),
  (
    codec: 'hevc',
    bitDepth: 10,
    height: kFullHd,
    contentType: 'video/mp4; codecs="hvc1.2.4.L120.B0"',
  ),
  (
    codec: 'hevc',
    bitDepth: 8,
    height: kFullHd,
    contentType: 'video/mp4; codecs="hvc1.1.6.L120.B0"',
  ),
  (
    codec: 'hevc',
    bitDepth: 10,
    height: kUltraHd,
    contentType: 'video/mp4; codecs="hvc1.2.4.L153.B0"',
  ),
  (
    codec: 'vp9',
    bitDepth: 10,
    height: kFullHd,
    contentType: 'video/webm; codecs="vp09.02.40.10"',
  ),
  (
    codec: 'vp9',
    bitDepth: 8,
    height: kFullHd,
    contentType: 'video/webm; codecs="vp09.00.40.08"',
  ),
  (
    codec: 'vp9',
    bitDepth: 10,
    height: kUltraHd,
    contentType: 'video/webm; codecs="vp09.02.51.10"',
  ),
  (
    codec: 'av1',
    bitDepth: 10,
    height: kFullHd,
    contentType: 'video/mp4; codecs="av01.0.08M.10"',
  ),
  (
    codec: 'av1',
    bitDepth: 8,
    height: kFullHd,
    contentType: 'video/mp4; codecs="av01.0.08M.08"',
  ),
  (
    codec: 'av1',
    bitDepth: 10,
    height: kUltraHd,
    contentType: 'video/mp4; codecs="av01.0.13M.10"',
  ),
  (
    codec: 'mpeg4',
    bitDepth: 8,
    height: kFullHd,
    contentType: 'video/mp4; codecs="mp4v.20.9"',
  ),
];

const Set<String> kUnaskableVideo = <String>{'msmpeg4v3'};

const List<AudioQuestion> kAudioQuestions = <AudioQuestion>[
  (codec: 'aac', contentType: 'audio/mp4; codecs="mp4a.40.2"'),
  (codec: 'mp3', contentType: 'audio/mpeg'),
  (codec: 'ac3', contentType: 'audio/mp4; codecs="ac-3"'),
  (codec: 'eac3', contentType: 'audio/mp4; codecs="ec-3"'),
  (codec: 'dts', contentType: 'audio/mp4; codecs="dtsc"'),
  (codec: 'truehd', contentType: 'audio/mp4; codecs="mlpa"'),
  (codec: 'flac', contentType: 'audio/mp4; codecs="flac"'),
  (codec: 'opus', contentType: 'audio/mp4; codecs="opus"'),
  (codec: 'vorbis', contentType: 'audio/mp4; codecs="vorbis"'),
  (codec: 'mp2', contentType: 'audio/mp4; codecs="mp4a.6B"'),
];

const List<int> kAudioChannels = <int>[2, 6, 8];

const List<String> kHdrContentTypes = <String>[
  'video/mp4; codecs="hvc1.2.4.L153.B0"',
  'video/webm; codecs="vp09.02.51.10"',
  'video/mp4; codecs="av01.0.13M.10"',
];

const String kHdrColorGamut = 'rec2020';

final List<HdrQuestion> kHdrQuestions = <HdrQuestion>[
  for (final String contentType in kHdrContentTypes) ...<HdrQuestion>[
    (
      format: 'hdr10',
      contentType: contentType,
      metadata: 'smpteSt2086',
      transfer: 'pq',
    ),
    (
      format: 'hdr10plus',
      contentType: contentType,
      metadata: 'smpteSt2094-40',
      transfer: 'pq',
    ),
    (format: 'hlg', contentType: contentType, metadata: null, transfer: 'hlg'),
  ],
];

bool _plays(Verdict file, Verdict stream) => file.supported && stream.supported;

bool _smooth(Verdict file, Verdict stream) => file.smooth && stream.smooth;

bool _hardware(Verdict file, Verdict stream) =>
    file.powerEfficient && stream.powerEfficient;

int _ceiling(Iterable<Answer<VideoQuestion>> answers) {
  int ceiling = 0;
  for (final Answer<VideoQuestion> a in answers) {
    if (!_plays(a.file, a.stream) || !_smooth(a.file, a.stream)) {
      continue;
    }
    final bool counts =
        a.question.height <= kFullHd || _hardware(a.file, a.stream);
    if (counts && a.question.height > ceiling) {
      ceiling = a.question.height;
    }
  }
  return ceiling;
}

List<VideoCodecCap> _video(List<Answer<VideoQuestion>> answers) {
  final List<String> codecs = <String>[];
  for (final Answer<VideoQuestion> a in answers) {
    if (_plays(a.file, a.stream) && !codecs.contains(a.question.codec)) {
      codecs.add(a.question.codec);
    }
  }
  final Map<String, int> ceilings = <String, int>{
    for (final String codec in codecs)
      codec: _ceiling(
        answers.where((Answer<VideoQuestion> a) => a.question.codec == codec),
      ),
  };
  final int top = ceilings.values.fold(0, (int a, int b) => a > b ? a : b);
  return <VideoCodecCap>[
    for (final String codec in codecs)
      VideoCodecCap(
        codec: codec,
        maxBitDepth: answers
            .where(
              (Answer<VideoQuestion> a) =>
                  a.question.codec == codec && _plays(a.file, a.stream),
            )
            .map((Answer<VideoQuestion> a) => a.question.bitDepth)
            .fold(0, (int a, int b) => a > b ? a : b),
        smooth: top > 0 && ceilings[codec] == top,
      ),
  ];
}

List<AudioCodecCap> _audio(List<AudioAnswer> answers) {
  final Map<String, int> channels = <String, int>{};
  for (final AudioAnswer a in answers) {
    if (!_plays(a.file, a.stream)) {
      continue;
    }
    final int known = channels[a.question.codec] ?? 0;
    channels[a.question.codec] = a.channels > known ? a.channels : known;
  }
  return <AudioCodecCap>[
    for (final MapEntry<String, int> entry in channels.entries)
      AudioCodecCap(codec: entry.key, maxChannels: entry.value),
  ];
}

List<String> _hdr(List<Answer<HdrQuestion>> answers) {
  final List<String> formats = <String>[];
  for (final Answer<HdrQuestion> a in answers) {
    if (_plays(a.file, a.stream) && !formats.contains(a.question.format)) {
      formats.add(a.question.format);
    }
  }
  return formats;
}

ClientDecoding? browserReport({
  required List<Answer<VideoQuestion>> video,
  required List<AudioAnswer> audio,
  required List<Answer<HdrQuestion>> hdr,
}) {
  final List<VideoCodecCap> codecs = _video(video);
  final int top = _ceiling(video);
  final ClientDecoding report = ClientDecoding(
    video: codecs,
    audio: _audio(audio),
    hdr: _hdr(hdr),
    maxHeight: top > 0 ? top : null,
  );
  return report.isEmpty ? null : report;
}
