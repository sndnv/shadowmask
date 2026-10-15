import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/subtitle_size.dart';

void main() {
  test('a size is a share of the height the video has', () {
    expect(subtitleFontSize(400, SubtitleSize.medium), closeTo(18, 0.001));
    expect(subtitleFontSize(1080, SubtitleSize.medium), closeTo(48.6, 0.001));
    expect(subtitleFontSize(400, SubtitleSize.small), closeTo(14.4, 0.001));
    expect(subtitleFontSize(400, SubtitleSize.large), closeTo(22.5, 0.001));
  });

  test('default draws as medium where the device has no setting', () {
    expect(
      subtitleFontSize(720, SubtitleSize.deviceDefault),
      subtitleFontSize(720, SubtitleSize.medium),
    );
  });

  test('a browser keeps its own size for default only', () {
    expect(cueFontSize(SubtitleSize.deviceDefault), isNull);
    expect(cueFontSize(SubtitleSize.small), '80%');
    expect(cueFontSize(SubtitleSize.medium), '100%');
    expect(cueFontSize(SubtitleSize.large), '125%');
  });

  test('a stored value reads back, and anything else is default', () {
    for (final SubtitleSize size in SubtitleSize.values) {
      expect(SubtitleSize.fromWire(size.wire), size);
    }
    expect(SubtitleSize.fromWire('huge'), SubtitleSize.deviceDefault);
    expect(SubtitleSize.fromWire(null), SubtitleSize.deviceDefault);
  });
}
