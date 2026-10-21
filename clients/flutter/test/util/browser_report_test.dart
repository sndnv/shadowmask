import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/browser_report.dart';

const Verdict software = (supported: true, smooth: true, powerEfficient: false);
const Verdict hardware = (supported: true, smooth: true, powerEfficient: true);
const Verdict stutters = (
  supported: true,
  smooth: false,
  powerEfficient: false,
);
const Verdict refused = (
  supported: false,
  smooth: false,
  powerEfficient: false,
);

List<Answer<VideoQuestion>> video(
  Verdict Function(VideoQuestion q) file, [
  Verdict Function(VideoQuestion q)? stream,
]) => <Answer<VideoQuestion>>[
  for (final VideoQuestion q in kVideoQuestions)
    (question: q, file: file(q), stream: (stream ?? file)(q)),
];

List<AudioAnswer> audio(Verdict Function(AudioQuestion q, int channels) both) =>
    <AudioAnswer>[
      for (final AudioQuestion q in kAudioQuestions)
        for (final int channels in kAudioChannels)
          (
            question: q,
            channels: channels,
            file: both(q, channels),
            stream: both(q, channels),
          ),
    ];

List<Answer<HdrQuestion>> hdr(Verdict Function(HdrQuestion q) both) =>
    <Answer<HdrQuestion>>[
      for (final HdrQuestion q in kHdrQuestions)
        (question: q, file: both(q), stream: both(q)),
    ];

Verdict onlyCodecs(Set<String> codecs, VideoQuestion q, Verdict verdict) =>
    codecs.contains(q.codec) ? verdict : refused;

ClientDecoding? report({
  List<Answer<VideoQuestion>>? pictures,
  List<AudioAnswer>? sound,
  List<Answer<HdrQuestion>>? ranges,
}) => browserReport(
  video: pictures ?? video((VideoQuestion q) => refused),
  audio:
      sound ??
      audio(
        (AudioQuestion q, int channels) =>
            q.codec == 'aac' && channels == 2 ? software : refused,
      ),
  hdr: ranges ?? hdr((HdrQuestion q) => refused),
);

void main() {
  test(
    'a codec counts only when it plays as a file and through media source',
    () {
      final ClientDecoding? decoding = report(
        pictures: video(
          (VideoQuestion q) => onlyCodecs(<String>{'h264', 'vp9'}, q, software),
          (VideoQuestion q) => onlyCodecs(<String>{'vp9'}, q, software),
        ),
      );

      expect(decoding?.video.map((VideoCodecCap c) => c.codec), <String>[
        'vp9',
      ]);
    },
  );

  test('a codec takes the deepest picture that plays', () {
    final ClientDecoding? decoding = report(
      pictures: video(
        (VideoQuestion q) => q.codec == 'h264' || q.bitDepth == 8
            ? onlyCodecs(<String>{'h264', 'hevc'}, q, software)
            : refused,
      ),
    );

    expect(decoding?.video, const <VideoCodecCap>[
      VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: true),
      VideoCodecCap(codec: 'hevc', maxBitDepth: 8, smooth: true),
    ]);
  });

  test('software decoding counts up to full hd', () {
    final ClientDecoding? decoding = report(
      pictures: video(
        (VideoQuestion q) => onlyCodecs(<String>{'h264', 'av1'}, q, software),
      ),
    );

    expect(decoding?.maxHeight, kFullHd);
    expect(decoding?.maxWidth, isNull);
    expect(decoding?.video.every((VideoCodecCap c) => c.smooth), isTrue);
  });

  test(
    'ultra hd counts only with hardware decoding, and a codec below it is not smooth',
    () {
      final ClientDecoding? decoding = report(
        pictures: video(
          (VideoQuestion q) => q.codec == 'av1'
              ? hardware
              : onlyCodecs(<String>{'h264'}, q, software),
        ),
      );

      expect(decoding?.maxHeight, kUltraHd);
      expect(decoding?.video, const <VideoCodecCap>[
        VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: false),
        VideoCodecCap(codec: 'av1', maxBitDepth: 10, smooth: true),
      ]);
    },
  );

  test('a codec that plays but never smoothly is reported as not smooth', () {
    final ClientDecoding? decoding = report(
      pictures: video(
        (VideoQuestion q) => onlyCodecs(<String>{'vp9'}, q, stutters),
      ),
    );

    expect(decoding?.video, const <VideoCodecCap>[
      VideoCodecCap(codec: 'vp9', maxBitDepth: 10, smooth: false),
    ]);
    expect(decoding?.maxHeight, isNull);
  });

  test('sound takes the most channels that play', () {
    final ClientDecoding? decoding = report(
      sound: audio((AudioQuestion q, int channels) {
        if (q.codec == 'aac') {
          return software;
        }
        return q.codec == 'flac' && channels == 2 ? software : refused;
      }),
    );

    expect(decoding?.audio, const <AudioCodecCap>[
      AudioCodecCap(codec: 'aac', maxChannels: 8),
      AudioCodecCap(codec: 'flac', maxChannels: 2),
    ]);
  });

  test(
    'an hdr format counts when any codec plays it, and none is still sent',
    () {
      final ClientDecoding? some = report(
        ranges: hdr(
          (HdrQuestion q) => q.format == 'hlg' && q.contentType.contains('vp09')
              ? software
              : refused,
        ),
      );

      expect(some?.hdr, <String>['hlg']);
      expect(report()?.hdr, isEmpty);
      expect(report()?.toJson()['hdr'], isEmpty);
    },
  );

  test('nothing that plays reports nothing', () {
    expect(
      report(sound: audio((AudioQuestion q, int channels) => refused)),
      isNull,
    );
  });

  test('every candidate has a question or is named as unaskable', () {
    final Map<String, dynamic> candidates =
        jsonDecode(
              File(
                '../../assets/capabilities/candidates.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    final Set<String> asked = <String>{
      for (final VideoQuestion q in kVideoQuestions) q.codec,
    };

    expect(<String>{
      ...asked,
      ...kUnaskableVideo,
    }, (candidates['video'] as List<dynamic>).cast<String>().toSet());
    expect(
      kAudioQuestions.map((AudioQuestion q) => q.codec).toSet(),
      (candidates['audio'] as List<dynamic>).cast<String>().toSet(),
    );
    expect(
      kHdrQuestions.map((HdrQuestion q) => q.format).toSet(),
      (candidates['hdr'] as List<dynamic>).cast<String>().toSet(),
    );
    expect(
      kVideoQuestions.every(
        (VideoQuestion q) => kProbeSizes.containsKey(q.height),
      ),
      isTrue,
    );
  });

  test('every sound is asked about in a container a stream carries it in', () {
    for (final AudioQuestion q in kAudioQuestions) {
      expect(
        q.contentType.startsWith('audio/mp4;') || q.contentType == 'audio/mpeg',
        isTrue,
        reason: q.codec,
      );
    }
  });
}
