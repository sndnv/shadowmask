import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/client_decoding.dart';

void main() {
  test('a report from the platform channel reads every field', () {
    final ClientDecoding decoding = ClientDecoding.fromChannel(
      <Object?, Object?>{
        'video': <Object?>[
          <Object?, Object?>{
            'codec': 'hevc',
            'max_bit_depth': 10,
            'smooth': true,
          },
          <Object?, Object?>{
            'codec': 'av1',
            'max_bit_depth': 10,
            'smooth': false,
          },
        ],
        'audio': <Object?>[
          <Object?, Object?>{'codec': 'eac3', 'max_channels': 8},
        ],
        'hdr': <Object?>['hdr10', 'hlg'],
        'max_width': 3840,
        'max_height': 2160,
        'max_frame_rate': 30,
      },
    );

    expect(decoding.video.length, 2);
    expect(decoding.video.first.codec, 'hevc');
    expect(decoding.video.last.smooth, isFalse);
    expect(decoding.audio.single.maxChannels, 8);
    expect(decoding.hdr, <String>['hdr10', 'hlg']);
    expect(decoding.ceiling, '3840x2160@30');
    expect(decoding.isEmpty, isFalse);
  });

  test('a codec that says nothing about smoothness counts as smooth', () {
    final ClientDecoding decoding = ClientDecoding.fromChannel(
      <Object?, Object?>{
        'video': <Object?>[
          <Object?, Object?>{'codec': 'h264', 'max_bit_depth': 8},
        ],
      },
    );

    expect(decoding.video.single.smooth, isTrue);
    expect(decoding.ceiling, isNull);
  });

  test('an empty report is recognised rather than sent', () {
    expect(
      ClientDecoding.fromChannel(const <Object?, Object?>{}).isEmpty,
      isTrue,
    );
  });

  test('the body carries only what was measured', () {
    const ClientDecoding decoding = ClientDecoding(
      video: <VideoCodecCap>[
        VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: false),
      ],
      maxHeight: 2160,
    );

    expect(decoding.toJson(), <String, dynamic>{
      'video': <dynamic>[
        <String, dynamic>{
          'codec': 'hevc',
          'max_bit_depth': 10,
          'smooth': false,
        },
      ],
      'max_height': 2160,
    });
  });

  test('a size without a frame rate still reads as a ceiling', () {
    const ClientDecoding decoding = ClientDecoding(
      maxWidth: 1920,
      maxHeight: 1080,
    );

    expect(decoding.ceiling, '1920x1080');
  });
}
