import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';

ClientDecoding _measured() => const ClientDecoding(
  video: <VideoCodecCap>[
    VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: true),
    VideoCodecCap(codec: 'vp9', maxBitDepth: 12, smooth: true),
  ],
  audio: <AudioCodecCap>[AudioCodecCap(codec: 'aac', maxChannels: 8)],
  hdr: <String>['hdr10'],
);

VideoCodecCap? _codec(ClientDecoding? decoding, String codec) {
  for (final VideoCodecCap cap in decoding?.video ?? const <VideoCodecCap>[]) {
    if (cap.codec == codec) {
      return cap;
    }
  }
  return null;
}

void main() {
  test('no overrides leave the measured report untouched', () {
    final ClientDecoding measured = _measured();
    expect(const CapabilityOverrides().applyTo(measured), same(measured));
  });

  test('marking a codec software keeps it listed but not smooth', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{'vp9': CodecSupport.software},
    );
    final ClientDecoding? applied = overrides.applyTo(_measured());
    expect(_codec(applied, 'vp9')?.smooth, isFalse);
    expect(_codec(applied, 'vp9')?.maxBitDepth, 12);
    expect(_codec(applied, 'h264')?.smooth, isTrue);
  });

  test('marking a codec unusable drops it from the report', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{'vp9': CodecSupport.unsupported},
    );
    expect(_codec(overrides.applyTo(_measured()), 'vp9'), isNull);
  });

  test('a codec the device never reported can be added', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{'av1': CodecSupport.hardware},
    );
    final VideoCodecCap? added = _codec(overrides.applyTo(_measured()), 'av1');
    expect(added?.smooth, isTrue);
    expect(added?.maxBitDepth, kAddedCodecBitDepth);
  });

  // An empty video list fails the server's own validation, which drops the
  // whole report and silently restores the profile the user was editing.
  test('dropping every codec falls back to what was measured', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{
        'h264': CodecSupport.unsupported,
        'vp9': CodecSupport.unsupported,
      },
    );
    expect(overrides.applyTo(_measured())?.video.length, 2);
  });

  test('denying high dynamic range sends an explicit empty list', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      hdr: HdrChoice.deny,
    );
    final ClientDecoding? applied = overrides.applyTo(_measured());
    expect(applied?.hdr, isEmpty);
    expect(applied?.toJson()['hdr'], isEmpty);
  });

  test('allowing high dynamic range on a device that reported none', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      hdr: HdrChoice.allow,
    );
    final ClientDecoding? applied = overrides.applyTo(
      const ClientDecoding(
        video: <VideoCodecCap>[
          VideoCodecCap(codec: 'h264', maxBitDepth: 8, smooth: true),
        ],
      ),
    );
    expect(applied?.hdr, kHdrFormats);
  });

  test('a ceiling can be set on a device that measured none', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      maxHeight: 1080,
      maxFrameRate: 30,
    );
    final ClientDecoding? applied = overrides.applyTo(null);
    expect(applied?.maxHeight, 1080);
    expect(applied?.maxFrameRate, 30);
    expect(applied?.toJson().containsKey('video'), isFalse);
  });

  test('overrides survive a round trip through storage', () {
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{
        'vp9': CodecSupport.software,
        'h264': CodecSupport.auto,
      },
      maxHeight: 1080,
      maxFrameRate: 60,
      hdr: HdrChoice.deny,
    );
    expect(CapabilityOverrides.fromJson(overrides.toJson()), overrides);
  });

  test('an unreadable stored value reads as no overrides', () {
    expect(
      CapabilityOverrides.fromJson(<String, dynamic>{
        'codecs': 'nonsense',
        'hdr': 'nonsense',
      }),
      const CapabilityOverrides(),
    );
  });
}
