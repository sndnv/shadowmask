import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/client_capabilities_native.dart';

void main() {
  const Set<String> decoders = <String>{
    'h264',
    'hevc',
    'mpeg2video',
    'aac',
    'truehd',
    'dts',
    'mp3',
    'pcm_s16le',
  };

  test('the sound comes from the decoders the player has', () {
    final ClientDecoding? decoding = withPlayerAudio(null, decoders);

    expect(decoding?.video, isEmpty);
    expect(decoding?.audio, const <AudioCodecCap>[
      AudioCodecCap(codec: 'aac', maxChannels: 8),
      AudioCodecCap(codec: 'dts', maxChannels: 8),
      AudioCodecCap(codec: 'truehd', maxChannels: 8),
      AudioCodecCap(codec: 'mp3', maxChannels: 2),
    ]);
  });

  test('the platform keeps its own picture measurements beside the sound', () {
    const ClientDecoding measured = ClientDecoding(
      video: <VideoCodecCap>[
        VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: true),
      ],
      audio: <AudioCodecCap>[AudioCodecCap(codec: 'opus', maxChannels: 2)],
      hdr: <String>['hdr10'],
      maxWidth: 3840,
      maxHeight: 2160,
      maxFrameRate: 60,
    );

    final ClientDecoding? decoding = withPlayerAudio(measured, <String>{'ac3'});

    expect(decoding?.video, measured.video);
    expect(decoding?.audio, const <AudioCodecCap>[
      AudioCodecCap(codec: 'ac3', maxChannels: 6),
    ]);
    expect(decoding?.hdr, <String>['hdr10']);
    expect(decoding?.ceiling, '3840x2160@60');
  });

  test('nothing measured reports nothing', () {
    expect(withPlayerAudio(null, <String>{}), isNull);
    expect(withPlayerAudio(const ClientDecoding(), <String>{'h264'}), isNull);
  });

  test('the desktop picture comes from the decoders the player has', () {
    expect(desktopVideo(decoders), const <VideoCodecCap>[
      VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: true),
      VideoCodecCap(codec: 'hevc', maxBitDepth: 12, smooth: true),
    ]);
  });
}
